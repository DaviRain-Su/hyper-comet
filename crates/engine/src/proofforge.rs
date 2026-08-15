//! Native ProofForge wiring for Sessions.
//!
//! Every dispatched run gets the ProgramV1 drafting skill prepended to the
//! prompt and the ProofForge MCP gate (`pf_check` / `pf_build` /
//! `pf_artifacts` / `pf_doctor`) attached to the ACP `session/new` — but
//! only when a ProofForge toolchain is actually present on the host. No
//! toolchain → runs are untouched.
//!
//! The gate server is `proofship-pf-mcp`, our own rmcp-based binary
//! maintained in this repo (crates/pf-mcp) and shipped next to the comet
//! executable. The old python MCP-V0 script from the proof_forge checkout
//! is kept only as a last-resort fallback.
//!
//! Detection (all env-driven, no repo layout assumptions):
//! - CLI: `PF_CLI` / `PROOF_FORGE_CLI`, `proof-forge-next` on PATH, or
//!   `$PROOF_FORGE_ROOT/.lake/build/bin/proof-forge-next`
//! - MCP gate: `PROOFSHIP_PF_MCP` override (binary, or `.py` run via
//!   python3), bundled `proofship-pf-mcp` next to the current executable,
//!   `proofship-pf-mcp` on PATH, or the legacy
//!   `$PROOF_FORGE_ROOT/tools/mcp/proof_forge_mcp_server.py`
//! - MCP over HTTP: `PROOFSHIP_PF_MCP_URL`
//!
//! Opt-outs: `PROOFSHIP_DISABLE_PF_MCP=1`, `PROOFSHIP_DISABLE_PF_SKILL=1`.

use std::path::PathBuf;

use zeron_proto::{McpEnvVar, McpServerConfig, RunRequest};

/// Marker so resume / retry / steer does not double-inject the skill body.
pub const SKILL_PROMPT_MARKER: &str = "<!-- proofship:proofforge-program-v1 -->";

/// The ProgramV1 drafting skill ships inside the engine binary — no
/// filesystem discovery, present on every install. Canonical source:
/// skills/proofforge-program-v1/SKILL.md (maintained in this repo).
const PROGRAM_V1_SKILL: &str = include_str!("../../../skills/proofforge-program-v1/SKILL.md");

const PF_MCP_BIN: &str = "proofship-pf-mcp";
const PF_MCP_SCRIPT_REL: &str = "tools/mcp/proof_forge_mcp_server.py";
const PF_CLI_BIN_REL: &str = ".lake/build/bin/proof-forge-next";

/// How to spawn the stdio MCP gate server.
#[derive(Debug, Clone, PartialEq)]
pub enum GateLauncher {
    /// Our rmcp binary (crates/pf-mcp), spawned directly.
    Binary(PathBuf),
    /// Legacy python MCP-V0 script from a proof_forge checkout.
    Script { python: PathBuf, script: PathBuf },
}

/// A ProofForge install found on this host.
#[derive(Debug, Clone, Default)]
pub struct Toolchain {
    pub cli: Option<PathBuf>,
    pub root: Option<PathBuf>,
    pub gate: Option<GateLauncher>,
    pub http_url: Option<String>,
}

impl Toolchain {
    /// The bundled gate binary ships with every install, so its presence
    /// alone says nothing about ProofForge being usable. The lane activates
    /// only on real signals: a product CLI, an HTTP gate URL, or an explicit
    /// `PROOFSHIP_PF_MCP` override.
    pub fn detect() -> Option<Self> {
        let root = env_path("PROOF_FORGE_ROOT").filter(|p| p.is_dir());
        let cli = env_path("PF_CLI")
            .or_else(|| env_path("PROOF_FORGE_CLI"))
            .filter(|p| p.is_file())
            .or_else(|| find_on_path("proof-forge-next"))
            .or_else(|| {
                root.as_ref()
                    .map(|r| r.join(PF_CLI_BIN_REL))
                    .filter(|p| p.is_file())
            });
        let explicit_gate = env_path("PROOFSHIP_PF_MCP")
            .filter(|p| p.is_file())
            .and_then(launcher_for);
        let http_url = std::env::var("PROOFSHIP_PF_MCP_URL")
            .ok()
            .map(|u| u.trim().to_string())
            .filter(|u| !u.is_empty());

        if cli.is_none() && explicit_gate.is_none() && http_url.is_none() {
            return None;
        }

        let gate = explicit_gate
            .or_else(|| bundled_gate().map(GateLauncher::Binary))
            .or_else(|| find_on_path(PF_MCP_BIN).map(GateLauncher::Binary))
            .or_else(|| {
                let script = root.as_ref().map(|r| r.join(PF_MCP_SCRIPT_REL))?;
                launcher_for(script)
            });

        Some(Self {
            cli,
            root,
            gate,
            http_url,
        })
    }
}

/// `.py` paths need a python3 host; anything else is spawned directly.
fn launcher_for(path: PathBuf) -> Option<GateLauncher> {
    if !path.is_file() {
        return None;
    }
    if path.extension().is_some_and(|e| e == "py") {
        let python = python3_path()?;
        Some(GateLauncher::Script {
            python,
            script: path,
        })
    } else {
        Some(GateLauncher::Binary(path))
    }
}

/// `proofship-pf-mcp` installed next to the running executable (dev builds
/// share target/, packaged apps share the bundle bin dir).
fn bundled_gate() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let sibling = exe.parent()?.join(PF_MCP_BIN);
    sibling.is_file().then_some(sibling)
}

/// Attach ProofForge MCP and prepend the ProgramV1 skill to the agent
/// prompt. The doc keeps the raw user message — only the harness sees the
/// enriched prompt. No-op when no ProofForge toolchain is detected.
pub fn enrich_run_request(request: RunRequest) -> RunRequest {
    match Toolchain::detect() {
        Some(toolchain) => enrich_with(request, &toolchain),
        None => request,
    }
}

fn enrich_with(mut request: RunRequest, toolchain: &Toolchain) -> RunRequest {
    if request.mcp_servers.is_empty() && !env_truthy("PROOFSHIP_DISABLE_PF_MCP") {
        request.mcp_servers = mcp_servers(toolchain, &request.cwd);
    }

    if !env_truthy("PROOFSHIP_DISABLE_PF_SKILL") && !request.prompt.contains(SKILL_PROMPT_MARKER) {
        let skill = strip_yaml_frontmatter(PROGRAM_V1_SKILL);
        let cli = toolchain
            .cli
            .as_ref()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|| "proof-forge-next".into());
        request.prompt = format!(
            "{SKILL_PROMPT_MARKER}\n{skill}\n\n## Local gate tools\n\n\
             ProofForge MCP exposes `pf_doctor` / `pf_check` / `pf_build` / `pf_artifacts`. \
             Prefer those tools for the gate. CLI fallback: `{cli}`.\n\n\
             ## User request\n\n{}",
            request.prompt
        );
    }

    request
}

/// MCP server list for the ACP `session/new`: stdio gate first, then the
/// optional HTTP endpoint.
fn mcp_servers(toolchain: &Toolchain, cwd: &str) -> Vec<McpServerConfig> {
    let mut servers = Vec::new();
    match toolchain.gate.as_ref() {
        Some(GateLauncher::Binary(bin)) => {
            servers.push(McpServerConfig::stdio(
                "proof-forge",
                bin.to_string_lossy(),
                Vec::new(),
                mcp_env(toolchain, cwd),
            ));
        }
        Some(GateLauncher::Script { python, script }) => {
            servers.push(McpServerConfig::stdio(
                "proof-forge",
                python.to_string_lossy(),
                vec!["-I".into(), script.to_string_lossy().into_owned()],
                mcp_env(toolchain, cwd),
            ));
        }
        None => {}
    }
    if let Some(url) = toolchain.http_url.as_ref() {
        servers.push(McpServerConfig::http("proof-forge-http", url));
    }
    servers
}

fn mcp_env(toolchain: &Toolchain, cwd: &str) -> Vec<McpEnvVar> {
    let mut env = Vec::new();
    if let Some(cli) = toolchain.cli.as_ref() {
        let cli = cli.to_string_lossy();
        push_env(&mut env, "PROOF_FORGE_CLI", &cli);
        push_env(&mut env, "PF_CLI", &cli);
    }
    if let Some(root) = toolchain.root.as_ref() {
        push_env(&mut env, "PROOF_FORGE_ROOT", &root.to_string_lossy());
    }
    // Passthroughs the PF server understands when the operator set them.
    for key in ["PROOF_FORGE_TOOL_ROOT", "ELAN_TOOLCHAIN"] {
        if let Ok(value) = std::env::var(key) {
            if !value.is_empty() {
                push_env(&mut env, key, &value);
            }
        }
    }
    // Gate outputs land next to the session's working tree.
    push_env(&mut env, "PROOFSHIP_PROJECT_ROOT", cwd);
    env
}

fn push_env(env: &mut Vec<McpEnvVar>, name: &str, value: &str) {
    env.push(McpEnvVar {
        name: name.into(),
        value: value.into(),
    });
}

fn strip_yaml_frontmatter(text: &str) -> &str {
    let Some(rest) = text.strip_prefix("---") else {
        return text.trim();
    };
    let rest = rest.strip_prefix('\n').unwrap_or(rest);
    let Some(end) = rest.find("\n---") else {
        return text.trim();
    };
    let after = &rest[end + "\n---".len()..];
    after.strip_prefix('\n').unwrap_or(after).trim()
}

fn env_path(key: &str) -> Option<PathBuf> {
    std::env::var_os(key)
        .map(PathBuf::from)
        .filter(|p| !p.as_os_str().is_empty())
}

fn python3_path() -> Option<PathBuf> {
    for cand in ["/usr/bin/python3", "/bin/python3"] {
        let p = PathBuf::from(cand);
        if p.is_file() {
            return Some(p);
        }
    }
    find_on_path("python3")
}

fn find_on_path(exe: &str) -> Option<PathBuf> {
    std::env::var_os("PATH").and_then(|path| {
        std::env::split_paths(&path)
            .map(|d| d.join(exe))
            .find(|p| p.is_file())
    })
}

fn env_truthy(key: &str) -> bool {
    matches!(
        std::env::var(key).ok().as_deref(),
        Some("1" | "true" | "TRUE" | "yes" | "YES")
    )
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use zeron_proto::SandboxLevel;

    fn bare_request(prompt: &str) -> RunRequest {
        RunRequest {
            prompt: prompt.into(),
            harness: None,
            model: None,
            reasoning: None,
            model_options: Default::default(),
            cwd: "/tmp".into(),
            sandbox: SandboxLevel::WorkspaceWrite,
            auto_approve: true,
            resume: None,
            attachments: Vec::new(),
            mcp_servers: Vec::new(),
        }
    }

    fn fake_toolchain(dir: &Path) -> Toolchain {
        let cli = dir.join("proof-forge-next");
        let gate = dir.join(PF_MCP_BIN);
        std::fs::write(&cli, "#!/bin/sh\n").unwrap();
        std::fs::write(&gate, "#!/bin/sh\n").unwrap();
        Toolchain {
            cli: Some(cli),
            root: Some(dir.to_path_buf()),
            gate: Some(GateLauncher::Binary(gate)),
            http_url: None,
        }
    }

    #[test]
    fn embedded_skill_is_program_v1() {
        let body = strip_yaml_frontmatter(PROGRAM_V1_SKILL);
        assert!(body.contains("import ProofForgeV2"));
        assert!(body.contains("ProgramV1"));
        assert!(!body.starts_with("---"), "frontmatter must be stripped");
    }

    #[test]
    fn enrich_prepends_skill_once() {
        let temp = tempfile::tempdir().unwrap();
        let toolchain = fake_toolchain(temp.path());

        let once = enrich_with(bare_request("make an escrow"), &toolchain);
        assert!(once.prompt.contains(SKILL_PROMPT_MARKER));
        assert!(once.prompt.contains("make an escrow"));
        assert!(once.prompt.contains("import ProofForgeV2"));

        let twice = enrich_with(once.clone(), &toolchain);
        assert_eq!(
            twice.prompt.matches(SKILL_PROMPT_MARKER).count(),
            1,
            "must not double-inject on resume/retry"
        );
    }

    #[test]
    fn enrich_attaches_bundled_gate_with_env() {
        let temp = tempfile::tempdir().unwrap();
        let toolchain = fake_toolchain(temp.path());

        let req = enrich_with(bare_request("hi"), &toolchain);
        let Some(McpServerConfig::Stdio {
            name,
            command,
            args,
            env,
        }) = req.mcp_servers.first()
        else {
            panic!("expected stdio proof-forge server, got {:?}", req.mcp_servers);
        };
        assert_eq!(name, "proof-forge");
        assert!(command.ends_with(PF_MCP_BIN), "spawns our binary directly");
        assert!(args.is_empty(), "bundled binary takes no args");
        let map: std::collections::HashMap<_, _> = env
            .iter()
            .map(|e| (e.name.as_str(), e.value.as_str()))
            .collect();
        assert!(map.get("PROOF_FORGE_CLI").is_some());
        assert_eq!(map.get("PROOFSHIP_PROJECT_ROOT"), Some(&"/tmp"));
    }

    #[test]
    fn legacy_python_script_still_launches() {
        let temp = tempfile::tempdir().unwrap();
        let script = temp.path().join("proof_forge_mcp_server.py");
        std::fs::write(&script, "print('ok')\n").unwrap();
        let Some(GateLauncher::Script { python, script }) = launcher_for(script) else {
            if python3_path().is_none() {
                return; // no python3 in this CI
            }
            panic!("expected script launcher for .py path");
        };
        let toolchain = Toolchain {
            gate: Some(GateLauncher::Script { python, script }),
            ..Toolchain::default()
        };
        let req = enrich_with(bare_request("hi"), &toolchain);
        let Some(McpServerConfig::Stdio { args, .. }) = req.mcp_servers.first() else {
            panic!("expected stdio server");
        };
        assert!(args.iter().any(|a| a.ends_with("proof_forge_mcp_server.py")));
    }

    #[test]
    fn preexisting_mcp_servers_are_kept() {
        let temp = tempfile::tempdir().unwrap();
        let toolchain = fake_toolchain(temp.path());

        let mut req = bare_request("hi");
        req.mcp_servers = vec![McpServerConfig::http("custom", "https://example.test/mcp")];
        let out = enrich_with(req, &toolchain);
        assert_eq!(out.mcp_servers.len(), 1, "caller-provided MCP wins");
    }

    #[test]
    fn http_only_toolchain_attaches_http() {
        let toolchain = Toolchain {
            http_url: Some("https://example.test/mcp".into()),
            ..Toolchain::default()
        };
        let req = enrich_with(bare_request("hi"), &toolchain);
        assert!(matches!(
            req.mcp_servers.first(),
            Some(McpServerConfig::Http { .. })
        ));
    }
}
