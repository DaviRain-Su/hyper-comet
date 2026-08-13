//! Local wallet address book persisted under `{data_dir}/studio/wallets.json`.
//!
//! **Private keys never appear in this file.** `env_key_name` stores only the
//! variable name for `DevEnvKey` rows. Local Alloy keys live in
//! `studio/wallet-secrets/` (mode 0600). WalletConnect session state is
//! memory-only — persisted rows are bookkeeping (label + address).

use std::path::{Path, PathBuf};

use comet_proto::{WalletAccount, WalletSource};

use super::local_wallet::{self, WalletSecrets};

#[derive(Debug, thiserror::Error)]
pub enum WalletError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid: {0}")]
    Invalid(String),
}

#[derive(Debug, Clone)]
pub struct WalletStore {
    file: PathBuf,
    secrets: WalletSecrets,
}

impl WalletStore {
    pub fn new(data_dir: &Path) -> Self {
        Self {
            file: data_dir.join("studio").join("wallets.json"),
            secrets: WalletSecrets::new(data_dir),
        }
    }

    pub fn secrets(&self) -> &WalletSecrets {
        &self.secrets
    }

    /// Create a random local signer. Returns the bookkeeping row plus a
    /// one-time hex backup (never written to `wallets.json`).
    pub fn create_local(
        &self,
        label: &str,
    ) -> Result<(Vec<WalletAccount>, WalletAccount, String), WalletError> {
        let label = label.trim();
        if label.is_empty() {
            return Err(WalletError::Invalid("label must not be empty".into()));
        }
        let (backup, address) =
            local_wallet::generate_local_key().map_err(|e| WalletError::Invalid(e.to_string()))?;
        let wallet = WalletAccount {
            id: format!("local-{}", &uuid::Uuid::new_v4().to_string()[..8]),
            label: label.to_string(),
            address,
            source: WalletSource::Local,
            env_key_name: None,
        };
        self.secrets
            .put(&wallet.id, &backup)
            .map_err(|e| WalletError::Invalid(e.to_string()))?;
        let wallets = self.upsert(wallet.clone())?;
        Ok((wallets, wallet, backup))
    }

    pub fn import_local(
        &self,
        label: &str,
        secret: &str,
    ) -> Result<(Vec<WalletAccount>, WalletAccount), WalletError> {
        let label = label.trim();
        if label.is_empty() {
            return Err(WalletError::Invalid("label must not be empty".into()));
        }
        let (backup, address) = local_wallet::import_local_key(secret)
            .map_err(|e| WalletError::Invalid(e.to_string()))?;
        if let Some(existing) = self.load()?.into_iter().find(|w| {
            w.source == WalletSource::Local && w.address.eq_ignore_ascii_case(&address)
        }) {
            self.secrets
                .put(&existing.id, &backup)
                .map_err(|e| WalletError::Invalid(e.to_string()))?;
            let mut wallet = existing;
            wallet.label = label.to_string();
            let wallets = self.upsert(wallet.clone())?;
            return Ok((wallets, wallet));
        }
        let wallet = WalletAccount {
            id: format!("local-{}", &uuid::Uuid::new_v4().to_string()[..8]),
            label: label.to_string(),
            address,
            source: WalletSource::Local,
            env_key_name: None,
        };
        self.secrets
            .put(&wallet.id, &backup)
            .map_err(|e| WalletError::Invalid(e.to_string()))?;
        let wallets = self.upsert(wallet.clone())?;
        Ok((wallets, wallet))
    }

    pub fn path(&self) -> &Path {
        &self.file
    }

    pub fn load(&self) -> Result<Vec<WalletAccount>, WalletError> {
        match std::fs::read_to_string(&self.file) {
            Ok(raw) => {
                let wallets: Vec<WalletAccount> = serde_json::from_str(&raw)?;
                Ok(wallets)
            }
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
            Err(err) => Err(err.into()),
        }
    }

    pub fn save(&self, wallets: &[WalletAccount]) -> Result<Vec<WalletAccount>, WalletError> {
        for wallet in wallets {
            validate_wallet(wallet)?;
        }
        if let Some(parent) = self.file.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let tmp = self.file.with_extension("json.tmp");
        let json = serde_json::to_vec(wallets)?;
        std::fs::write(&tmp, json)?;
        std::fs::rename(&tmp, &self.file)?;
        Ok(wallets.to_vec())
    }

    pub fn upsert(&self, wallet: WalletAccount) -> Result<Vec<WalletAccount>, WalletError> {
        validate_wallet(&wallet)?;
        let mut wallets = self.load()?;
        if let Some(ix) = wallets.iter().position(|w| w.id == wallet.id) {
            wallets[ix] = wallet;
        } else {
            wallets.push(wallet);
        }
        self.save(&wallets)
    }

    pub fn remove(&self, id: &str) -> Result<Vec<WalletAccount>, WalletError> {
        if id.trim().is_empty() {
            return Err(WalletError::Invalid("id must not be empty".into()));
        }
        self.secrets.delete(id);
        let mut wallets = self.load()?;
        wallets.retain(|w| w.id != id);
        self.save(&wallets)
    }
}

fn validate_wallet(wallet: &WalletAccount) -> Result<(), WalletError> {
    if wallet.id.trim().is_empty() {
        return Err(WalletError::Invalid("id must not be empty".into()));
    }
    if wallet.label.trim().is_empty() {
        return Err(WalletError::Invalid("label must not be empty".into()));
    }
    for field in [&wallet.id, &wallet.label] {
        if looks_like_private_key(field) {
            return Err(WalletError::Invalid(format!(
                "{field} looks like a private key"
            )));
        }
    }
    if let Some(name) = wallet.env_key_name.as_deref() {
        if looks_like_private_key(name) {
            return Err(WalletError::Invalid(
                "env_key_name looks like a private key".into(),
            ));
        }
    }
    if !wallet.address.is_empty() && looks_like_private_key_address(&wallet.address) {
        return Err(WalletError::Invalid(
            "address looks like a private key".into(),
        ));
    }

    match wallet.source {
        WalletSource::Watch => {
            if !is_eth_address(&wallet.address) {
                return Err(WalletError::Invalid(
                    "watch address must match 0x + 40 hex digits".into(),
                ));
            }
        }
        WalletSource::DevEnvKey => {
            let name = wallet
                .env_key_name
                .as_deref()
                .filter(|s| !s.trim().is_empty())
                .ok_or_else(|| WalletError::Invalid("env_key_name is required".into()))?;
            if !is_valid_env_var_name(name) {
                return Err(WalletError::Invalid(
                    "env_key_name must match [A-Za-z_][A-Za-z0-9_]*".into(),
                ));
            }
            if !wallet.address.is_empty() && !is_eth_address(&wallet.address) {
                return Err(WalletError::Invalid(
                    "address must be empty or match 0x + 40 hex digits".into(),
                ));
            }
        }
        WalletSource::WalletConnect | WalletSource::Local => {
            if !is_eth_address(&wallet.address) {
                return Err(WalletError::Invalid(
                    "address must match 0x + 40 hex digits".into(),
                ));
            }
        }
    }
    Ok(())
}

/// `0x` + exactly 40 hex digits.
fn is_eth_address(value: &str) -> bool {
    value.len() == 42
        && value.starts_with("0x")
        && value[2..].chars().all(|c| c.is_ascii_hexdigit())
}

/// 64 hex chars, or `0x` + 64 hex chars — private-key shaped.
fn looks_like_private_key(value: &str) -> bool {
    if value.len() == 64 && value.chars().all(|c| c.is_ascii_hexdigit()) {
        return true;
    }
    value.len() == 66
        && value.starts_with("0x")
        && value[2..].chars().all(|c| c.is_ascii_hexdigit())
}

/// Reject `0x` + 64 hex in the address field (key, not an address).
fn looks_like_private_key_address(value: &str) -> bool {
    looks_like_private_key(value)
}

fn is_valid_env_var_name(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn watch(label: &str, address: &str) -> WalletAccount {
        WalletAccount {
            id: "watch-1".into(),
            label: label.into(),
            address: address.into(),
            source: WalletSource::Watch,
            env_key_name: None,
        }
    }

    #[test]
    fn roundtrip_watch_address() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = WalletStore::new(dir.path());
        let wallet = watch("Treasury", "0xAbCdEf0123456789AbCdEf0123456789AbCdEf01");
        store.upsert(wallet.clone()).expect("upsert");
        let loaded = store.load().expect("load");
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0], wallet);
        assert!(store.path().exists());
    }

    #[test]
    fn roundtrip_env_key_name_does_not_persist_key_value() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = WalletStore::new(dir.path());
        let var_name = "PROOFSHIP_TEST_WALLET_ENV_KEY";
        let secret = "deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef";
        unsafe {
            std::env::set_var(var_name, secret);
        }

        let wallet = WalletAccount {
            id: "env-1".into(),
            label: "Deploy key".into(),
            address: String::new(),
            source: WalletSource::DevEnvKey,
            env_key_name: Some(var_name.into()),
        };
        store.upsert(wallet.clone()).expect("upsert");

        let raw = std::fs::read_to_string(store.path()).expect("read file");
        assert!(raw.contains(var_name));
        assert!(!raw.contains(secret));

        let loaded = store.load().expect("load");
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0], wallet);

        unsafe {
            std::env::remove_var(var_name);
        }
    }

    #[test]
    fn rejects_private_key_shaped_address() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = WalletStore::new(dir.path());
        let wallet = WalletAccount {
            id: "bad".into(),
            label: "Bad".into(),
            address: "0xdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef".into(),
            source: WalletSource::Watch,
            env_key_name: None,
        };
        let err = store.upsert(wallet).expect_err("must reject");
        assert!(matches!(err, WalletError::Invalid(_)));
    }

    #[test]
    fn remove_by_id() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = WalletStore::new(dir.path());
        store
            .upsert(watch("A", "0x1111111111111111111111111111111111111111"))
            .expect("upsert a");
        store
            .upsert(WalletAccount {
                id: "watch-2".into(),
                label: "B".into(),
                address: "0x2222222222222222222222222222222222222222".into(),
                source: WalletSource::Watch,
                env_key_name: None,
            })
            .expect("upsert b");
        let remaining = store.remove("watch-1").expect("remove");
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].id, "watch-2");
    }

    #[test]
    fn create_local_keeps_secret_out_of_address_book() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = WalletStore::new(dir.path());
        let (wallets, wallet, backup) = store.create_local("Studio").expect("create");
        assert_eq!(wallets.len(), 1);
        assert_eq!(wallet.source, WalletSource::Local);
        assert!(wallet.address.starts_with("0x"));
        assert_eq!(wallet.address.len(), 42);
        assert!(backup.starts_with("0x"));
        assert_eq!(backup.len(), 66);
        let raw = std::fs::read_to_string(store.path()).expect("read");
        assert!(!raw.contains(&backup[2..]));
        assert!(raw.contains(&wallet.address));
        store.remove(&wallet.id).expect("remove");
        assert!(store.load().unwrap().is_empty());
    }
}
