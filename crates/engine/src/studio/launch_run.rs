//! ProofShip Studio bounded draft → gate → repair orchestration.

use std::time::Duration;

use comet_proto::{
    HarnessId, StudioDraftEvent, StudioGateArtifact, StudioGateDigest, StudioGateEvent,
    StudioGateStage, StudioLaunchRunEvent, StudioLaunchRunPhase,
};
use futures::StreamExt;
use futures::stream::BoxStream;
use tokio::sync::mpsc;

use crate::studio::draft::{REPAIR_DIAGNOSTIC_CAP_CHARS, compose_repair_prompt, tail_chars};
use crate::studio::{DraftRunner, StudioGate};

pub(crate) const REPAIR_ROUND_LIMIT: u32 = 4;
pub(crate) const LAUNCH_RUN_TIMEOUT: Duration = Duration::from_secs(30 * 60);

#[derive(Clone)]
pub struct StudioLaunchRunner {
    draft: DraftRunner,
    gate: StudioGate,
}

#[derive(Debug, Clone)]
struct GateOutcome {
    ok: bool,
    stage: StudioGateStage,
    artifacts: Vec<StudioGateArtifact>,
    digest: StudioGateDigest,
    diagnostics: Option<String>,
}

impl StudioLaunchRunner {
    pub fn new(draft: DraftRunner, gate: StudioGate) -> Self {
        Self { draft, gate }
    }

    pub fn launch_run(
        &self,
        nl: String,
        harness_id: HarnessId,
    ) -> BoxStream<'static, StudioLaunchRunEvent> {
        let this = self.clone();
        let (tx, rx) = mpsc::channel(32);
        tokio::spawn(async move {
            let _ = tokio::time::timeout(
                LAUNCH_RUN_TIMEOUT,
                this.run_loop(nl, harness_id, tx.clone()),
            )
            .await
            .map_err(|_| async {
                let _ = tx
                    .send(StudioLaunchRunEvent::Done {
                        ok: false,
                        round: REPAIR_ROUND_LIMIT,
                        module: None,
                        source: None,
                        artifacts: Vec::new(),
                        digest: StudioGateDigest::default(),
                        last_diagnostics: Some("launch run timed out after 30 minutes".into()),
                        exhausted: true,
                    })
                    .await;
            });
        });
        futures::stream::unfold(rx, |mut rx| async move {
            rx.recv().await.map(|item| (item, rx))
        })
        .boxed()
    }

    async fn run_loop(
        &self,
        nl: String,
        harness_id: HarnessId,
        tx: mpsc::Sender<StudioLaunchRunEvent>,
    ) {
        let mut prompt_input = nl.clone();
        let mut last_module = None;
        let mut last_source = None;
        let mut last_diagnostics = None;
        for round in 1..=REPAIR_ROUND_LIMIT {
            let Some((module, source)) = self
                .run_draft_round(round, prompt_input.clone(), harness_id, &tx)
                .await
            else {
                return;
            };
            last_module = Some(module.clone());
            last_source = Some(source.clone());
            let outcome = self
                .run_gate_round(round, module.clone(), source.clone(), &tx)
                .await;
            if outcome.ok {
                let _ = tx
                    .send(StudioLaunchRunEvent::Done {
                        ok: true,
                        round,
                        module: Some(module),
                        source: Some(source),
                        artifacts: outcome.artifacts,
                        digest: outcome.digest,
                        last_diagnostics: None,
                        exhausted: false,
                    })
                    .await;
                return;
            }
            let diagnostics = outcome
                .diagnostics
                .unwrap_or_else(|| format!("gate failed at {}", gate_stage_name(outcome.stage)));
            let diagnostics = tail_chars(&diagnostics, REPAIR_DIAGNOSTIC_CAP_CHARS);
            last_diagnostics = Some(diagnostics.clone());
            if round == REPAIR_ROUND_LIMIT {
                break;
            }
            prompt_input = compose_repair_prompt(&nl, &module, &source, &diagnostics);
        }
        let _ = tx
            .send(StudioLaunchRunEvent::Done {
                ok: false,
                round: REPAIR_ROUND_LIMIT,
                module: last_module,
                source: last_source,
                artifacts: Vec::new(),
                digest: StudioGateDigest::default(),
                last_diagnostics,
                exhausted: true,
            })
            .await;
    }

    async fn run_draft_round(
        &self,
        round: u32,
        prompt_input: String,
        harness_id: HarnessId,
        tx: &mpsc::Sender<StudioLaunchRunEvent>,
    ) -> Option<(String, String)> {
        let mut stream = self.draft.draft_with_prompt(prompt_input, harness_id);
        while let Some(event) = stream.next().await {
            let done = match &event {
                StudioDraftEvent::Done {
                    ok: true,
                    module: Some(module),
                    source: Some(source),
                    ..
                } => Some(Ok((module.clone(), source.clone()))),
                StudioDraftEvent::Done {
                    ok: false, error, ..
                } => Some(Err(error.clone().unwrap_or_else(|| "draft failed".into()))),
                _ => None,
            };
            if tx
                .send(StudioLaunchRunEvent::Draft {
                    round,
                    phase: StudioLaunchRunPhase::Draft,
                    event,
                })
                .await
                .is_err()
            {
                return None;
            }
            match done {
                Some(Ok(result)) => return Some(result),
                Some(Err(error)) => {
                    let _ = tx
                        .send(StudioLaunchRunEvent::Done {
                            ok: false,
                            round,
                            module: None,
                            source: None,
                            artifacts: Vec::new(),
                            digest: StudioGateDigest::default(),
                            last_diagnostics: Some(error),
                            exhausted: false,
                        })
                        .await;
                    return None;
                }
                None => {}
            }
        }
        None
    }

    async fn run_gate_round(
        &self,
        round: u32,
        module: String,
        source: String,
        tx: &mpsc::Sender<StudioLaunchRunEvent>,
    ) -> GateOutcome {
        let mut stream = self.gate.run_gate(module, source);
        let mut last_output = None;
        let mut last_stage = StudioGateStage::Check;
        while let Some(event) = stream.next().await {
            match &event {
                StudioGateEvent::StageDone { stage, output, .. } => {
                    last_stage = *stage;
                    if !output.trim().is_empty() {
                        last_output = Some(output.clone());
                    }
                }
                StudioGateEvent::Done {
                    ok,
                    stage,
                    artifacts,
                    digest,
                } => {
                    let outcome = GateOutcome {
                        ok: *ok,
                        stage: *stage,
                        artifacts: artifacts.clone(),
                        digest: digest.clone(),
                        diagnostics: last_output.clone(),
                    };
                    let _ = tx
                        .send(StudioLaunchRunEvent::Gate {
                            round,
                            phase: StudioLaunchRunPhase::Gate,
                            event,
                        })
                        .await;
                    return outcome;
                }
                _ => {}
            }
            if tx
                .send(StudioLaunchRunEvent::Gate {
                    round,
                    phase: StudioLaunchRunPhase::Gate,
                    event,
                })
                .await
                .is_err()
            {
                break;
            }
        }
        GateOutcome {
            ok: false,
            stage: last_stage,
            artifacts: Vec::new(),
            digest: StudioGateDigest::default(),
            diagnostics: last_output,
        }
    }
}

fn gate_stage_name(stage: StudioGateStage) -> &'static str {
    match stage {
        StudioGateStage::Check => "check",
        StudioGateStage::Build => "build",
        StudioGateStage::Inspect => "inspect",
        StudioGateStage::Done => "done",
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use comet_harness::mock::MockHarness;
    use comet_proto::{AgentEvent, DoneStatus};

    use super::*;
    use crate::registry::HarnessRegistry;
    use crate::studio::{GateConfig, StudioPaths};

    fn executable(path: &std::path::Path, script: &str) {
        std::fs::write(path, script).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(path).unwrap().permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(path, perms).unwrap();
        }
    }

    fn registry() -> Arc<HarnessRegistry> {
        let registry = Arc::new(HarnessRegistry::new());
        registry.register(Arc::new(MockHarness {
            script: vec![AgentEvent::Done {
                status: DoneStatus::Completed,
                result: None,
                error: None,
                session_id: None,
            }],
        }));
        registry
    }

    async fn write_drafts(inbox: std::path::PathBuf, sources: Vec<&'static str>) {
        tokio::spawn(async move {
            for (ix, source) in sources.into_iter().enumerate() {
                let dir = inbox.join("agent").join(format!("draft-{}", ix + 1));
                loop {
                    if tokio::fs::create_dir_all(&dir).await.is_ok()
                        && tokio::fs::write(dir.join("Demo.lean"), source)
                            .await
                            .is_ok()
                    {
                        break;
                    }
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
            }
        });
    }

    fn config(
        root: &std::path::Path,
        inbox: std::path::PathBuf,
        cli: std::path::PathBuf,
    ) -> GateConfig {
        GateConfig {
            paths: StudioPaths {
                repo_root: Some(root.to_path_buf()),
                inbox_root: Some(inbox),
                pf_cli: Some(cli),
                ..StudioPaths::default()
            },
            timeout: Duration::from_secs(5),
        }
    }

    #[tokio::test]
    async fn launch_run_repairs_after_gate_failure_and_passes_round_two() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(temp.path().join("proofship/scripts")).unwrap();
        std::fs::write(temp.path().join("proofship/scripts/gate.sh"), "#!/bin/sh\n").unwrap();
        let cli = temp.path().join("proof-forge-next");
        executable(
            &cli,
            "#!/bin/sh\ncase \"$1\" in\ncheck) grep -q BAD \"$2\" && { echo 'PF-001 bad draft'; exit 1; } || exit 0;;\nbuild) out=\"\"; prev=\"\"; for arg in \"$@\"; do if [ \"$prev\" = \"-o\" ]; then out=\"$arg\"; fi; prev=\"$arg\"; done; mkdir -p \"$out\"; echo bin > \"$out/Demo.bin\"; exit 0;;\ninspect) echo 'outputSetDigest 0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef'; exit 0;;\nesac\n",
        );
        let inbox = temp.path().join("proofship/inbox");
        let runner = StudioLaunchRunner::new(
            DraftRunner::new(registry(), config(temp.path(), inbox.clone(), cli.clone())),
            StudioGate::new(config(temp.path(), inbox.clone(), cli)),
        );
        write_drafts(
            inbox,
            vec![
                "import ProofForgeV2\n-- BAD\nnamespace Proofship\nopen ProofForgeV2.Language\nend Proofship\n",
                "import ProofForgeV2\nnamespace Proofship\nopen ProofForgeV2.Language\nend Proofship\n",
            ],
        )
        .await;
        let events: Vec<_> = runner
            .launch_run("demo".into(), HarnessId::Mock)
            .collect()
            .await;
        assert!(
            events
                .iter()
                .any(|event| matches!(event, StudioLaunchRunEvent::Gate { round: 1, .. }))
        );
        assert!(
            events
                .iter()
                .any(|event| matches!(event, StudioLaunchRunEvent::Draft { round: 2, .. }))
        );
        assert!(matches!(
            events.last(),
            Some(StudioLaunchRunEvent::Done {
                ok: true,
                round: 2,
                exhausted: false,
                ..
            })
        ));
    }

    #[tokio::test]
    async fn launch_run_exhausts_after_four_failed_rounds() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(temp.path().join("proofship/scripts")).unwrap();
        std::fs::write(temp.path().join("proofship/scripts/gate.sh"), "#!/bin/sh\n").unwrap();
        let cli = temp.path().join("proof-forge-next");
        executable(&cli, "#!/bin/sh\necho 'PF-999 nope'\nexit 1\n");
        let inbox = temp.path().join("proofship/inbox");
        let runner = StudioLaunchRunner::new(
            DraftRunner::new(registry(), config(temp.path(), inbox.clone(), cli.clone())),
            StudioGate::new(config(temp.path(), inbox.clone(), cli)),
        );
        write_drafts(
            inbox,
            vec![
                "import ProofForgeV2\nnamespace Proofship\nopen ProofForgeV2.Language\nend Proofship\n",
                "import ProofForgeV2\nnamespace Proofship\nopen ProofForgeV2.Language\nend Proofship\n",
                "import ProofForgeV2\nnamespace Proofship\nopen ProofForgeV2.Language\nend Proofship\n",
                "import ProofForgeV2\nnamespace Proofship\nopen ProofForgeV2.Language\nend Proofship\n",
            ],
        )
        .await;
        let events: Vec<_> = runner
            .launch_run("demo".into(), HarnessId::Mock)
            .collect()
            .await;
        assert!(
            matches!(events.last(), Some(StudioLaunchRunEvent::Done { ok: false, round: 4, exhausted: true, last_diagnostics: Some(diag), .. }) if diag.contains("PF-999"))
        );
    }
}
