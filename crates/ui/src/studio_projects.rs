//! Project grouping for Launch Studio (Phase 2.5).

use comet_proto::{DeploymentRecord, StudioChatMsg, StudioGateState, StudioLaunch};

/// Group launches by `project_name`, falling back to `"Ungrouped"`.
/// Groups are sorted by name; launches inside a group keep caller order.
pub fn group_launches(launches: &[StudioLaunch]) -> Vec<(String, Vec<&StudioLaunch>)> {
    let mut groups: Vec<(String, Vec<&StudioLaunch>)> = Vec::new();
    for launch in launches {
        let name = launch
            .project_name
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or("Ungrouped")
            .to_string();
        if let Some((_, bucket)) = groups.iter_mut().find(|(existing, _)| existing == &name) {
            bucket.push(launch);
        } else {
            groups.push((name, vec![launch]));
        }
    }
    groups.sort_by(|a, b| a.0.cmp(&b.0));
    groups
}

/// Launches that belong to the same project as `current` (by `project_id`,
/// falling back to `project_name`).
pub fn launches_in_project<'a>(
    launches: &'a [StudioLaunch],
    current: &StudioLaunch,
) -> Vec<&'a StudioLaunch> {
    let id = current.project_id.as_deref().map(str::trim).filter(|s| !s.is_empty());
    let name = current
        .project_name
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    launches
        .iter()
        .filter(|launch| {
            if let (Some(want), Some(have)) = (id, launch.project_id.as_deref()) {
                return want == have;
            }
            match (name, launch.project_name.as_deref().map(str::trim)) {
                (Some(want), Some(have)) if !have.is_empty() => want == have,
                (None, None) | (None, Some("")) => launch.id == current.id,
                _ => false,
            }
        })
        .collect()
}

/// Deployments that belong to a project: explicit `project_id`, or a launch in
/// that project.
pub fn deployments_for_project<'a>(
    deployments: &'a [DeploymentRecord],
    project_id: Option<&str>,
    project_launch_ids: &[&str],
) -> Vec<&'a DeploymentRecord> {
    let project_id = project_id.map(str::trim).filter(|s| !s.is_empty());
    deployments
        .iter()
        .filter(|dep| {
            if let (Some(want), Some(have)) = (project_id, dep.project_id.as_deref()) {
                return want == have;
            }
            dep.launch_id
                .as_deref()
                .is_some_and(|lid| project_launch_ids.iter().any(|id| *id == lid))
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectSummary {
    pub name: String,
    pub id: String,
    pub path: Option<String>,
    pub launch_count: usize,
    pub program: Option<String>,
    pub source_chars: usize,
    pub gate_passes: usize,
    pub gate_fails: usize,
    pub last_digest: Option<String>,
    pub deployment_count: usize,
}

pub fn summarize_project(
    current: &StudioLaunch,
    siblings: &[&StudioLaunch],
    deployments: &[&DeploymentRecord],
) -> ProjectSummary {
    let mut gate_passes = 0usize;
    let mut gate_fails = 0usize;
    let mut last_digest = None;
    for launch in siblings {
        for msg in &launch.msgs {
            if let StudioChatMsg::AgentGate(gate) = msg {
                match gate.state {
                    StudioGateState::Pass => gate_passes += 1,
                    StudioGateState::Fail => gate_fails += 1,
                    _ => {}
                }
                if let Some(inspect) = gate.result.as_ref().and_then(|r| r.inspect.as_ref()) {
                    if let Some(digest) = extract_digest(inspect) {
                        last_digest = Some(digest);
                    }
                }
            }
        }
    }
    // Prefer the active launch's live fields for program/source.
    let program = current
        .program
        .clone()
        .or_else(|| siblings.iter().find_map(|l| l.program.clone()));
    let source_chars = current
        .source
        .as_ref()
        .map(|s| s.len())
        .or_else(|| siblings.iter().find_map(|l| l.source.as_ref().map(|s| s.len())))
        .unwrap_or(0);
    ProjectSummary {
        name: current
            .project_name
            .clone()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| "Studio".into()),
        id: current
            .project_id
            .clone()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| "studio".into()),
        path: current.project_path.clone(),
        launch_count: siblings.len().max(1),
        program,
        source_chars,
        gate_passes,
        gate_fails,
        last_digest,
        deployment_count: deployments.len(),
    }
}

fn extract_digest(raw: &str) -> Option<String> {
    let idx = raw.find("outputSetDigest")?;
    raw[idx..]
        .split(|c: char| !(c.is_ascii_alphanumeric()))
        .find(|part| part.len() == 64 && part.chars().all(|c| c.is_ascii_hexdigit()))
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;
    use comet_proto::{StudioGateMsg, StudioGateRunResult, StudioGateStage};

    fn launch(id: &str, project: Option<&str>) -> StudioLaunch {
        let mut launch = StudioLaunch::new_now();
        launch.id = id.into();
        launch.title = id.into();
        launch.project_name = project.map(str::to_string);
        launch.project_id = project.map(|p| p.to_ascii_lowercase());
        launch
    }

    #[test]
    fn groups_by_project_name_and_keeps_ungrouped() {
        let launches = vec![
            launch("a", Some("RWA")),
            launch("b", None),
            launch("c", Some("RWA")),
            launch("d", Some("Escrow")),
        ];
        let groups = group_launches(&launches);
        assert_eq!(groups.len(), 3);
        assert_eq!(groups[0].0, "Escrow");
        assert_eq!(groups[1].0, "RWA");
        assert_eq!(groups[1].1.len(), 2);
        assert_eq!(groups[2].0, "Ungrouped");
    }

    #[test]
    fn launches_in_project_match_by_id() {
        let launches = vec![
            launch("a", Some("RWA")),
            launch("b", Some("RWA")),
            launch("c", Some("Other")),
        ];
        let peers = launches_in_project(&launches, &launches[0]);
        assert_eq!(peers.len(), 2);
        assert_eq!(peers[0].id, "a");
        assert_eq!(peers[1].id, "b");
    }

    #[test]
    fn deployments_match_project_id_or_launch() {
        let deps = vec![
            DeploymentRecord {
                id: "1".into(),
                launch_id: Some("a".into()),
                project_id: Some("rwa".into()),
                module: "M".into(),
                network_id: "xlayer-testnet".into(),
                address: "0x1".into(),
                ctor: None,
                digest: None,
                tx_hash: "0xt".into(),
                ts: "t".into(),
            },
            DeploymentRecord {
                id: "2".into(),
                launch_id: Some("b".into()),
                project_id: None,
                module: "M".into(),
                network_id: "xlayer-testnet".into(),
                address: "0x2".into(),
                ctor: None,
                digest: None,
                tx_hash: "0xt".into(),
                ts: "t".into(),
            },
            DeploymentRecord {
                id: "3".into(),
                launch_id: Some("z".into()),
                project_id: Some("other".into()),
                module: "M".into(),
                network_id: "xlayer-testnet".into(),
                address: "0x3".into(),
                ctor: None,
                digest: None,
                tx_hash: "0xt".into(),
                ts: "t".into(),
            },
        ];
        let matched = deployments_for_project(&deps, Some("rwa"), &["a", "b"]);
        assert_eq!(matched.len(), 2);
        assert_eq!(matched[0].id, "1");
        assert_eq!(matched[1].id, "2");
    }

    #[test]
    fn summarize_counts_gates_and_deployments() {
        let mut current = launch("a", Some("RWA"));
        current.program = Some("RwaShareRegistry".into());
        current.source = Some("import ProofForgeV2\n".into());
        current.project_path = Some("projects/rwa".into());
        current.msgs.push(StudioChatMsg::AgentGate(StudioGateMsg {
            role: "agent".into(),
            kind: "gate".into(),
            state: StudioGateState::Pass,
            result: Some(StudioGateRunResult {
                ok: true,
                stage: StudioGateStage::Done,
                check: None,
                build: None,
                inspect: Some(
                    "outputSetDigest 0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
                        .into(),
                ),
                error: None,
            }),
            at: "t".into(),
        }));
        let siblings = vec![&current];
        let dep = DeploymentRecord {
            id: "1".into(),
            launch_id: Some("a".into()),
            project_id: Some("rwa".into()),
            module: "RwaShareRegistry".into(),
            network_id: "xlayer-testnet".into(),
            address: "0x1".into(),
            ctor: None,
            digest: None,
            tx_hash: "0xt".into(),
            ts: "t".into(),
        };
        let summary = summarize_project(&current, &siblings, &[&dep]);
        assert_eq!(summary.name, "RWA");
        assert_eq!(summary.launch_count, 1);
        assert_eq!(summary.program.as_deref(), Some("RwaShareRegistry"));
        assert_eq!(summary.gate_passes, 1);
        assert_eq!(summary.deployment_count, 1);
        assert!(summary.last_digest.is_some());
        assert_eq!(summary.path.as_deref(), Some("projects/rwa"));
    }
}
