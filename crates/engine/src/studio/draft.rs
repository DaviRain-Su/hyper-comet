//! ProofShip Studio agent draft runner.
//!
//! Drafting is a one-shot ACP harness run in a scratch inbox directory. The
//! harness is unattended: any user-input request is answered with an empty set
//! so a Launch Studio draft cannot block behind an interactive question panel.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use comet_harness::{CancellationToken, HarnessError, RunControls, SteerMessage};
use comet_proto::{
    AgentEvent, DoneStatus, HarnessId, ReasoningLevel, RunRequest, SandboxLevel, StudioDraftEvent,
    UserInputAnswer, UserInputQuestion,
};
use futures::StreamExt;
use futures::stream::BoxStream;
use tokio::sync::{mpsc, oneshot};

use crate::registry::HarnessRegistry;
use crate::studio::{GateConfig, GateError, StudioGate, StudioPaths};

const NOTE_CAP_CHARS: usize = 600;
pub(crate) const REPAIR_DIAGNOSTIC_CAP_CHARS: usize = 4 * 1024;
const FALLBACK_PROMPT: &str = r#"You draft ProofForge ProgramV1 contracts. Output exactly one .lean file.
First line must be exactly: import ProofForgeV2
Use this skeleton:
namespace Proofship
open ProofForgeV2.Language

program <Module> where
  -- use only ProgramV1 DSL constructs requested by the user

end Proofship
Choose a valid Lean module name from the contract domain. Ask for required numeric or parameter values instead of inventing them. Do not create ABI, bytecode, README, or additional Lean files."#;

#[derive(Debug, thiserror::Error)]
pub enum DraftError {
    #[error("repo root not found")]
    RepoRootNotFound,
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("harness: {0}")]
    Harness(#[from] HarnessError),
    #[error("drafting run error: {0}")]
    Agent(String),
    #[error("drafting run ended {status:?}: {error}")]
    Done { status: DoneStatus, error: String },
    #[error("expected exactly one .lean file in draft workdir, found {0}")]
    LeanFileCount(usize),
    #[error("draft source contract violated: {0}")]
    Gate(#[from] GateError),
}

#[derive(Clone)]
pub struct DraftRunner {
    registry: Arc<HarnessRegistry>,
    paths: StudioPaths,
    seq: Arc<AtomicU64>,
}

impl DraftRunner {
    pub fn new(registry: Arc<HarnessRegistry>, config: GateConfig) -> Self {
        Self {
            registry,
            paths: config.paths,
            seq: Arc::new(AtomicU64::new(1)),
        }
    }

    pub fn draft(&self, nl: String, harness_id: HarnessId) -> BoxStream<'static, StudioDraftEvent> {
        self.draft_with_prompt(nl, harness_id)
    }

    pub(crate) fn draft_with_prompt(
        &self,
        prompt_input: String,
        harness_id: HarnessId,
    ) -> BoxStream<'static, StudioDraftEvent> {
        let this = self.clone();
        let (tx, rx) = mpsc::channel(16);
        tokio::spawn(async move {
            let _ = tx
                .send(StudioDraftEvent::Started { lane: harness_id })
                .await;
            match this.draft_inner(prompt_input, harness_id, tx.clone()).await {
                Ok((module, source)) => {
                    let _ = tx
                        .send(StudioDraftEvent::Done {
                            ok: true,
                            lane: harness_id,
                            module: Some(module),
                            source: Some(source),
                            error: None,
                        })
                        .await;
                }
                Err(err) => {
                    let _ = tx
                        .send(StudioDraftEvent::Done {
                            ok: false,
                            lane: harness_id,
                            module: None,
                            source: None,
                            error: Some(err.to_string()),
                        })
                        .await;
                }
            }
        });
        futures::stream::unfold(rx, |mut rx| async move {
            rx.recv().await.map(|item| (item, rx))
        })
        .boxed()
    }

    async fn draft_inner(
        &self,
        nl: String,
        harness_id: HarnessId,
        tx: mpsc::Sender<StudioDraftEvent>,
    ) -> Result<(String, String), DraftError> {
        let harness = self.registry.resolve(harness_id)?;
        let repo_root = self.repo_root()?;
        let workdir = self.workdir(&repo_root);
        tokio::fs::create_dir_all(&workdir).await?;
        let gate = StudioGate::new(GateConfig {
            paths: self.paths.clone(),
            ..GateConfig::default()
        });
        let status = gate.status();
        let prompt = compose_prompt(&repo_root, &nl, status.pf_cli.as_deref()).await;
        let request = RunRequest {
            prompt,
            harness: Some(harness_id),
            model: None,
            reasoning: Some(ReasoningLevel::High),
            model_options: serde_json::Map::new(),
            cwd: workdir.to_string_lossy().to_string(),
            sandbox: SandboxLevel::WorkspaceWrite,
            auto_approve: true,
            resume: None,
            attachments: Vec::new(),
        };
        let (_steer_tx, steer_rx) = mpsc::channel::<SteerMessage>(1);
        let controls = RunControls {
            request_input: Box::new(|_questions: Vec<UserInputQuestion>| {
                let (tx, rx) = oneshot::channel::<Vec<UserInputAnswer>>();
                let _ = tx.send(Vec::new());
                rx
            }),
            steering: steer_rx,
            interrupt: CancellationToken::new(),
        };
        let mut stream = harness.run(request, controls).await?;
        let mut note = String::new();
        while let Some(event) = stream.next().await {
            match event? {
                AgentEvent::TextDelta { text } => {
                    note.push_str(&text);
                    if note.chars().count() > NOTE_CAP_CHARS {
                        note = note
                            .chars()
                            .rev()
                            .take(NOTE_CAP_CHARS)
                            .collect::<Vec<_>>()
                            .into_iter()
                            .rev()
                            .collect();
                    }
                    if !note.trim().is_empty() {
                        let _ = tx
                            .send(StudioDraftEvent::Note {
                                text: note.trim().to_string(),
                            })
                            .await;
                    }
                }
                AgentEvent::Error { message } => return Err(DraftError::Agent(message)),
                AgentEvent::Done { status, error, .. } => {
                    if status == DoneStatus::Completed {
                        break;
                    }
                    return Err(DraftError::Done {
                        status,
                        error: error.unwrap_or_default(),
                    });
                }
                _ => {}
            }
        }
        collect_lean(&workdir).await
    }

    fn workdir(&self, repo_root: &Path) -> PathBuf {
        let root = self
            .paths
            .inbox_root
            .clone()
            .unwrap_or_else(|| repo_root.join("proofship/inbox"));
        let id = self.seq.fetch_add(1, Ordering::Relaxed);
        root.join("agent").join(format!("draft-{id}"))
    }

    fn repo_root(&self) -> Result<PathBuf, DraftError> {
        if let Some(root) = &self.paths.repo_root {
            return Ok(root.clone());
        }
        if let Some(root) = std::env::var_os("PROOFSHIP_REPO_ROOT").filter(|s| !s.is_empty()) {
            return Ok(PathBuf::from(root));
        }
        let mut starts = Vec::new();
        if let Some(exe_dir) = &self.paths.exe_dir {
            starts.push(exe_dir.clone());
        } else if let Ok(exe) = std::env::current_exe()
            && let Some(parent) = exe.parent()
        {
            starts.push(parent.to_path_buf());
        }
        if let Some(cwd) = &self.paths.cwd {
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
        Err(DraftError::RepoRootNotFound)
    }
}

async fn compose_prompt(repo_root: &Path, nl: &str, pf_cli: Option<&str>) -> String {
    let system =
        tokio::fs::read_to_string(repo_root.join("proofship/prompts/program-v1-author.md"))
            .await
            .unwrap_or_else(|_| FALLBACK_PROMPT.to_string());
    let cli = pf_cli.unwrap_or("proof-forge-next");
    format!(
        "{system}\n\n## Local gate self-check\n\nThe resolved ProofForge CLI is `{cli}`. Before returning, self-verify from the repository's `proofship/inbox` project root (or the current draft workdir if you copy the file there) with:\n\n```sh\n{cli} check <Module>.lean --module <Module>\n{cli} build <Module>.lean --module <Module> --target evm -o out-<module-lowercase>\n{cli} inspect --output-dir out-<module-lowercase>\n```\n\nUse the absolute CLI path exactly as shown when available.\n\n## User request\n\n{nl}\n\nWrite exactly one `<Module>.lean` file in the current working directory. Choose `<Module>` from the contract domain as a valid Lean identifier."
    )
}

pub(crate) fn compose_repair_prompt(
    original_nl: &str,
    failed_module: &str,
    failed_source: &str,
    diagnostics: &str,
) -> String {
    format!(
        "## Repair request\n\nOriginal user request:\n\n{original_nl}\n\nThe previous draft `{failed_module}.lean` failed the ProofForge machine gate. Follow the repair-loop discipline in the system prompt: preserve the user's intent, change only what is needed to fix the PF-* diagnostics, and return exactly one revised `.lean` file.\n\nFailed source:\n\n```lean\n{failed_source}\n```\n\nGate diagnostics (truncated to the last ~4KiB):\n\n```text\n{}\n```",
        tail_chars(diagnostics, REPAIR_DIAGNOSTIC_CAP_CHARS)
    )
}

pub(crate) fn tail_chars(text: &str, cap: usize) -> String {
    let count = text.chars().count();
    if count <= cap {
        text.to_string()
    } else {
        let kept: String = text.chars().skip(count - cap).collect();
        format!("[diagnostics truncated; kept last {cap} chars]\n{kept}")
    }
}

async fn collect_lean(workdir: &Path) -> Result<(String, String), DraftError> {
    let mut entries = tokio::fs::read_dir(workdir).await?;
    let mut files = Vec::new();
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if entry.metadata().await?.is_file()
            && path.extension().and_then(|s| s.to_str()) == Some("lean")
        {
            files.push(path);
        }
    }
    files.sort();
    if files.len() != 1 {
        return Err(DraftError::LeanFileCount(files.len()));
    }
    let path = files.remove(0);
    let source = tokio::fs::read_to_string(&path).await?;
    let module = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
        .to_string();
    StudioGate::validate(&module, &source)?;
    Ok((module, source))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use comet_harness::mock::MockHarness;
    use comet_proto::{AgentEvent, DoneStatus, HarnessId};
    use futures::StreamExt;

    use super::*;
    use crate::registry::HarnessRegistry;

    #[tokio::test]
    async fn mock_draft_collects_single_lean_file() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(temp.path().join("proofship/scripts")).unwrap();
        std::fs::write(temp.path().join("proofship/scripts/gate.sh"), "#!/bin/sh\n").unwrap();
        let inbox = temp.path().join("studio/inbox");
        let writer_root = inbox.clone();
        let registry = Arc::new(HarnessRegistry::new());
        registry.register(Arc::new(MockHarness {
            script: vec![
                AgentEvent::TextDelta {
                    text: "drafting".into(),
                },
                AgentEvent::ToolCall {
                    id: "write".into(),
                    call: comet_proto::ToolCall::WriteFile {
                        path: "Demo.lean".into(),
                        content: Some(String::new()),
                    },
                },
                AgentEvent::Done {
                    status: DoneStatus::Completed,
                    result: None,
                    error: None,
                    session_id: None,
                },
            ],
        }));
        let runner = DraftRunner::new(
            registry,
            GateConfig {
                paths: StudioPaths {
                    repo_root: Some(temp.path().to_path_buf()),
                    inbox_root: Some(inbox),
                    ..StudioPaths::default()
                },
                ..GateConfig::default()
            },
        );
        let mut stream = runner.draft("demo contract".into(), HarnessId::Mock);
        let first = stream.next().await.unwrap();
        assert!(matches!(first, StudioDraftEvent::Started { .. }));
        let draft_dir = writer_root.join("agent/draft-1");
        tokio::fs::create_dir_all(&draft_dir).await.unwrap();
        tokio::fs::write(
            draft_dir.join("Demo.lean"),
            "import ProofForgeV2\nnamespace Proofship\nopen ProofForgeV2.Language\nend Proofship\n",
        )
        .await
        .unwrap();
        let mut done = None;
        while let Some(event) = stream.next().await {
            if let StudioDraftEvent::Done { .. } = event {
                done = Some(event);
                break;
            }
        }
        match done.unwrap() {
            StudioDraftEvent::Done {
                ok, module, source, ..
            } => {
                assert!(ok);
                assert_eq!(module.as_deref(), Some("Demo"));
                assert!(source.unwrap().starts_with("import ProofForgeV2"));
            }
            _ => unreachable!(),
        }
    }
}
