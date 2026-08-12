//! Discover ProgramV1 sources the Sessions Preview pane can deploy.
//!
//! Studio chat no longer holds drafts. Sessions agents write Lean next to the
//! project; the gate stages copies under `studio-inbox/`. This scanner lists
//! those files plus any leftover launch-store drafts so Deploy does not depend
//! on the retired Studio chat surface.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use comet_proto::{StudioCandidate, StudioGateReport, StudioLaunch};

const MAX_SOURCE: usize = 64 * 1024;

/// Scan inbox roots and launch drafts. Later modules win only when they carry
/// a certified gate report; otherwise first-seen (inbox before launches).
pub fn discover_candidates(
    inbox_roots: &[PathBuf],
    launches: &[StudioLaunch],
) -> Vec<StudioCandidate> {
    let mut by_module: BTreeMap<String, StudioCandidate> = BTreeMap::new();

    for root in inbox_roots {
        let inbox = root.join("studio-inbox");
        let Ok(entries) = fs::read_dir(&inbox) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("lean") {
                continue;
            }
            let Some(candidate) = candidate_from_lean(&path) else {
                continue;
            };
            insert_candidate(&mut by_module, candidate);
        }
    }

    for launch in launches {
        let Some(module) = launch
            .program
            .as_deref()
            .map(str::trim)
            .filter(|s| valid_module(s))
        else {
            continue;
        };
        let Some(source) = launch.source.as_deref().map(str::trim).filter(|s| !s.is_empty()) else {
            continue;
        };
        if source.len() > MAX_SOURCE || !source.starts_with("import ProofForgeV2") {
            continue;
        }
        insert_candidate(
            &mut by_module,
            StudioCandidate {
                module: module.to_string(),
                source: source.to_string(),
                origin: format!("launch:{}", launch.id),
                digest: None,
                certified: false,
            },
        );
    }

    by_module.into_values().collect()
}

fn insert_candidate(map: &mut BTreeMap<String, StudioCandidate>, next: StudioCandidate) {
    match map.get(&next.module) {
        Some(existing) if existing.certified && !next.certified => {}
        _ => {
            map.insert(next.module.clone(), next);
        }
    }
}

fn candidate_from_lean(path: &Path) -> Option<StudioCandidate> {
    let source = fs::read_to_string(path).ok()?;
    if source.len() > MAX_SOURCE || !source.starts_with("import ProofForgeV2") {
        return None;
    }
    let stem = path.file_stem()?.to_str()?;
    let module = parse_program_name(&source)
        .filter(|name| valid_module(name))
        .unwrap_or_else(|| stem.to_string());
    if !valid_module(&module) {
        return None;
    }
    let (digest, certified) = gate_report_for(path, &module);
    Some(StudioCandidate {
        module,
        source,
        origin: path.to_string_lossy().into_owned(),
        digest,
        certified,
    })
}

fn parse_program_name(source: &str) -> Option<String> {
    for line in source.lines() {
        let trimmed = line.trim();
        let rest = trimmed.strip_prefix("program ")?;
        let name = rest.split_whitespace().next()?.trim();
        if valid_module(name) {
            return Some(name.to_string());
        }
    }
    None
}

fn gate_report_for(lean_path: &Path, module: &str) -> (Option<String>, bool) {
    let inbox = lean_path.parent();
    let report = inbox.map(|dir| {
        dir.join(format!("out-{}", module.to_lowercase()))
            .join(StudioGateReport::FILE_NAME)
    });
    let Some(report) = report else {
        return (None, false);
    };
    let Ok(raw) = fs::read_to_string(report) else {
        return (None, false);
    };
    let Ok(parsed) = serde_json::from_str::<StudioGateReport>(&raw) else {
        return (None, false);
    };
    if parsed.ok {
        (parsed.output_set_digest, parsed.certified)
    } else {
        (None, false)
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

#[cfg(test)]
mod tests {
    use super::*;
    use comet_proto::StudioLaunch;

    fn write_lean(dir: &Path, module: &str, extra: &str) -> PathBuf {
        let inbox = dir.join("studio-inbox");
        fs::create_dir_all(&inbox).unwrap();
        let path = inbox.join(format!("{module}.lean"));
        fs::write(
            &path,
            format!("import ProofForgeV2\nprogram {module} where\n{extra}"),
        )
        .unwrap();
        path
    }

    #[test]
    fn inbox_lean_becomes_a_candidate() {
        let dir = tempfile::tempdir().unwrap();
        write_lean(dir.path(), "RwaShareRegistry", "");
        let found = discover_candidates(&[dir.path().to_path_buf()], &[]);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].module, "RwaShareRegistry");
        assert!(found[0].source.starts_with("import ProofForgeV2"));
        assert!(!found[0].certified);
    }

    #[test]
    fn certified_gate_report_is_attached() {
        let dir = tempfile::tempdir().unwrap();
        write_lean(dir.path(), "TimeLock", "");
        let out = dir
            .path()
            .join("studio-inbox/out-timelock");
        fs::create_dir_all(&out).unwrap();
        fs::write(
            out.join("gate-report.json"),
            r#"{
              "schemaVersion": 1,
              "ok": true,
              "module": "TimeLock",
              "target": "evm",
              "outputSetDigest": "deadbeef",
              "artifacts": [],
              "certified": true,
              "honesty": "x",
              "generatedAt": "2026-08-13T00:00:00Z"
            }"#,
        )
        .unwrap();
        let found = discover_candidates(&[dir.path().to_path_buf()], &[]);
        assert_eq!(found[0].digest.as_deref(), Some("deadbeef"));
        assert!(found[0].certified);
    }

    #[test]
    fn rejects_non_programv1_and_bad_names() {
        let dir = tempfile::tempdir().unwrap();
        let inbox = dir.path().join("studio-inbox");
        fs::create_dir_all(&inbox).unwrap();
        fs::write(inbox.join("Nope.lean"), "def x := 1\n").unwrap();
        fs::write(inbox.join("bad-name.lean"), "import ProofForgeV2\n").unwrap();
        let found = discover_candidates(&[dir.path().to_path_buf()], &[]);
        assert!(found.is_empty());
    }

    #[test]
    fn launch_draft_fills_gap_without_overwriting_certified() {
        let dir = tempfile::tempdir().unwrap();
        write_lean(dir.path(), "OnDisk", "");
        let out = dir.path().join("studio-inbox/out-ondisk");
        fs::create_dir_all(&out).unwrap();
        fs::write(
            out.join("gate-report.json"),
            r#"{
              "schemaVersion": 1,
              "ok": true,
              "module": "OnDisk",
              "target": "evm",
              "artifacts": [],
              "certified": true,
              "honesty": "x",
              "generatedAt": "2026-08-13T00:00:00Z"
            }"#,
        )
        .unwrap();
        let mut launch = StudioLaunch::new_now();
        launch.program = Some("FromLaunch".into());
        launch.source = Some("import ProofForgeV2\nprogram FromLaunch where\n".into());
        let mut stale = StudioLaunch::new_now();
        stale.program = Some("OnDisk".into());
        stale.source = Some("import ProofForgeV2\nprogram OnDisk where\n-- stale\n".into());
        let found = discover_candidates(&[dir.path().to_path_buf()], &[launch, stale]);
        assert_eq!(found.len(), 2);
        let on_disk = found.iter().find(|c| c.module == "OnDisk").unwrap();
        assert!(on_disk.certified);
        assert!(!on_disk.source.contains("stale"));
        assert!(found.iter().any(|c| c.module == "FromLaunch"));
    }
}
