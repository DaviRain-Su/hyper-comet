//! One-click deploy of sealed ProofForge gate artifacts.
//!
//! The gate (skill + MCP in Sessions) leaves `<Module>.bin` hex bytecode —
//! plus optional `<Module>.abi.json` and a gate report with the
//! `outputSetDigest` — in an output directory under the session cwd.
//! [`scan_artifacts`] finds those sets; [`deploy`] ABI-encodes constructor
//! args natively (alloy dyn-abi — no foundry install needed), signs a create
//! tx with a Settings → Wallets signer, and appends a [`DeploymentRecord`]
//! to `{data_dir}/studio/deployments.json`.
//!
//! Only artifacts are deployable — there is no source path here by design:
//! no gate pass, no `.bin`, nothing to deploy.

use std::path::{Path, PathBuf};

use alloy::dyn_abi::{DynSolType, DynSolValue};
use chrono::Utc;
use comet_proto::{
    DeployArtifact, DeploySendRequest, DeploymentRecord, EvmNetwork, WalletAccount, WalletSource,
};
use uuid::Uuid;

use crate::local_wallet::{WalletSecrets, send_with_key, send_with_local};
use crate::walletconnect::{WalletConnectBridge, wait_contract_address};

const MAX_DEPLOYMENTS: usize = 100;
const SCAN_MAX_DEPTH: usize = 4;
const SCAN_MAX_DIRS: usize = 2000;
/// Bytecode sanity cap — EVM init code tops out well below this.
const MAX_BIN_BYTES: u64 = 4 * 1024 * 1024;

/// Known production / mainnet chain ids. DevEnvKey signing is testnet-only.
const MAINNET_CHAIN_IDS: &[u64] = &[
    1,     // Ethereum
    10,    // Optimism
    56,    // BNB Smart Chain
    137,   // Polygon
    196,   // X Layer
    8453,  // Base
    42161, // Arbitrum One
    43114, // Avalanche C-Chain
];

#[derive(Debug, thiserror::Error)]
pub enum DeployStoreError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
}

/// Device-local deployment history, newest first, capped at 100.
#[derive(Debug, Clone)]
pub struct DeployStore {
    file: PathBuf,
}

impl DeployStore {
    pub fn new(data_dir: &Path) -> Self {
        Self {
            file: data_dir.join("studio").join("deployments.json"),
        }
    }

    pub fn load(&self) -> Result<Vec<DeploymentRecord>, DeployStoreError> {
        match std::fs::read_to_string(&self.file) {
            Ok(raw) => {
                let mut deployments: Vec<DeploymentRecord> = serde_json::from_str(&raw)?;
                deployments.truncate(MAX_DEPLOYMENTS);
                Ok(deployments)
            }
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
            Err(err) => Err(err.into()),
        }
    }

    /// Prepend `record`, cap, atomically persist, return newest-first list.
    pub fn append(
        &self,
        record: DeploymentRecord,
    ) -> Result<Vec<DeploymentRecord>, DeployStoreError> {
        let mut deployments = self.load()?;
        deployments.insert(0, record);
        deployments.truncate(MAX_DEPLOYMENTS);
        if let Some(parent) = self.file.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let tmp = self.file.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_vec(&deployments)?)?;
        std::fs::rename(&tmp, &self.file)?;
        Ok(deployments)
    }
}

// ---------------------------------------------------------------------------
// Artifact scan
// ---------------------------------------------------------------------------

/// Find gate artifact sets under `cwd`: any `<Module>.bin` file whose stem is
/// a valid module identifier and whose content is hex bytecode. Bounded walk
/// (depth 4, 2000 dirs), skipping VCS/build trees. Newest first.
pub fn scan_artifacts(cwd: &Path) -> Vec<DeployArtifact> {
    let mut artifacts = Vec::new();
    let mut stack = vec![(cwd.to_path_buf(), 0usize)];
    let mut visited = 0usize;
    while let Some((dir, depth)) = stack.pop() {
        visited += 1;
        if visited > SCAN_MAX_DIRS {
            break;
        }
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            let Ok(meta) = entry.metadata() else { continue };
            if meta.is_dir() {
                if depth < SCAN_MAX_DEPTH && !skip_dir(&name) {
                    stack.push((path, depth + 1));
                }
                continue;
            }
            if !meta.is_file() || path.extension().is_none_or(|e| e != "bin") {
                continue;
            }
            let Some(module) = path
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .filter(|m| valid_module(m))
            else {
                continue;
            };
            if meta.len() == 0 || meta.len() > MAX_BIN_BYTES || !is_hex_bytecode(&path) {
                continue;
            }
            let abi = path.with_file_name(format!("{module}.abi.json"));
            artifacts.push(DeployArtifact {
                module,
                dir: relative_dir(cwd, &dir),
                bin_path: path.to_string_lossy().into_owned(),
                abi_path: abi
                    .is_file()
                    .then(|| abi.to_string_lossy().into_owned()),
                digest: digest_near(&dir),
                modified_ms: modified_ms(&meta),
            });
        }
    }
    artifacts.sort_by(|a, b| b.modified_ms.cmp(&a.modified_ms));
    artifacts
}

fn skip_dir(name: &str) -> bool {
    name.starts_with('.')
        || matches!(
            name,
            "node_modules" | "target" | "dist" | "build" | "vendor" | "__pycache__"
        )
}

fn valid_module(module: &str) -> bool {
    let mut chars = module.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    first.is_ascii_alphabetic()
        && module.len() <= 64
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Cheap content check: the head of the file must be hex (after optional 0x).
fn is_hex_bytecode(path: &Path) -> bool {
    use std::io::Read;
    let Ok(mut file) = std::fs::File::open(path) else {
        return false;
    };
    let mut head = [0u8; 256];
    let Ok(n) = file.read(&mut head) else {
        return false;
    };
    let text = String::from_utf8_lossy(&head[..n]);
    let text = text.trim_start().trim_start_matches("0x");
    !text.is_empty()
        && text
            .chars()
            .all(|c| c.is_ascii_hexdigit() || c.is_ascii_whitespace())
}

/// `outputSetDigest` from a gate report / inspect dump next to the bytecode.
fn digest_near(dir: &Path) -> Option<String> {
    for name in ["gate-report.json", "inspect.json", "inspect.txt"] {
        let path = dir.join(name);
        let Ok(raw) = std::fs::read_to_string(&path) else {
            continue;
        };
        if let Some(digest) = parse_output_set_digest(&raw) {
            return Some(digest);
        }
    }
    None
}

/// Find the 64-hex value following an `outputSetDigest` mention.
pub fn parse_output_set_digest(raw: &str) -> Option<String> {
    let idx = raw.find("outputSetDigest")?;
    raw[idx..]
        .split(|c: char| !c.is_ascii_alphanumeric())
        .find(|part| part.len() == 64 && part.chars().all(|c| c.is_ascii_hexdigit()))
        .map(str::to_string)
}

fn relative_dir(cwd: &Path, dir: &Path) -> String {
    dir.strip_prefix(cwd)
        .map(|p| {
            let s = p.to_string_lossy();
            if s.is_empty() { ".".into() } else { s.into_owned() }
        })
        .unwrap_or_else(|_| dir.to_string_lossy().into_owned())
}

fn modified_ms(meta: &std::fs::Metadata) -> u64 {
    meta.modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

// ---------------------------------------------------------------------------
// Constructor args (native ABI encoding — no foundry dependency)
// ---------------------------------------------------------------------------

/// ABI-encode constructor args from a human signature. Accepts
/// `constructor(uint64,address)`, `(uint64,address)`, or `uint64,address`;
/// empty / `-` with no args encodes to nothing.
pub fn encode_ctor_args(sig: &str, args: &[String]) -> Result<Vec<u8>, String> {
    let inner = sig.trim();
    let inner = if inner == "-" { "" } else { inner };
    let inner = inner.strip_prefix("constructor").unwrap_or(inner).trim();
    let inner = inner
        .strip_prefix('(')
        .and_then(|s| s.strip_suffix(')'))
        .unwrap_or(inner)
        .trim();
    if inner.is_empty() {
        return if args.is_empty() {
            Ok(Vec::new())
        } else {
            Err(format!(
                "constructor signature lists no parameters but {} args were given",
                args.len()
            ))
        };
    }
    let tuple: DynSolType = format!("({inner})")
        .parse()
        .map_err(|e| format!("bad constructor signature '{sig}': {e}"))?;
    let DynSolType::Tuple(types) = tuple else {
        return Err(format!("bad constructor signature '{sig}'"));
    };
    if types.len() != args.len() {
        return Err(format!(
            "constructor expects {} args, got {}",
            types.len(),
            args.len()
        ));
    }
    let mut values = Vec::with_capacity(types.len());
    for (ty, arg) in types.iter().zip(args) {
        let value = ty
            .coerce_str(arg.trim())
            .map_err(|e| format!("arg '{arg}' does not fit {ty}: {e}"))?;
        values.push(value);
    }
    Ok(DynSolValue::Tuple(values).abi_encode_params())
}

// ---------------------------------------------------------------------------
// Deploy
// ---------------------------------------------------------------------------

/// Wallet/network checks before anything is read or sent.
pub fn preflight(network: &EvmNetwork, wallet: &WalletAccount) -> Result<(), String> {
    if !network.enabled {
        return Err(format!(
            "network {} is disabled — enable it in Settings → Networks first",
            network.name
        ));
    }
    match wallet.source {
        WalletSource::Watch => {
            return Err("watch-only wallets cannot sign deploy transactions".into());
        }
        WalletSource::WalletConnect => {
            if wallet.address.trim().is_empty() {
                return Err(
                    "WalletConnect wallet has no address — Connect in Settings → Wallets first"
                        .into(),
                );
            }
        }
        WalletSource::DevEnvKey => {
            if MAINNET_CHAIN_IDS.contains(&network.chain_id) {
                return Err(format!(
                    "DevEnvKey wallets cannot deploy to mainnet (chain id {}); use a testnet only",
                    network.chain_id
                ));
            }
            if wallet
                .env_key_name
                .as_deref()
                .unwrap_or("")
                .trim()
                .is_empty()
            {
                return Err("DevEnvKey wallet missing env_key_name".into());
            }
        }
        WalletSource::Local => {
            if wallet.address.trim().is_empty() {
                return Err("local wallet has no address — create or import it again".into());
            }
        }
    }
    Ok(())
}

/// Read the sealed bytecode, encode ctor args, sign + broadcast a create tx,
/// persist and return the [`DeploymentRecord`].
pub async fn deploy(
    req: &DeploySendRequest,
    network: &EvmNetwork,
    wallet: &WalletAccount,
    secrets: &WalletSecrets,
    wallet_connect: &WalletConnectBridge,
    store: &DeployStore,
) -> Result<DeploymentRecord, String> {
    preflight(network, wallet)?;

    let bytecode = read_bytecode(Path::new(&req.bin_path)).await?;
    let ctor_hex = if req.ctor_sig.trim().is_empty() || req.ctor_sig.trim() == "-" {
        String::new()
    } else {
        alloy::hex::encode(encode_ctor_args(&req.ctor_sig, &req.ctor_args)?)
    };
    let create_data = format!("0x{bytecode}{ctor_hex}");

    let (address, tx_hash) = match wallet.source {
        WalletSource::Watch => return Err("watch-only wallets cannot sign".into()),
        WalletSource::Local => {
            let sent = send_with_local(
                secrets,
                &wallet.id,
                &network.rpc_url,
                network.chain_id,
                None,
                &create_data,
            )
            .await?;
            match sent.contract_address {
                Some(address) => (address, sent.tx_hash),
                None => {
                    return Err(format!(
                        "deploy tx {} mined without a contract address",
                        sent.tx_hash
                    ));
                }
            }
        }
        WalletSource::DevEnvKey => {
            let env_name = wallet.env_key_name.as_deref().unwrap_or("");
            let key = match std::env::var(env_name) {
                Ok(key) if !key.trim().is_empty() => key,
                Ok(_) => return Err(format!("env var '{env_name}' is empty")),
                Err(_) => return Err(format!("env var '{env_name}' is not set")),
            };
            let sent = send_with_key(
                key.trim(),
                &network.rpc_url,
                network.chain_id,
                None,
                &create_data,
            )
            .await?;
            match sent.contract_address {
                Some(address) => (address, sent.tx_hash),
                None => {
                    return Err(format!(
                        "deploy tx {} mined without a contract address",
                        sent.tx_hash
                    ));
                }
            }
        }
        WalletSource::WalletConnect => {
            let from = wallet.address.clone();
            let tx = serde_json::json!({
                "from": from,
                "data": create_data,
                "chainId": format!("0x{:x}", network.chain_id),
            });
            let tx_hash = wallet_connect.request_send_transaction(&from, tx).await?;
            let address = wait_contract_address(&network.rpc_url, &tx_hash).await?;
            (address, tx_hash)
        }
    };

    let record = DeploymentRecord {
        id: Uuid::new_v4().to_string(),
        module: req.module.clone(),
        network_id: network.id.clone(),
        address,
        ctor: Some(req.ctor_sig.trim())
            .filter(|s| !s.is_empty() && *s != "-")
            .map(str::to_string),
        digest: req.digest.clone(),
        tx_hash,
        ts: Utc::now().to_rfc3339(),
    };
    // Persist before returning so a client disconnect still keeps the record.
    store
        .append(record.clone())
        .map_err(|err| format!("deployed on-chain but failed to persist record: {err}"))?;
    Ok(record)
}

async fn read_bytecode(path: &Path) -> Result<String, String> {
    let raw = tokio::fs::read_to_string(path)
        .await
        .map_err(|err| format!("bin missing or unreadable at {}: {err}", path.display()))?;
    let stripped: String = raw.chars().filter(|c| !c.is_whitespace()).collect();
    let hex = stripped.trim_start_matches("0x");
    if hex.is_empty() {
        return Err(format!("bin file is empty: {}", path.display()));
    }
    if hex.len() % 2 != 0 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(format!("bin file is not hex bytecode: {}", path.display()));
    }
    Ok(hex.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use comet_proto::builtin_networks;

    fn xlayer_testnet() -> EvmNetwork {
        builtin_networks()
            .into_iter()
            .find(|n| n.id == "xlayer-testnet")
            .unwrap()
    }

    fn xlayer_mainnet() -> EvmNetwork {
        builtin_networks()
            .into_iter()
            .find(|n| n.id == "xlayer-mainnet")
            .unwrap()
    }

    #[test]
    fn store_roundtrip_newest_first() {
        let dir = tempfile::tempdir().unwrap();
        let store = DeployStore::new(dir.path());
        let record = |id: &str| DeploymentRecord {
            id: id.into(),
            module: "Mod".into(),
            network_id: "xlayer-testnet".into(),
            address: "0xabc".into(),
            ctor: None,
            digest: None,
            tx_hash: "0xtx".into(),
            ts: "2026-08-13T00:00:00Z".into(),
        };
        store.append(record("d1")).unwrap();
        let list = store.append(record("d2")).unwrap();
        assert_eq!(list[0].id, "d2");
        assert_eq!(list[1].id, "d1");
        assert_eq!(store.load().unwrap(), list);
    }

    #[test]
    fn scan_finds_bin_with_abi_and_digest() {
        let temp = tempfile::tempdir().unwrap();
        let out = temp.path().join("out-evm");
        std::fs::create_dir_all(&out).unwrap();
        std::fs::write(out.join("EscrowVault.bin"), "0x6080604052\n").unwrap();
        std::fs::write(out.join("EscrowVault.abi.json"), "[]").unwrap();
        let digest = "a".repeat(64);
        std::fs::write(
            out.join("gate-report.json"),
            format!(r#"{{"outputSetDigest":"{digest}"}}"#),
        )
        .unwrap();
        // Noise that must not match.
        std::fs::write(out.join("notes.bin"), "not hex at all!").unwrap();
        std::fs::create_dir_all(temp.path().join(".git")).unwrap();
        std::fs::write(temp.path().join(".git/junk.bin"), "6080").unwrap();

        let artifacts = scan_artifacts(temp.path());
        assert_eq!(artifacts.len(), 1, "{artifacts:?}");
        let a = &artifacts[0];
        assert_eq!(a.module, "EscrowVault");
        assert_eq!(a.dir, "out-evm");
        assert!(a.abi_path.is_some());
        assert_eq!(a.digest.as_deref(), Some(digest.as_str()));
    }

    #[test]
    fn ctor_encoding_matches_abi() {
        // No-arg forms.
        assert!(encode_ctor_args("", &[]).unwrap().is_empty());
        assert!(encode_ctor_args("-", &[]).unwrap().is_empty());
        assert!(encode_ctor_args("constructor()", &[]).unwrap().is_empty());
        // uint64 pair — 2 head words.
        let encoded =
            encode_ctor_args("constructor(uint64,uint64)", &["7".into(), "9".into()]).unwrap();
        assert_eq!(encoded.len(), 64);
        assert_eq!(encoded[31], 7);
        assert_eq!(encoded[63], 9);
        // Bare type list works too.
        let bare = encode_ctor_args("uint64,uint64", &["7".into(), "9".into()]).unwrap();
        assert_eq!(bare, encoded);
        // Address coercion.
        let encoded = encode_ctor_args(
            "(address)",
            &["0x00000000000000000000000000000000000000aB".into()],
        )
        .unwrap();
        assert_eq!(encoded.len(), 32);
        assert_eq!(encoded[31], 0xab);
        // Arity + coercion failures are errors, not silent misencodes.
        assert!(encode_ctor_args("(uint64)", &[]).is_err());
        assert!(encode_ctor_args("(uint64)", &["not-a-number".into()]).is_err());
        assert!(encode_ctor_args("", &["7".into()]).is_err());
    }

    #[test]
    fn digest_parse_finds_hex() {
        let digest = "b".repeat(64);
        let raw = format!("stuff outputSetDigest = {digest} more");
        assert_eq!(parse_output_set_digest(&raw).as_deref(), Some(digest.as_str()));
        assert!(parse_output_set_digest("no digest here").is_none());
    }

    #[test]
    fn preflight_policy() {
        let wallet = |source: WalletSource| WalletAccount {
            id: "w".into(),
            label: "w".into(),
            address: "0x0000000000000000000000000000000000000001".into(),
            source,
            env_key_name: Some("PF_XLAYER_KEY".into()),
        };
        assert!(preflight(&xlayer_testnet(), &wallet(WalletSource::Watch)).is_err());
        preflight(&xlayer_testnet(), &wallet(WalletSource::Local)).unwrap();
        preflight(&xlayer_mainnet(), &wallet(WalletSource::Local)).unwrap();
        preflight(&xlayer_testnet(), &wallet(WalletSource::DevEnvKey)).unwrap();
        assert!(preflight(&xlayer_mainnet(), &wallet(WalletSource::DevEnvKey)).is_err());
        let mut wc = wallet(WalletSource::WalletConnect);
        preflight(&xlayer_mainnet(), &wc).unwrap();
        wc.address = String::new();
        assert!(preflight(&xlayer_testnet(), &wc).is_err());
        // Disabled networks fail preflight regardless of wallet.
        let mut disabled = xlayer_testnet();
        disabled.enabled = false;
        assert!(preflight(&disabled, &wallet(WalletSource::Local)).is_err());
    }
}
