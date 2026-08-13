//! OKX OnchainOS integration wire types.
//!
//! One credential — the OnchainOS API key from
//! <https://web3.okx.com/onchainos/dev-portal/project> — unlocks the hosted
//! OnchainOS MCP server (DEX quotes / liquidity / approve + swap calldata).
//! The engine attaches that server to every agent session automatically.

use serde::{Deserialize, Serialize};

/// `OkxStatus` response: whether the OnchainOS key is configured and where
/// it came from. The key itself never crosses the wire — only a hint.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OkxStatusResponse {
    pub configured: bool,
    /// The pluggable on/off switch: a stored key with `enabled = false`
    /// stays on disk but the MCP server is not attached to sessions.
    #[serde(default)]
    pub enabled: bool,
    /// `"env"` (OKX_ONCHAINOS_API_KEY / OK_ACCESS_KEY) or `"stored"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// Masked key for display, e.g. `ab12…f9`.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub key_hint: String,
}

/// `OkxPutKey` request: store (or clear, when empty) the OnchainOS API key.
/// Storing a key re-enables the integration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OkxPutKeyRequest {
    #[serde(default)]
    pub api_key: String,
}

/// `OkxSetEnabled` request: flip the integration on/off without touching
/// the stored key.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OkxSetEnabledRequest {
    pub enabled: bool,
}
