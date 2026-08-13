//! One-click deploy of sealed ProofForge gate artifacts.
//!
//! The agent runs the gate (skill + MCP); `pf_build` leaves `<Module>.bin`
//! (hex EVM bytecode) + optional `<Module>.abi.json` in an output dir under
//! the session cwd. `DeployScan` finds those artifact sets; `DeploySend`
//! signs a create tx with a Settings → Wallets signer on a Settings →
//! Networks chain. Records persist device-local, newest first.

use serde::{Deserialize, Serialize};

/// One gate-passing artifact set found under a session's working tree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeployArtifact {
    /// Module name, from the `.bin` file stem (e.g. `EscrowVault`).
    pub module: String,
    /// Directory holding the artifact set, relative path shown to the user.
    pub dir: String,
    /// Absolute path of the hex bytecode file.
    pub bin_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub abi_path: Option<String>,
    /// `outputSetDigest` when a gate report sits next to the bytecode.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub digest: Option<String>,
    /// Bytecode file mtime (ms since epoch) — newest artifacts sort first.
    #[serde(default)]
    pub modified_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeployScanRequest {
    /// Session working directory to scan.
    pub cwd: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeployScanResponse {
    pub artifacts: Vec<DeployArtifact>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeploySendRequest {
    /// Absolute `.bin` path from a prior `DeployScan`.
    pub bin_path: String,
    pub module: String,
    pub network_id: String,
    pub wallet_id: String,
    /// Constructor signature, e.g. `constructor(uint64,uint64)`. Empty or
    /// `-` means no constructor args.
    #[serde(default)]
    pub ctor_sig: String,
    #[serde(default)]
    pub ctor_args: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub digest: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeploySendResponse {
    pub record: DeploymentRecord,
}

/// One on-chain deployment of a gate-passing artifact set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeploymentRecord {
    pub id: String,
    pub module: String,
    pub network_id: String,
    pub address: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ctor: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub digest: Option<String>,
    pub tx_hash: String,
    pub ts: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeploymentsResponse {
    pub deployments: Vec<DeploymentRecord>,
}
