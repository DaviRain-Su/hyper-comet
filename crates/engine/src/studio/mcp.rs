//! Resolve ProofForge MCP servers for Studio ACP draft sessions.

use std::path::{Path, PathBuf};

use comet_proto::{McpEnvVar, McpServerConfig, StudioStatusResponse};

/// Default remote ProofForge MCP (catalog / remote tools). Local compile still
/// prefers stdio against the vendored CLI; web UI surfaces this URL.
pub const DEFAULT_REMOTE_PF_MCP_URL: &str =
    "https://proof-forge-mcp.davirain-yin.workers.dev/mcp";

const STUDIO_SLIM_MCP_REL: &str = "proofship/mcp/proofship_pf_mcp.py";
const FULL_PF_MCP_REL: &str = "tools/mcp/proof_forge_mcp_server.py";

/// Build the MCP server list for a Studio draft ACP session.
///
/// Order:
/// 1. `PROOFSHIP_DISABLE_PF_MCP=1` → empty
/// 2. Stdio: `PROOFSHIP_PF_MCP` override, else full PF MCP under
///    `PROOF_FORGE_ROOT`, else ProofShip slim `proofship_pf_mcp.py`
/// 3. Optional HTTP: `PROOFSHIP_PF_MCP_URL` when set
pub fn resolve_studio_mcp_servers(
    repo_root: &Path,
    status: &StudioStatusResponse,
) -> Vec<McpServerConfig> {
    if env_truthy("PROOFSHIP_DISABLE_PF_MCP") {
        return Vec::new();
    }

    let mut servers = Vec::new();
    if let Some(stdio) = resolve_stdio_mcp(repo_root, status) {
        servers.push(stdio);
    }
    if let Ok(url) = std::env::var("PROOFSHIP_PF_MCP_URL") {
        let url = url.trim().to_string();
        if !url.is_empty() {
            servers.push(McpServerConfig::http("proof-forge-http", url));
        }
    }
    servers
}

fn resolve_stdio_mcp(
    repo_root: &Path,
    status: &StudioStatusResponse,
) -> Option<McpServerConfig> {
    let python = python3_path()?;
    let (script, pf_root) = if let Ok(override_script) = std::env::var("PROOFSHIP_PF_MCP") {
        let script = PathBuf::from(override_script.trim());
        if !script.is_file() {
            return None;
        }
        (script, std::env::var_os("PROOF_FORGE_ROOT").map(PathBuf::from))
    } else if let Some((root, script)) = full_pf_mcp_script() {
        (script, Some(root))
    } else {
        let script = repo_root.join(STUDIO_SLIM_MCP_REL);
        if !script.is_file() {
            return None;
        }
        (
            script,
            std::env::var_os("PROOF_FORGE_ROOT").map(PathBuf::from),
        )
    };

    Some(McpServerConfig::stdio(
        "proof-forge",
        python.to_string_lossy(),
        vec!["-I".into(), script.to_string_lossy().into_owned()],
        mcp_env(repo_root, status, pf_root.as_deref()),
    ))
}

fn full_pf_mcp_script() -> Option<(PathBuf, PathBuf)> {
    let root = std::env::var_os("PROOF_FORGE_ROOT").map(PathBuf::from)?;
    let script = root.join(FULL_PF_MCP_REL);
    script.is_file().then(|| (root, script))
}

fn mcp_env(
    repo_root: &Path,
    status: &StudioStatusResponse,
    pf_root: Option<&Path>,
) -> Vec<McpEnvVar> {
    let mut env = Vec::new();
    if let Some(cli) = status.pf_cli.as_ref() {
        push_env(&mut env, "PROOF_FORGE_CLI", cli);
        push_env(&mut env, "PF_CLI", cli);
    }
    if let Some(tool) = status
        .proof_forge_tool_root
        .as_ref()
        .filter(|s| !s.is_empty())
    {
        push_env(&mut env, "PROOF_FORGE_TOOL_ROOT", tool);
    } else if let Ok(tool) = std::env::var("PROOF_FORGE_TOOL_ROOT") {
        push_env(&mut env, "PROOF_FORGE_TOOL_ROOT", &tool);
    }
    if let Some(root) = pf_root {
        push_env(&mut env, "PROOF_FORGE_ROOT", &root.to_string_lossy());
    } else if let Ok(root) = std::env::var("PROOF_FORGE_ROOT") {
        push_env(&mut env, "PROOF_FORGE_ROOT", &root);
    }
    let inbox = repo_root.join("proofship/inbox");
    push_env(
        &mut env,
        "PROOFSHIP_PROJECT_ROOT",
        &inbox.to_string_lossy(),
    );
    if let Some(elan) = status.elan_toolchain.as_ref() {
        push_env(&mut env, "ELAN_TOOLCHAIN", elan);
    } else if let Ok(elan) = std::env::var("ELAN_TOOLCHAIN") {
        push_env(&mut env, "ELAN_TOOLCHAIN", &elan);
    }
    env
}

fn push_env(env: &mut Vec<McpEnvVar>, name: &str, value: &str) {
    env.push(McpEnvVar {
        name: name.into(),
        value: value.into(),
    });
}

fn python3_path() -> Option<PathBuf> {
    for cand in ["/usr/bin/python3", "/bin/python3"] {
        let p = PathBuf::from(cand);
        if p.is_file() {
            return Some(p);
        }
    }
    which("python3")
}

fn which(exe: &str) -> Option<PathBuf> {
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
    use super::*;

    fn status_with_cli(cli: &str) -> StudioStatusResponse {
        StudioStatusResponse {
            repo_root: None,
            pf_cli: Some(cli.into()),
            cli_resolved: true,
            elan_toolchain: None,
            proof_forge_tool_root: None,
            toolchain_ok: true,
            error: None,
        }
    }

    #[test]
    fn mcp_env_pins_cli_and_inbox() {
        let temp = tempfile::tempdir().unwrap();
        let env = mcp_env(temp.path(), &status_with_cli("/opt/pf/proof-forge-next"), None);
        let map: std::collections::HashMap<_, _> =
            env.into_iter().map(|e| (e.name, e.value)).collect();
        assert_eq!(
            map.get("PROOF_FORGE_CLI").map(String::as_str),
            Some("/opt/pf/proof-forge-next")
        );
        assert!(
            map.get("PROOFSHIP_PROJECT_ROOT")
                .unwrap()
                .ends_with("proofship/inbox")
        );
    }

    #[test]
    fn http_config_serializes_with_type() {
        let cfg = McpServerConfig::http("proof-forge-http", DEFAULT_REMOTE_PF_MCP_URL);
        let v = serde_json::to_value(&cfg).unwrap();
        assert_eq!(v["type"], "http");
        assert_eq!(v["url"], DEFAULT_REMOTE_PF_MCP_URL);
    }

    #[test]
    fn stdio_config_omits_type_field() {
        let cfg = McpServerConfig::stdio(
            "proof-forge",
            "/usr/bin/python3",
            vec!["-I".into(), "/tmp/x.py".into()],
            vec![],
        );
        let v = serde_json::to_value(&cfg).unwrap();
        assert!(v.get("type").is_none());
        assert_eq!(v["command"], "/usr/bin/python3");
        assert_eq!(v["name"], "proof-forge");
    }

    #[test]
    fn resolve_finds_slim_script_when_present() {
        let temp = tempfile::tempdir().unwrap();
        let script = temp.path().join(STUDIO_SLIM_MCP_REL);
        std::fs::create_dir_all(script.parent().unwrap()).unwrap();
        std::fs::write(&script, "print('ok')\n").unwrap();
        // Disable HTTP / overrides without touching process env: call resolve_stdio only.
        let status = status_with_cli("/tmp/proof-forge-next");
        let Some(McpServerConfig::Stdio { name, args, env, .. }) =
            resolve_stdio_mcp(temp.path(), &status)
        else {
            // python3 missing in weird CI — skip soft
            if python3_path().is_none() {
                return;
            }
            panic!("expected stdio mcp");
        };
        assert_eq!(name, "proof-forge");
        assert!(args.iter().any(|a| a.ends_with("proofship_pf_mcp.py")));
        assert!(env.iter().any(|e| e.name == "PROOF_FORGE_CLI"));
    }
}
