//! Device-local EVM network presets + custom entries.
//!
//! Persisted as `{data_dir}/studio/networks.json`. Not synced — RPC URLs and
//! explorer hosts are machine-local operator config.

use serde::{Deserialize, Serialize};

/// One EVM network the Studio can deploy to / call.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvmNetwork {
    pub id: String,
    pub name: String,
    pub chain_id: u64,
    pub rpc_url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub explorer_url: Option<String>,
    pub currency_symbol: String,
    /// Built-in presets cannot be deleted; the operator may still edit RPC URL.
    #[serde(default)]
    pub builtin: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworksResponse {
    pub networks: Vec<EvmNetwork>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PutNetworksRequest {
    pub networks: Vec<EvmNetwork>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpsertNetworkRequest {
    pub network: EvmNetwork,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoveNetworkRequest {
    pub id: String,
}

/// X Layer testnet (chain 1952) + mainnet (chain 196).
pub fn builtin_networks() -> Vec<EvmNetwork> {
    vec![
        EvmNetwork {
            id: "xlayer-testnet".into(),
            name: "X Layer Testnet".into(),
            chain_id: 1952,
            rpc_url: "https://testrpc.xlayer.tech/terigon".into(),
            explorer_url: Some("https://www.okx.com/web3/explorer/xlayer-test".into()),
            currency_symbol: "OKB".into(),
            builtin: true,
        },
        EvmNetwork {
            id: "xlayer-mainnet".into(),
            name: "X Layer".into(),
            chain_id: 196,
            rpc_url: "https://rpc.xlayer.tech".into(),
            explorer_url: Some("https://www.okx.com/web3/explorer/xlayer".into()),
            currency_symbol: "OKB".into(),
            builtin: true,
        },
    ]
}
