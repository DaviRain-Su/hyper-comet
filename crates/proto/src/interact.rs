//! ABI-driven contract calls from Launch Studio.

use serde::{Deserialize, Serialize};

/// Load the form schema for a gated module's `*.abi.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StudioAbiRequest {
    pub module: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StudioAbiResponse {
    pub module: String,
    /// Raw solc ABI JSON array (string) so UI can parse with `comet-abi`.
    pub abi_json: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StudioCallKind {
    View,
    Send,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StudioCallRequest {
    pub network_id: String,
    pub address: String,
    /// e.g. `issue(address,uint64)` or `totalSupply()`.
    pub signature: String,
    #[serde(default)]
    pub args: Vec<String>,
    pub kind: StudioCallKind,
    /// Required for `Send`; ignored for `View`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wallet_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StudioCallResponse {
    pub ok: bool,
    pub output: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tx_hash: Option<String>,
}

/// Fetch recent contract logs (X Layer–first interact polish).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StudioLogsRequest {
    pub network_id: String,
    pub address: String,
    /// Optional Solidity event signature, e.g. `Claimed(uint64)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_signature: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_block: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to_block: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StudioLogEntry {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub block_number: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tx_hash: Option<String>,
    #[serde(default)]
    pub topics: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StudioLogsResponse {
    pub ok: bool,
    #[serde(default)]
    pub logs: Vec<StudioLogEntry>,
    #[serde(default)]
    pub output: String,
}
