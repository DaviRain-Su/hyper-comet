//! WalletConnect connect-bridge RPCs.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WalletConnectStartRequest {
    #[serde(default)]
    pub label: String,
    /// Reown / WalletConnect Cloud project id. Falls back to
    /// `PROOFSHIP_WC_PROJECT_ID` / `REOWN_PROJECT_ID` when omitted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WalletConnectStartResponse {
    pub url: String,
    pub label: String,
}
