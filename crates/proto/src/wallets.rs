//! Device-local wallet address book.
//!
//! Persisted as `{data_dir}/studio/wallets.json`. **Private keys never appear
//! in this file or any other app storage.** WalletConnect sessions live in
//! memory only; the address book records the resulting address + label.

use serde::{Deserialize, Serialize};

/// How an address-book row was added.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WalletSource {
    /// WalletConnect / Reown session (desktop QR / deeplink). Session itself
    /// is memory-only; this row is the address that session unlocked.
    WalletConnect,
    /// Watch-only address. Can drive `eth_call`; cannot sign.
    Watch,
    /// Reference to an env var *name* holding a hex key. Testnet-only by
    /// product policy; the key value is never persisted.
    DevEnvKey,
}

/// One signer / watch address the operator can pick at deploy time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WalletAccount {
    pub id: String,
    pub label: String,
    /// `0x`-prefixed address. Empty until a WalletConnect session reports one.
    #[serde(default)]
    pub address: String,
    pub source: WalletSource,
    /// Env var *name* (not value) when `source == DevEnvKey`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub env_key_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WalletsResponse {
    pub wallets: Vec<WalletAccount>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PutWalletsRequest {
    pub wallets: Vec<WalletAccount>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpsertWalletRequest {
    pub wallet: WalletAccount,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoveWalletRequest {
    pub id: String,
}
