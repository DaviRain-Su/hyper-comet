//! Enrich ordinary Sessions runs with ProofForge skill + MCP.
//!
//! Studio chat UI is deprecated as a product entry; the Cursor-shaped path is
//! Sessions → ACP agent with skill text + stdio MCP tools.

use std::path::{Path, PathBuf};

use comet_proto::RunRequest;

use super::gate::{GateConfig, StudioGate, StudioPaths};
use super::mcp::resolve_studio_mcp_servers;

/// Marker so resume / retry does not double-inject the skill body.
pub const SKILL_PROMPT_MARKER: &str = "<!-- proofship:proofforge-program-v1 -->";

const AUTHOR_SKILL_REL: &str = ".agents/skills/proofforge-program-v1/SKILL.md";

/// Attach ProofForge MCP (when resolvable) and prepend the ProgramV1 skill to
/// the agent prompt. Does **not** change what was already written to the doc
/// as the user message — call this only inside `drive_run` after dispatch
/// recorded the raw prompt.
pub fn enrich_sessions_run_request(mut request: RunRequest) -> RunRequest {
    let repo_root = discover_repo_root(Path::new(&request.cwd));
    let gate = StudioGate::new(GateConfig {
        paths: StudioPaths {
            repo_root: repo_root.clone(),
            cwd: Some(PathBuf::from(&request.cwd)),
            ..StudioPaths::default()
        },
        ..GateConfig::default()
    });
    let status = gate.status();
    let root = repo_root
        .or_else(|| status.repo_root.as_ref().map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from(&request.cwd));

    if request.mcp_servers.is_empty() {
        request.mcp_servers = resolve_studio_mcp_servers(&root, &status);
    }

    if !request.prompt.contains(SKILL_PROMPT_MARKER) {
        if let Some(skill) = load_program_v1_skill(&root) {
            let cli = status
                .pf_cli
                .as_deref()
                .unwrap_or("proof-forge-next");
            request.prompt = format!(
                "{SKILL_PROMPT_MARKER}\n{skill}\n\n## Local gate tools\n\n\
                 ProofForge MCP may expose `pf_check` / `pf_build` / `pf_artifacts`. \
                 Prefer those tools. CLI fallback: `{cli}`.\n\n\
                 ## User request\n\n{}",
                request.prompt
            );
        }
    }

    request
}

fn load_program_v1_skill(repo_root: &Path) -> Option<String> {
    let raw = std::fs::read_to_string(repo_root.join(AUTHOR_SKILL_REL)).ok()?;
    let body = strip_yaml_frontmatter(&raw);
    if body.contains("import ProofForgeV2") && body.contains("ProgramV1") {
        Some(body.to_string())
    } else {
        None
    }
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

fn discover_repo_root(start: &Path) -> Option<PathBuf> {
    if let Ok(root) = std::env::var("PROOFSHIP_REPO_ROOT") {
        let p = PathBuf::from(root);
        if p.join("proofship/scripts/gate.sh").is_file() {
            return Some(p);
        }
    }
    for dir in start.ancestors() {
        if dir.join("proofship/scripts/gate.sh").is_file() {
            return Some(dir.to_path_buf());
        }
    }
    // Walk from the running binary when cwd is outside the repo (common for ~).
    if let Ok(exe) = std::env::current_exe() {
        for dir in exe.ancestors() {
            if dir.join("proofship/scripts/gate.sh").is_file() {
                return Some(dir.to_path_buf());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use comet_proto::SandboxLevel;

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

    #[test]
    fn enrich_prepends_skill_once_when_repo_present() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(temp.path().join("proofship/scripts")).unwrap();
        std::fs::write(temp.path().join("proofship/scripts/gate.sh"), "#!/bin/sh\n").unwrap();
        let skill_dir = temp
            .path()
            .join(".agents/skills/proofforge-program-v1");
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: x\n---\n\n# Body\n\nimport ProofForgeV2\nProgramV1\n",
        )
        .unwrap();

        let mut req = bare_request("make an escrow");
        req.cwd = temp.path().to_string_lossy().into_owned();
        let once = enrich_sessions_run_request(req);
        assert!(once.prompt.contains(SKILL_PROMPT_MARKER));
        assert!(once.prompt.contains("make an escrow"));
        assert!(once.prompt.contains("import ProofForgeV2"));

        let twice = enrich_sessions_run_request(once.clone());
        assert_eq!(
            twice.prompt.matches(SKILL_PROMPT_MARKER).count(),
            1,
            "must not double-inject on resume"
        );
    }
}
