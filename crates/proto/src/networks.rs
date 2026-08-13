//! Device-local EVM network presets + custom entries.
//!
//! Persisted as `{data_dir}/studio/networks.json`. Not synced — RPC URLs and
//! explorer hosts are machine-local operator config.

use serde::{Deserialize, Serialize};

/// One EVM network the operator can deploy to / call.
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
    /// Pluggable multi-chain: disabled networks stay configured but are
    /// hidden from deploy pickers and rejected by preflight. Built-ins can
    /// be disabled, never deleted.
    #[serde(default = "default_true")]
    pub enabled: bool,
}

pub(crate) fn default_true() -> bool {
    true
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

/// Built-in EVM presets. **X Layer first** (product priority); other testnets
/// are available for multi-EVM work without changing the default story.
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
            enabled: true,
        },
        EvmNetwork {
            id: "xlayer-mainnet".into(),
            name: "X Layer".into(),
            chain_id: 196,
            rpc_url: "https://rpc.xlayer.tech".into(),
            explorer_url: Some("https://www.okx.com/web3/explorer/xlayer".into()),
            currency_symbol: "OKB".into(),
            builtin: true,
            enabled: true,
        },
        EvmNetwork {
            id: "ethereum-sepolia".into(),
            name: "Ethereum Sepolia".into(),
            chain_id: 11155111,
            rpc_url: "https://ethereum-sepolia-rpc.publicnode.com".into(),
            explorer_url: Some("https://sepolia.etherscan.io".into()),
            currency_symbol: "ETH".into(),
            builtin: true,
            enabled: true,
        },
        EvmNetwork {
            id: "base-sepolia".into(),
            name: "Base Sepolia".into(),
            chain_id: 84532,
            rpc_url: "https://sepolia.base.org".into(),
            explorer_url: Some("https://sepolia.basescan.org".into()),
            currency_symbol: "ETH".into(),
            builtin: true,
            enabled: true,
        },
    ]
}

/// Default network id when the operator has not picked one.
pub fn default_network_id() -> &'static str {
    "xlayer-testnet"
}
