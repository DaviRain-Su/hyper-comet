//! Studio vertical templates (data-driven).

use serde::{Deserialize, Serialize};

/// One Launch Studio template (manifest + optional source).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StudioTemplate {
    pub id: String,
    pub name: String,
    pub description: String,
    pub module: String,
    /// Preferred deploy network id (X Layer first by product policy).
    pub preferred_network_id: String,
    /// NL seed dropped into the Studio composer.
    pub nl_seed: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ctor_sig: Option<String>,
    #[serde(default)]
    pub ctor_hints: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    /// Design system id (see `proofship/templates/_design/`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub design: Option<String>,
    /// ProgramV1 source when the template ships a golden file.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// Optional ABI JSON string for Preview demos.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub abi_json: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StudioTemplatesResponse {
    pub templates: Vec<StudioTemplate>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StudioTemplateRequest {
    pub id: String,
}
