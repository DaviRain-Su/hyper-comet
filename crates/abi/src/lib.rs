//! comet-abi — Solidity JSON ABI → UI-agnostic form schema.

mod parse;
mod preview;

use serde::{Deserialize, Serialize};

pub use parse::{schema_from_abi_json, schema_from_abi_value};
pub use preview::{DappPreviewConfig, render_dapp_html};

/// Top-level form schema derived from a contract ABI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AbiFormSchema {
    pub constructor: Option<AbiFormFn>,
    pub views: Vec<AbiFormFn>,
    pub entries: Vec<AbiFormFn>,
    pub events: Vec<AbiEvent>,
}

/// A constructor, view, or entry function.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AbiFormFn {
    /// Empty string for the constructor.
    pub name: String,
    pub state_mutability: String,
    pub inputs: Vec<AbiFormParam>,
    pub outputs: Vec<AbiFormParam>,
}

impl AbiFormFn {
    /// Solidity-style signature for `cast abi-encode` / `cast call` / `cast send`.
    /// Constructor → `constructor(uint64,uint64)`; function → `issue(address,uint64)`.
    pub fn signature(&self) -> String {
        let inputs = self
            .inputs
            .iter()
            .map(|param| param.sol_type.as_str())
            .collect::<Vec<_>>()
            .join(",");
        if self.name.is_empty() {
            format!("constructor({inputs})")
        } else {
            format!("{}({inputs})", self.name)
        }
    }
}

/// A named Solidity parameter with its mapped widget.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AbiFormParam {
    pub name: String,
    pub sol_type: String,
    pub widget: AbiWidget,
}

/// UI-agnostic widget hint for a Solidity type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AbiWidget {
    Address,
    Uint { bits: u16 },
    Int { bits: u16 },
    Bool,
    Bytes { fixed: Option<u16> },
    String,
    Array { inner: Box<AbiWidget> },
    Unsupported { sol_type: String },
}

/// An event declaration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AbiEvent {
    pub name: String,
    pub inputs: Vec<AbiFormParam>,
}

/// Errors while parsing an ABI JSON payload.
#[derive(Debug, thiserror::Error)]
pub enum AbiError {
    #[error("invalid ABI JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("ABI root must be a JSON array")]
    NotArray,
}
