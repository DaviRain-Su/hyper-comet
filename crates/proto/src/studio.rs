//! ProofShip Launch Studio wire types.
//!
//! Plain serde shapes (`camelCase`) for launch threads plus stream-friendly
//! gate events for the native engine pipeline. Vertical-agnostic: a draft's
//! structured summary is an opaque key→value table the drafting agent fills
//! (field names vary per contract family).

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::HarnessId;

/// A draft's structured field summary (e.g. `totalSupply` → `1000000`);
/// keys are whatever the drafting agent/template reports.
pub type StudioDraftFields = BTreeMap<String, String>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StudioGateStage {
    Check,
    Build,
    Inspect,
    Done,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StudioGateRequest {
    pub module: String,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StudioDraftRequest {
    pub nl: String,
    pub harness: HarnessId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StudioLaunchRunRequest {
    pub nl: String,
    pub harness: HarnessId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StudioLaunchRunPhase {
    Draft,
    Gate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum StudioDraftEvent {
    #[serde(rename_all = "camelCase")]
    Started { lane: HarnessId },
    #[serde(rename_all = "camelCase")]
    Note { text: String },
    #[serde(rename_all = "camelCase")]
    Done {
        ok: bool,
        lane: HarnessId,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        module: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        source: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        error: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StudioGateArtifact {
    pub name: String,
    pub size: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct StudioGateDigest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_set_digest: Option<String>,
    #[serde(default)]
    pub raw: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum StudioGateEvent {
    #[serde(rename_all = "camelCase")]
    Started { stage: StudioGateStage },
    #[serde(rename_all = "camelCase")]
    StageDone {
        stage: StudioGateStage,
        ok: bool,
        output: String,
    },
    #[serde(rename_all = "camelCase")]
    Done {
        ok: bool,
        stage: StudioGateStage,
        #[serde(default)]
        artifacts: Vec<StudioGateArtifact>,
        #[serde(default)]
        digest: StudioGateDigest,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum StudioLaunchRunEvent {
    #[serde(rename_all = "camelCase")]
    Draft {
        round: u32,
        phase: StudioLaunchRunPhase,
        event: StudioDraftEvent,
    },
    #[serde(rename_all = "camelCase")]
    Gate {
        round: u32,
        phase: StudioLaunchRunPhase,
        event: StudioGateEvent,
    },
    #[serde(rename_all = "camelCase")]
    Done {
        ok: bool,
        round: u32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        module: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        source: Option<String>,
        #[serde(default)]
        artifacts: Vec<StudioGateArtifact>,
        #[serde(default)]
        digest: StudioGateDigest,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        last_diagnostics: Option<String>,
        exhausted: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StudioStatusResponse {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repo_root: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pf_cli: Option<String>,
    pub cli_resolved: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub elan_toolchain: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proof_forge_tool_root: Option<String>,
    pub toolchain_ok: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum StudioChatMsg {
    User(StudioUserMsg),
    AgentDraft(StudioDraftMsg),
    AgentGate(StudioGateMsg),
    AgentNote(StudioNoteMsg),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StudioUserMsg {
    pub role: String,
    pub text: String,
    pub at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StudioDraftMsg {
    pub role: String,
    pub kind: String,
    #[serde(default)]
    pub fields: StudioDraftFields,
    pub program: String,
    pub source: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    pub at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StudioGateMsg {
    pub role: String,
    pub kind: String,
    pub state: StudioGateState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<StudioGateRunResult>,
    pub at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StudioNoteMsg {
    pub role: String,
    pub kind: String,
    pub text: String,
    pub at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StudioGateState {
    Running,
    Pass,
    Fail,
    Offline,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StudioGateRunResult {
    pub ok: bool,
    pub stage: StudioGateStage,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub check: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub build: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inspect: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StudioLaunch {
    pub id: String,
    pub title: String,
    pub created_at: String,
    #[serde(default)]
    pub msgs: Vec<StudioChatMsg>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fields: Option<StudioDraftFields>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub program: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

impl StudioLaunch {
    pub fn new_now() -> Self {
        let id = uuid::Uuid::new_v4().to_string();
        Self {
            id,
            title: "New launch".into(),
            created_at: Utc::now().to_rfc3339(),
            msgs: Vec::new(),
            fields: None,
            program: None,
            source: None,
        }
    }

    pub fn created_at_datetime(&self) -> Option<DateTime<Utc>> {
        DateTime::parse_from_rfc3339(&self.created_at)
            .ok()
            .map(|dt| dt.with_timezone(&Utc))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StudioLaunchesResponse {
    pub launches: Vec<StudioLaunch>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StudioPutLaunchesRequest {
    pub launches: Vec<StudioLaunch>,
}
