//! In-app Alloy signer for ProofShip.
//!
//! Metadata lives in `wallets.json`. The hex key lives next to it under
//! `studio/wallet-secrets/{id}` with mode 0600. The secret is never written
//! into the address book.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use alloy::network::{EthereumWallet, TransactionBuilder};
use alloy::primitives::{Address, Bytes};
use alloy::providers::{Provider, ProviderBuilder};
use alloy::rpc::types::TransactionRequest;
use alloy::signers::local::PrivateKeySigner;

const SECRET_MODE: u32 = 0o600;

#[derive(Debug, thiserror::Error)]
pub enum LocalWalletError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Invalid(String),
}

#[derive(Debug, Clone)]
pub struct WalletSecrets {
    dir: PathBuf,
}

impl WalletSecrets {
    pub fn new(data_dir: &Path) -> Self {
        Self {
            dir: data_dir.join("studio").join("wallet-secrets"),
        }
    }

    pub fn path_for(&self, id: &str) -> PathBuf {
        self.dir.join(id)
    }

    pub fn put(&self, id: &str, hex_key: &str) -> Result<(), LocalWalletError> {
        if id.trim().is_empty() || id.contains('/') || id.contains('\\') {
            return Err(LocalWalletError::Invalid("invalid wallet id".into()));
        }
        let hex_key = normalize_hex_key(hex_key)?;
        std::fs::create_dir_all(&self.dir)?;
        #[cfg(unix)]
        {
            let mut perms = std::fs::metadata(&self.dir)
                .map(|m| m.permissions())
                .unwrap_or_else(|_| std::fs::Permissions::from_mode(0o700));
            perms.set_mode(0o700);
            let _ = std::fs::set_permissions(&self.dir, perms);
        }
        let path = self.path_for(id);
        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, hex_key.as_bytes())?;
        #[cfg(unix)]
        {
            let mut perms = std::fs::Permissions::from_mode(SECRET_MODE);
            perms.set_mode(SECRET_MODE);
            std::fs::set_permissions(&tmp, perms)?;
        }
        std::fs::rename(&tmp, &path)?;
        Ok(())
    }

    pub fn get(&self, id: &str) -> Result<String, LocalWalletError> {
        let raw = std::fs::read_to_string(self.path_for(id)).map_err(|err| {
            if err.kind() == std::io::ErrorKind::NotFound {
                LocalWalletError::Invalid(format!(
                    "local wallet {id} has no stored key — recreate or import it"
                ))
            } else {
                LocalWalletError::Io(err)
            }
        })?;
        normalize_hex_key(raw.trim())
    }

    pub fn delete(&self, id: &str) {
        let _ = std::fs::remove_file(self.path_for(id));
    }
}

pub fn generate_local_key() -> Result<(String, String), LocalWalletError> {
    let signer = PrivateKeySigner::random();
    let backup = format!("0x{}", alloy::hex::encode(signer.to_bytes()));
    Ok((backup, format_address(signer.address())))
}

pub fn import_local_key(secret: &str) -> Result<(String, String), LocalWalletError> {
    let signer = parse_signer(secret)?;
    let backup = format!("0x{}", alloy::hex::encode(signer.to_bytes()));
    Ok((backup, format_address(signer.address())))
}

pub fn parse_signer(secret: &str) -> Result<PrivateKeySigner, LocalWalletError> {
    let secret = secret.trim();
    if secret.is_empty() {
        return Err(LocalWalletError::Invalid("secret is empty".into()));
    }
    secret.parse::<PrivateKeySigner>().map_err(|err| {
        LocalWalletError::Invalid(format!(
            "could not parse private key (hex 0x…64). {err}"
        ))
    })
}

pub fn signer_from_hex(hex_key: &str) -> Result<PrivateKeySigner, LocalWalletError> {
    parse_signer(hex_key)
}

pub struct SendOutcome {
    pub tx_hash: String,
    pub contract_address: Option<String>,
}

/// Sign and broadcast a tx with the stored local key (create if `to` is None).
pub async fn send_with_local(
    secrets: &WalletSecrets,
    wallet_id: &str,
    rpc_url: &str,
    chain_id: u64,
    to: Option<&str>,
    data: &str,
) -> Result<SendOutcome, String> {
    let hex_key = secrets.get(wallet_id).map_err(|e| e.to_string())?;
    send_with_key(&hex_key, rpc_url, chain_id, to, data).await
}

/// Sign and broadcast with a caller-held hex key (DevEnvKey wallets — the key
/// value lives in an env var, never in app storage).
pub async fn send_with_key(
    hex_key: &str,
    rpc_url: &str,
    chain_id: u64,
    to: Option<&str>,
    data: &str,
) -> Result<SendOutcome, String> {
    let signer = signer_from_hex(hex_key).map_err(|e| e.to_string())?;
    let wallet = EthereumWallet::from(signer);
    let url: reqwest::Url = rpc_url
        .parse()
        .map_err(|e| format!("invalid rpc url: {e}"))?;
    let provider = ProviderBuilder::new().wallet(wallet).connect_http(url);

    let input: Bytes = data
        .parse()
        .map_err(|e| format!("invalid tx data: {e}"))?;
    let mut tx = TransactionRequest::default()
        .with_chain_id(chain_id)
        .with_input(input);
    if let Some(to) = to {
        let addr: Address = to
            .parse()
            .map_err(|e| format!("invalid to address: {e}"))?;
        tx = tx.with_to(addr);
    }

    let pending = provider
        .send_transaction(tx)
        .await
        .map_err(|e| format!("send transaction: {e}"))?;
    let tx_hash = format!("{:#x}", pending.tx_hash());
    let receipt = pending
        .get_receipt()
        .await
        .map_err(|e| format!("wait for receipt: {e}"))?;
    let contract_address = receipt.contract_address.map(|a| format!("{a:#x}"));
    Ok(SendOutcome {
        tx_hash,
        contract_address,
    })
}

fn format_address(address: Address) -> String {
    format!("{address:#x}")
}

fn normalize_hex_key(value: &str) -> Result<String, LocalWalletError> {
    let value = value.trim();
    let hex = value.strip_prefix("0x").unwrap_or(value);
    if hex.len() != 64 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(LocalWalletError::Invalid(
            "private key must be 32 bytes hex".into(),
        ));
    }
    Ok(format!("0x{}", hex.to_ascii_lowercase()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_and_reload_same_address() {
        let dir = tempfile::tempdir().unwrap();
        let secrets = WalletSecrets::new(dir.path());
        let (backup, address) = generate_local_key().unwrap();
        secrets.put("w1", &backup).unwrap();
        let loaded = secrets.get("w1").unwrap();
        let signer = signer_from_hex(&loaded).unwrap();
        assert_eq!(format_address(signer.address()), address);
        assert!(!std::fs::read_to_string(secrets.path_for("w1"))
            .unwrap()
            .contains(&address[2..]));
    }

    #[test]
    fn import_hex_roundtrip() {
        let (backup, address) = generate_local_key().unwrap();
        let (again, imported) = import_local_key(&backup).unwrap();
        assert_eq!(imported, address);
        assert_eq!(again, backup);
    }

    #[test]
    fn rejects_empty_and_short_keys() {
        assert!(import_local_key("").is_err());
        assert!(import_local_key("0x1234").is_err());
        assert!(normalize_hex_key("not-a-key").is_err());
    }

    #[test]
    fn secret_file_is_owner_rw_only() {
        let dir = tempfile::tempdir().unwrap();
        let secrets = WalletSecrets::new(dir.path());
        let (backup, _) = generate_local_key().unwrap();
        secrets.put("w1", &backup).unwrap();
        let mode = std::fs::metadata(secrets.path_for("w1"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o600);
    }
}
