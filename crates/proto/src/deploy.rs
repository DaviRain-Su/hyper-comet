//! Studio deploy records + the `StudioDeploy` RPC request/events.

use serde::{Deserialize, Serialize};

/// One on-chain deployment of a gate-passing artifact set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeploymentRecord {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub launch_id: Option<String>,
    /// Studio project id (Phase 2.5); absent on older deployment files.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    /// Program / artifact module name (e.g. `RwaShareRegistry`).
    #[serde(default)]
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

/// A ProgramV1 source the Sessions Preview pane can gate-and-deploy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StudioCandidate {
    pub module: String,
    pub source: String,
    /// Filesystem path or `launch:{id}` for store-backed drafts.
    pub origin: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub digest: Option<String>,
    #[serde(default)]
    pub certified: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StudioCandidatesResponse {
    pub candidates: Vec<StudioCandidate>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StudioDeployRequest {
    pub module: String,
    pub source: String,
    pub network_id: String,
    pub wallet_id: String,
    /// Constructor signature, e.g. `constructor(uint64,uint64,uint64)`, or `-`.
    pub ctor_sig: String,
    #[serde(default)]
    pub ctor_args: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub launch_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum StudioDeployEvent {
    #[serde(rename_all = "camelCase")]
    Started { network_id: String },
    #[serde(rename_all = "camelCase")]
    Gate { ok: bool, output: String },
    #[serde(rename_all = "camelCase")]
    Sending { rpc_url: String },
    #[serde(rename_all = "camelCase")]
    Done {
        ok: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        record: Option<DeploymentRecord>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        error: Option<String>,
    },
}
