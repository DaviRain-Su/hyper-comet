//! ProofShip Studio gate runner.
//!
//! The runner mirrors `proofship/bridge/server.mjs`: validate a single
//! ProgramV1 source, stage it into the studio inbox, run
//! `proof-forge-next check → build --target evm → inspect` with the vendored
//! toolchain environment, and stream stage boundaries plus capped combined
//! output. Paths are injectable for tests because CLI/repo discovery is
//! environment-heavy by design.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use comet_proto::{
    StudioGateArtifact, StudioGateDigest, StudioGateEvent, StudioGateStage, StudioStatusResponse,
};
use futures::StreamExt;
use futures::stream::BoxStream;
use tokio::io::AsyncReadExt;
use tokio::process::Command;
use tokio::sync::mpsc;

const MAX_SOURCE: usize = 64 * 1024;
const OUTPUT_CAP: usize = 1024 * 1024;
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(240);

#[derive(Debug, thiserror::Error)]
pub enum GateError {
    #[error("bad module name")]
    BadModuleName,
    #[error("source contract violated")]
    SourceContractViolated,
    #[error("source too large")]
    SourceTooLarge,
    #[error("repo root not found")]
    RepoRootNotFound,
    #[error(
        "product CLI missing. Resolve it by one of:\n  1) PF_CLI=/absolute/path/to/proof-forge-next\n  2) put proof-forge-next on PATH\n  3) PROOF_FORGE_ROOT=/path/to/proof_forge (uses .lake/build/bin/proof-forge-next)\n  4) install vendored toolchain: proofship/scripts/install-toolchain.sh [dist.tar.gz]"
    )]
    CliMissing,
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("stage {stage:?} timed out after {timeout_secs}s")]
    Timeout {
        stage: StudioGateStage,
        timeout_secs: u64,
    },
}

#[derive(Debug, Clone, Default)]
pub struct StudioPaths {
    pub repo_root: Option<PathBuf>,
    pub pf_cli: Option<PathBuf>,
    pub proof_forge_root: Option<PathBuf>,
    pub cwd: Option<PathBuf>,
    pub exe_dir: Option<PathBuf>,
    /// Staging project root for gate runs (draft sources + out dirs). When
    /// unset, falls back to `<repo_root>/proofship/inbox`.
    pub inbox_root: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct GateConfig {
    pub paths: StudioPaths,
    pub timeout: Duration,
}

impl Default for GateConfig {
    fn default() -> Self {
        Self {
            paths: StudioPaths::default(),
            timeout: DEFAULT_TIMEOUT,
        }
    }
}

#[derive(Debug, Clone)]
pub struct StudioGate {
    config: GateConfig,
}

#[derive(Debug, Clone)]
struct ResolvedGate {
    repo_root: PathBuf,
    project_root: PathBuf,
    cli: PathBuf,
    elan_toolchain: Option<String>,
    tool_root: Option<PathBuf>,
}

#[derive(Debug)]
struct StageOutput {
    code: i32,
    output: String,
}

impl StudioGate {
    pub fn new(config: GateConfig) -> Self {
        Self { config }
    }

    pub fn default_detect() -> Self {
        Self::new(GateConfig::default())
    }

    pub fn validate(module: &str, source: &str) -> Result<(), GateError> {
        if !valid_module(module) {
            return Err(GateError::BadModuleName);
        }
        if source.len() > MAX_SOURCE {
            return Err(GateError::SourceTooLarge);
        }
        if !source.starts_with("import ProofForgeV2") {
            return Err(GateError::SourceContractViolated);
        }
        Ok(())
    }

    pub fn status(&self) -> StudioStatusResponse {
        match self.resolve() {
            Ok(resolved) => StudioStatusResponse {
                repo_root: Some(resolved.repo_root.to_string_lossy().to_string()),
                pf_cli: Some(resolved.cli.to_string_lossy().to_string()),
                cli_resolved: true,
                elan_toolchain: resolved.elan_toolchain,
                proof_forge_tool_root: resolved.tool_root.map(|p| p.to_string_lossy().to_string()),
                toolchain_ok: true,
                error: None,
            },
            Err(err) => StudioStatusResponse {
                repo_root: self
                    .repo_root()
                    .ok()
                    .map(|p| p.to_string_lossy().to_string()),
                pf_cli: None,
                cli_resolved: false,
                elan_toolchain: std::env::var("ELAN_TOOLCHAIN").ok(),
                proof_forge_tool_root: std::env::var("PROOF_FORGE_TOOL_ROOT").ok(),
                toolchain_ok: false,
                error: Some(err.to_string()),
            },
        }
    }

    pub fn run_gate(&self, module: String, source: String) -> BoxStream<'static, StudioGateEvent> {
        let this = self.clone();
        let (tx, rx) = mpsc::channel(16);
        tokio::spawn(async move {
            if let Err(err) = this.run_gate_inner(module, source, tx.clone()).await {
                let _ = tx
                    .send(StudioGateEvent::StageDone {
                        stage: StudioGateStage::Check,
                        ok: false,
                        output: err.to_string(),
                    })
                    .await;
                let _ = tx
                    .send(StudioGateEvent::Done {
                        ok: false,
                        stage: StudioGateStage::Check,
                        artifacts: Vec::new(),
                        digest: StudioGateDigest::default(),
                    })
                    .await;
            }
        });
        futures::stream::unfold(rx, |mut rx| async move {
            rx.recv().await.map(|item| (item, rx))
        })
        .boxed()
    }

    async fn run_gate_inner(
        &self,
        module: String,
        source: String,
        tx: mpsc::Sender<StudioGateEvent>,
    ) -> Result<(), GateError> {
        Self::validate(&module, &source)?;
        let resolved = self.resolve()?;
        let inbox = resolved.project_root.join("studio-inbox");
        tokio::fs::create_dir_all(&inbox).await?;
        let rel_source = format!("studio-inbox/{module}.lean");
        tokio::fs::write(resolved.project_root.join(&rel_source), source).await?;

        let check = self
            .run_stage(
                &resolved,
                StudioGateStage::Check,
                &["check", &rel_source, "--module", &module],
                &tx,
            )
            .await?;
        if check.code != 0 {
            done_fail(&tx, StudioGateStage::Check).await;
            return Ok(());
        }

        let out_rel = format!("studio-inbox/out-{}", module.to_lowercase());
        let out_dir = resolved.project_root.join(&out_rel);
        let _ = tokio::fs::remove_dir_all(&out_dir).await;
        let build = self
            .run_stage(
                &resolved,
                StudioGateStage::Build,
                &[
                    "build",
                    &rel_source,
                    "--module",
                    &module,
                    "--target",
                    "evm",
                    "-o",
                    &out_rel,
                ],
                &tx,
            )
            .await?;
        if build.code != 0 {
            done_fail(&tx, StudioGateStage::Build).await;
            return Ok(());
        }

        let inspect = self
            .run_stage(
                &resolved,
                StudioGateStage::Inspect,
                &["inspect", "--output-dir", &out_rel],
                &tx,
            )
            .await?;
        if inspect.code != 0 {
            done_fail(&tx, StudioGateStage::Inspect).await;
            return Ok(());
        }

        let artifacts = list_artifacts(&out_dir).await?;
        let digest = parse_digest(&inspect.output);
        let _ = tx
            .send(StudioGateEvent::Done {
                ok: true,
                stage: StudioGateStage::Done,
                artifacts,
                digest,
            })
            .await;
        Ok(())
    }

    async fn run_stage(
        &self,
        resolved: &ResolvedGate,
        stage: StudioGateStage,
        args: &[&str],
        tx: &mpsc::Sender<StudioGateEvent>,
    ) -> Result<StageOutput, GateError> {
        let _ = tx.send(StudioGateEvent::Started { stage }).await;
        let mut command = Command::new(&resolved.cli);
        command
            .args(args)
            .current_dir(&resolved.project_root)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if std::env::var_os("ELAN_TOOLCHAIN").is_none()
            && let Some(pin) = &resolved.elan_toolchain
        {
            command.env("ELAN_TOOLCHAIN", pin);
        }
        if std::env::var_os("PROOF_FORGE_TOOL_ROOT").is_none()
            && let Some(root) = &resolved.tool_root
        {
            command.env("PROOF_FORGE_TOOL_ROOT", root);
        }
        let mut child = command.spawn()?;
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        let out_task = tokio::spawn(read_capped(stdout));
        let err_task = tokio::spawn(read_capped(stderr));
        let status = match tokio::time::timeout(self.config.timeout, child.wait()).await {
            Ok(status) => status?,
            Err(_) => {
                let _ = child.kill().await;
                return Err(GateError::Timeout {
                    stage,
                    timeout_secs: self.config.timeout.as_secs(),
                });
            }
        };
        let mut output = String::new();
        if let Ok(Ok(stdout)) = out_task.await {
            output.push_str(&stdout);
        }
        if let Ok(Ok(stderr)) = err_task.await {
            output.push_str(&stderr);
        }
        output = tail_string(&output, OUTPUT_CAP);
        let ok = status.success();
        let code = status.code().unwrap_or(if ok { 0 } else { 1 });
        let _ = tx
            .send(StudioGateEvent::StageDone {
                stage,
                ok,
                output: output.trim().to_string(),
            })
            .await;
        Ok(StageOutput { code, output })
    }

    fn resolve(&self) -> Result<ResolvedGate, GateError> {
        let repo_root = self.repo_root()?;
        let cli = self.resolve_cli(&repo_root)?;
        let project_root = self
            .config
            .paths
            .inbox_root
            .clone()
            .unwrap_or_else(|| repo_root.join("proofship/inbox"));
        let elan_toolchain = std::env::var("ELAN_TOOLCHAIN")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .or_else(|| read_pin(repo_root.join("proofship/toolchain/lean-toolchain")))
            .or_else(|| {
                self.proof_forge_root()
                    .and_then(|root| read_pin(root.join("lean-toolchain")))
            });
        let tool_root = if std::env::var_os("PROOF_FORGE_TOOL_ROOT").is_some() {
            std::env::var_os("PROOF_FORGE_TOOL_ROOT").map(PathBuf::from)
        } else {
            let candidate = repo_root
                .join("proofship/toolchain/tool-root")
                .join(platform_name());
            candidate.is_dir().then_some(candidate)
        };
        Ok(ResolvedGate {
            repo_root,
            project_root,
            cli,
            elan_toolchain,
            tool_root,
        })
    }

    fn repo_root(&self) -> Result<PathBuf, GateError> {
        if let Some(root) = &self.config.paths.repo_root {
            return Ok(root.clone());
        }
        if let Some(root) = std::env::var_os("PROOFSHIP_REPO_ROOT").filter(|s| !s.is_empty()) {
            return Ok(PathBuf::from(root));
        }
        let mut starts = Vec::new();
        if let Some(exe_dir) = &self.config.paths.exe_dir {
            starts.push(exe_dir.clone());
        } else if let Ok(exe) = std::env::current_exe()
            && let Some(parent) = exe.parent()
        {
            starts.push(parent.to_path_buf());
        }
        if let Some(cwd) = &self.config.paths.cwd {
            starts.push(cwd.clone());
        } else if let Ok(cwd) = std::env::current_dir() {
            starts.push(cwd);
        }
        for start in starts {
            for dir in start.ancestors() {
                if dir.join("proofship/scripts/gate.sh").is_file() {
                    return Ok(dir.to_path_buf());
                }
            }
        }
        Err(GateError::RepoRootNotFound)
    }

    fn resolve_cli(&self, repo_root: &Path) -> Result<PathBuf, GateError> {
        if let Some(cli) = &self.config.paths.pf_cli {
            return is_executable(cli)
                .then(|| cli.clone())
                .ok_or(GateError::CliMissing);
        }
        if let Some(cli) = std::env::var_os("PF_CLI").filter(|s| !s.is_empty()) {
            let cli = PathBuf::from(cli);
            return is_executable(&cli)
                .then_some(cli)
                .ok_or(GateError::CliMissing);
        }
        if let Some(cli) = find_on_path("proof-forge-next") {
            return Ok(cli);
        }
        if let Some(root) = self.proof_forge_root() {
            let cli = root.join(".lake/build/bin/proof-forge-next");
            if is_executable(&cli) {
                return Ok(cli);
            }
        }
        let vendored = repo_root.join("proofship/toolchain/bin/proof-forge-next");
        is_executable(&vendored)
            .then_some(vendored)
            .ok_or(GateError::CliMissing)
    }

    fn proof_forge_root(&self) -> Option<PathBuf> {
        self.config.paths.proof_forge_root.clone().or_else(|| {
            std::env::var_os("PROOF_FORGE_ROOT")
                .filter(|s| !s.is_empty())
                .map(PathBuf::from)
        })
    }
}

async fn done_fail(tx: &mpsc::Sender<StudioGateEvent>, stage: StudioGateStage) {
    let _ = tx
        .send(StudioGateEvent::Done {
            ok: false,
            stage,
            artifacts: Vec::new(),
            digest: StudioGateDigest::default(),
        })
        .await;
}

async fn read_capped<R>(reader: Option<R>) -> std::io::Result<String>
where
    R: tokio::io::AsyncRead + Unpin,
{
    let Some(mut reader) = reader else {
        return Ok(String::new());
    };
    let mut buf = Vec::new();
    reader.read_to_end(&mut buf).await?;
    Ok(tail_bytes(&buf, OUTPUT_CAP))
}

fn tail_bytes(bytes: &[u8], cap: usize) -> String {
    if bytes.len() <= cap {
        String::from_utf8_lossy(bytes).into_owned()
    } else {
        let start = bytes.len() - cap;
        format!(
            "[output truncated; kept last {cap} bytes]\n{}",
            String::from_utf8_lossy(&bytes[start..])
        )
    }
}

fn tail_string(text: &str, cap: usize) -> String {
    if text.len() <= cap {
        text.to_string()
    } else {
        let start = text.len() - cap;
        format!(
            "[output truncated; kept last {cap} bytes]\n{}",
            &text[start..]
        )
    }
}

async fn list_artifacts(out_dir: &Path) -> Result<Vec<StudioGateArtifact>, GateError> {
    let mut entries = tokio::fs::read_dir(out_dir).await?;
    let mut artifacts = Vec::new();
    while let Some(entry) = entries.next_entry().await? {
        let meta = entry.metadata().await?;
        if meta.is_file() {
            artifacts.push(StudioGateArtifact {
                name: entry.file_name().to_string_lossy().to_string(),
                size: meta.len(),
            });
        }
    }
    artifacts.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(artifacts)
}

fn parse_digest(raw: &str) -> StudioGateDigest {
    let output_set_digest = raw.lines().find_map(|line| {
        let idx = line.find("outputSetDigest")?;
        line[idx..]
            .split(|c: char| !(c.is_ascii_alphanumeric()))
            .find(|part| part.len() == 64 && part.chars().all(|c| c.is_ascii_hexdigit()))
            .map(str::to_string)
    });
    StudioGateDigest {
        output_set_digest,
        raw: raw.trim().to_string(),
    }
}

fn valid_module(module: &str) -> bool {
    let mut chars = module.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    first.is_ascii_alphabetic()
        && module.len() <= 64
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn read_pin(path: PathBuf) -> Option<String> {
    std::fs::read_to_string(path)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn find_on_path(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join(name))
        .find(|candidate| is_executable(candidate))
}

fn is_executable(path: &Path) -> bool {
    let Ok(meta) = std::fs::metadata(path) else {
        return false;
    };
    if !meta.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        meta.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
}

fn platform_name() -> String {
    let os = match std::env::consts::OS {
        "macos" => "darwin",
        other => other,
    };
    let arch = match std::env::consts::ARCH {
        "x86_64" => "x86_64",
        "aarch64" => "arm64",
        other => other,
    };
    format!("{os}-{arch}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;
    use std::io::Write;

    #[test]
    fn validation_rejects_bad_inputs() {
        assert!(matches!(
            StudioGate::validate("bad-module", "import ProofForgeV2\n"),
            Err(GateError::BadModuleName)
        ));
        assert!(matches!(
            StudioGate::validate("1Bad", "import ProofForgeV2\n"),
            Err(GateError::BadModuleName)
        ));
        assert!(matches!(
            StudioGate::validate("Good", "import Other\n"),
            Err(GateError::SourceContractViolated)
        ));
        let oversized = format!("import ProofForgeV2\n{}", "x".repeat(MAX_SOURCE));
        assert!(matches!(
            StudioGate::validate("Good", &oversized),
            Err(GateError::SourceTooLarge)
        ));
    }

    #[test]
    fn resolves_paths_and_cli_from_fake_layout() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join("proofship/scripts")).unwrap();
        std::fs::write(root.join("proofship/scripts/gate.sh"), "").unwrap();
        std::fs::create_dir_all(root.join("proofship/toolchain/bin")).unwrap();
        let cli = root.join("proofship/toolchain/bin/proof-forge-next");
        std::fs::File::create(&cli)
            .unwrap()
            .write_all(b"#!/bin/sh\n")
            .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&cli).unwrap().permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&cli, perms).unwrap();
        }
        std::fs::write(
            root.join("proofship/toolchain/lean-toolchain"),
            "lean-pin\n",
        )
        .unwrap();
        let platform = platform_name();
        std::fs::create_dir_all(root.join("proofship/toolchain/tool-root").join(platform)).unwrap();

        let gate = StudioGate::new(GateConfig {
            paths: StudioPaths {
                repo_root: Some(root.to_path_buf()),
                pf_cli: Some(cli.clone()),
                cwd: Some(root.join("proofship")),
                ..StudioPaths::default()
            },
            timeout: Duration::from_secs(1),
        });
        let resolved = gate.resolve().unwrap();
        assert_eq!(resolved.repo_root, root);
        assert_eq!(resolved.cli, cli);
        assert_eq!(resolved.elan_toolchain.as_deref(), Some("lean-pin"));
        assert!(resolved.tool_root.unwrap().is_dir());
        assert_eq!(resolved.project_root, root.join("proofship/inbox"));
    }

    #[tokio::test]
    #[ignore = "requires the vendored proofship toolchain"]
    async fn studio_gate_real_toolchain_passes() {
        let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|p| p.parent())
            .unwrap();
        let source = tokio::fs::read_to_string(
            repo.join("crates/engine/tests/fixtures/rwa_share_registry.lean"),
        )
        .await
        .unwrap();
        let gate = StudioGate::default_detect();
        let mut stream = gate.run_gate("RwaShareRegistry".into(), source);
        let mut done = None;
        while let Some(event) = stream.next().await {
            if let StudioGateEvent::Done { .. } = &event {
                done = Some(event);
            }
        }
        let Some(StudioGateEvent::Done {
            ok,
            stage,
            artifacts,
            digest,
        }) = done
        else {
            panic!("missing done event");
        };
        assert!(ok);
        assert_eq!(stage, StudioGateStage::Done);
        eprintln!("artifacts={artifacts:?}");
        eprintln!("outputSetDigest={:?}", digest.output_set_digest);
        assert!(artifacts.iter().any(|a| a.name.ends_with(".abi.json")));
        assert!(artifacts.iter().any(|a| a.name.ends_with(".yul")));
        assert!(artifacts.iter().any(|a| a.name.ends_with(".bin")));
        assert!(digest.output_set_digest.is_some());
    }
}
