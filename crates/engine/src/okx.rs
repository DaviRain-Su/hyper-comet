//! OKX OnchainOS integration.
//!
//! OKX ships its Web3 dev surface (DEX aggregator quotes, liquidity,
//! ERC-20 approve calldata, swap construction, Solana swap instructions)
//! as a hosted MCP server. One API key from the OnchainOS dev portal
//! unlocks it; the engine then attaches the server to every agent session
//! automatically — no per-project integration code.
//!
//! Key resolution: `OKX_ONCHAINOS_API_KEY` / `OK_ACCESS_KEY` env override
//! first (dev convenience), then the key stored from Settings → Networks
//! (`{data_dir}/studio/okx.json`, mode 0600 — never synced, never sent
//! anywhere except as the `OK-ACCESS-KEY` header to OKX).
//!
//! Pluggable by design: the Settings toggle (`enabled`) turns the whole
//! integration off without deleting the key — the multi-chain story means
//! any single ecosystem hookup must be switchable. A disabled toggle wins
//! over an env key too. Env opt-out: `PROOFSHIP_DISABLE_OKX_MCP=1`.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use comet_proto::{McpHttpHeader, McpServerConfig, OkxStatusResponse, RunRequest};
use serde::{Deserialize, Serialize};

/// Hosted OnchainOS MCP endpoint (docs: web3.okx.com/onchainos/dev-docs).
pub const ONCHAINOS_MCP_URL: &str = "https://web3.okx.com/api/v1/onchainos-mcp";
/// Server name in the session's MCP list (also the dedupe key).
pub const ONCHAINOS_MCP_NAME: &str = "okx-onchainos";
/// Auth header the OnchainOS gateway expects.
const ACCESS_KEY_HEADER: &str = "OK-ACCESS-KEY";

#[cfg(unix)]
const SECRET_MODE: u32 = 0o600;

/// On-disk shape of `studio/okx.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OkxConfig {
    #[serde(default)]
    api_key: String,
    #[serde(default = "default_true")]
    enabled: bool,
}

fn default_true() -> bool {
    true
}

impl Default for OkxConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            enabled: true,
        }
    }
}

/// File-backed store for the OnchainOS API key + enable toggle.
#[derive(Debug, Clone)]
pub struct OkxStore {
    file: PathBuf,
}

impl OkxStore {
    pub fn new(data_dir: &Path) -> Self {
        Self {
            file: data_dir.join("studio").join("okx.json"),
        }
    }

    fn read(&self) -> OkxConfig {
        std::fs::read_to_string(&self.file)
            .ok()
            .and_then(|raw| serde_json::from_str(&raw).ok())
            .unwrap_or_default()
    }

    fn write(&self, config: &OkxConfig) -> Result<(), std::io::Error> {
        if let Some(parent) = self.file.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let tmp = self.file.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_vec_pretty(config)?)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(SECRET_MODE))?;
        }
        std::fs::rename(&tmp, &self.file)?;
        Ok(())
    }

    pub fn get(&self) -> Option<String> {
        let config = self.read();
        let key = config.api_key.trim().to_string();
        (!key.is_empty()).then_some(key)
    }

    pub fn enabled(&self) -> bool {
        self.read().enabled
    }

    /// Store a key (re-enables the integration) or clear it when empty.
    pub fn put(&self, api_key: &str) -> Result<(), std::io::Error> {
        let key = api_key.trim();
        if key.is_empty() {
            let _ = std::fs::remove_file(&self.file);
            return Ok(());
        }
        self.write(&OkxConfig {
            api_key: key.to_string(),
            enabled: true,
        })
    }

    /// Flip the pluggable toggle without touching the stored key.
    pub fn set_enabled(&self, enabled: bool) -> Result<(), std::io::Error> {
        let mut config = self.read();
        config.enabled = enabled;
        self.write(&config)
    }

    pub fn status(&self) -> OkxStatusResponse {
        let enabled = self.enabled();
        match access_key_from(Some(self)) {
            Some((key, source)) => OkxStatusResponse {
                configured: true,
                enabled,
                source: Some(source.into()),
                key_hint: mask_key(&key),
            },
            None => OkxStatusResponse {
                enabled,
                ..OkxStatusResponse::default()
            },
        }
    }
}

/// Set once at engine assembly so session enrichment (which has no data-dir
/// plumbing) can read the stored key. Env overrides still work without it.
static STORE: OnceLock<OkxStore> = OnceLock::new();

pub fn init(data_dir: &Path) {
    let _ = STORE.set(OkxStore::new(data_dir));
}

/// The effective OnchainOS key and its origin (`env` beats `stored`).
fn access_key_from(store: Option<&OkxStore>) -> Option<(String, &'static str)> {
    for var in ["OKX_ONCHAINOS_API_KEY", "OK_ACCESS_KEY"] {
        if let Ok(value) = std::env::var(var) {
            let value = value.trim().to_string();
            if !value.is_empty() {
                return Some((value, "env"));
            }
        }
    }
    let key = store.or_else(|| STORE.get()).and_then(OkxStore::get)?;
    Some((key, "stored"))
}

pub fn mask_key(key: &str) -> String {
    let chars: Vec<char> = key.chars().collect();
    if chars.len() <= 8 {
        return "····".into();
    }
    let head: String = chars[..4].iter().collect();
    let tail: String = chars[chars.len() - 4..].iter().collect();
    format!("{head}…{tail}")
}

/// Append the OnchainOS MCP server to the run when a key is configured.
/// Runs after the ProofForge enrichment — appends, never replaces, and
/// dedupes by server name so resume/retry stays idempotent.
pub fn enrich_run_request(mut request: RunRequest) -> RunRequest {
    if env_truthy("PROOFSHIP_DISABLE_OKX_MCP") {
        return request;
    }
    // The Settings toggle wins over everything, env keys included — "off"
    // must mean off for the whole integration.
    if STORE.get().is_some_and(|store| !store.enabled()) {
        return request;
    }
    if request.mcp_servers.iter().any(|s| server_name(s) == ONCHAINOS_MCP_NAME) {
        return request;
    }
    let Some((key, _)) = access_key_from(None) else {
        return request;
    };
    request.mcp_servers.push(McpServerConfig::Http {
        transport: "http".into(),
        name: ONCHAINOS_MCP_NAME.into(),
        url: ONCHAINOS_MCP_URL.into(),
        headers: vec![McpHttpHeader {
            name: ACCESS_KEY_HEADER.into(),
            value: key,
        }],
    });
    request
}

fn server_name(server: &McpServerConfig) -> &str {
    match server {
        McpServerConfig::Http { name, .. } => name,
        McpServerConfig::Stdio { name, .. } => name,
    }
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

    #[test]
    fn store_round_trip_and_mask() {
        let dir = tempfile::tempdir().unwrap();
        let store = OkxStore::new(dir.path());
        assert!(store.get().is_none());
        assert!(!store.status().configured);
        assert!(store.enabled(), "default is enabled");

        store.put("  abcd1234efgh5678  ").unwrap();
        assert_eq!(store.get().as_deref(), Some("abcd1234efgh5678"));
        let status = store.status();
        assert!(status.configured);
        assert!(status.enabled);
        assert_eq!(status.key_hint, "abcd…5678");

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(dir.path().join("studio/okx.json"))
                .unwrap()
                .permissions()
                .mode();
            assert_eq!(mode & 0o777, 0o600, "key file must be private");
        }

        store.put("").unwrap();
        assert!(store.get().is_none(), "empty key clears the store");
    }

    #[test]
    fn toggle_survives_and_key_reenables() {
        let dir = tempfile::tempdir().unwrap();
        let store = OkxStore::new(dir.path());
        store.put("abcd1234efgh5678").unwrap();

        store.set_enabled(false).unwrap();
        let status = store.status();
        assert!(status.configured, "key survives the off toggle");
        assert!(!status.enabled);
        assert_eq!(store.get().as_deref(), Some("abcd1234efgh5678"));

        store.set_enabled(true).unwrap();
        assert!(store.status().enabled);

        // Storing a fresh key always re-enables (Save = "I want this on").
        store.set_enabled(false).unwrap();
        store.put("new-key-9876abcd").unwrap();
        assert!(store.status().enabled);
    }

    #[test]
    fn short_keys_are_fully_masked() {
        assert_eq!(mask_key("abc"), "····");
        assert_eq!(mask_key("abcd1234x"), "abcd…234x");
    }

    #[test]
    fn enrich_dedupes_by_name() {
        let request = RunRequest {
            prompt: "hi".into(),
            harness: None,
            model: None,
            reasoning: None,
            model_options: Default::default(),
            cwd: "/tmp".into(),
            sandbox: comet_proto::SandboxLevel::WorkspaceWrite,
            auto_approve: true,
            resume: None,
            attachments: Vec::new(),
            mcp_servers: vec![McpServerConfig::http(ONCHAINOS_MCP_NAME, "https://x.test")],
        };
        let out = enrich_run_request(request);
        assert_eq!(out.mcp_servers.len(), 1, "must not double-attach");
    }
}
