//! Studio deploy lane — re-run the product gate, then `cast create` on an EVM network.
//!
//! Persistence lives in `{data_dir}/studio/deployments.json`; the deployer streams
//! progress events and returns a [`DeploymentRecord`] on success. The RPC coordinator
//! resolves network + wallet rows and appends records via [`DeployStore`].

use std::path::{Path, PathBuf};
use std::process::Stdio;

use chrono::Utc;
use comet_proto::{
    DeploymentRecord, EvmNetwork, StudioDeployEvent, StudioDeployRequest, StudioGateDigest,
    StudioGateEvent, WalletAccount, WalletSource,
};
use futures::StreamExt;
use futures::stream::BoxStream;
use tokio::io::AsyncReadExt;
use tokio::process::Command;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::studio::gate::StudioGate;

const MAX_DEPLOYMENTS: usize = 100;
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

    pub fn path(&self) -> &Path {
        &self.file
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

    /// Prepend `record`, cap at 100, atomically persist, return newest-first list.
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
        let json = serde_json::to_vec(&deployments)?;
        std::fs::write(&tmp, json)?;
        std::fs::rename(&tmp, &self.file)?;
        Ok(deployments)
    }
}

#[derive(Debug, Clone)]
pub struct StudioDeployer {
    gate: StudioGate,
    inbox_root: PathBuf,
    store: DeployStore,
}

impl StudioDeployer {
    pub fn new(gate: StudioGate, inbox_root: PathBuf, store: DeployStore) -> Self {
        Self {
            gate,
            inbox_root,
            store,
        }
    }

    pub fn deploy(
        &self,
        req: StudioDeployRequest,
        network: EvmNetwork,
        wallet: WalletAccount,
    ) -> BoxStream<'static, StudioDeployEvent> {
        let gate = self.gate.clone();
        let inbox_root = self.inbox_root.clone();
        let store = self.store.clone();
        let (tx, rx) = mpsc::channel(16);
        tokio::spawn(async move {
            deploy_inner(req, network, wallet, gate, inbox_root, store, tx).await;
        });
        futures::stream::unfold(rx, |mut rx| async move {
            rx.recv().await.map(|item| (item, rx))
        })
        .boxed()
    }
}

/// Wallet/network checks before gate or cast run.
pub fn preflight(network: &EvmNetwork, wallet: &WalletAccount) -> Result<(), String> {
    match wallet.source {
        WalletSource::Watch => {
            return Err("watch-only wallets cannot sign deploy transactions".into());
        }
        WalletSource::WalletConnect => {
            return Err("WalletConnect signing is not implemented yet".into());
        }
        WalletSource::DevEnvKey => {}
    }
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
    Ok(())
}

/// Gate output directory + `{module}.bin`, matching `gate.rs` / `deploy-xlayer-testnet.sh`.
pub fn artifact_bin_path(inbox_root: &Path, module: &str) -> PathBuf {
    inbox_root
        .join("studio-inbox")
        .join(format!("out-{}", module.to_lowercase()))
        .join(format!("{module}.bin"))
}

async fn deploy_inner(
    req: StudioDeployRequest,
    network: EvmNetwork,
    wallet: WalletAccount,
    gate: StudioGate,
    inbox_root: PathBuf,
    store: DeployStore,
    tx: mpsc::Sender<StudioDeployEvent>,
) {
    let network_id = req.network_id.clone();
    let _ = tx
        .send(StudioDeployEvent::Started {
            network_id: network_id.clone(),
        })
        .await;

    if let Err(err) = preflight(&network, &wallet) {
        let _ = tx
            .send(StudioDeployEvent::Done {
                ok: false,
                record: None,
                error: Some(err),
            })
            .await;
        return;
    }

    let gate_outcome = run_gate(&gate, req.module.clone(), req.source.clone()).await;
    if !gate_outcome.ok {
        let output = gate_outcome
            .diagnostics
            .unwrap_or_else(|| "gate failed".into());
        let _ = tx
            .send(StudioDeployEvent::Gate {
                ok: false,
                output: output.clone(),
            })
            .await;
        let _ = tx
            .send(StudioDeployEvent::Done {
                ok: false,
                record: None,
                error: Some(output),
            })
            .await;
        return;
    }

    let digest_str = gate_outcome
        .digest
        .output_set_digest
        .clone()
        .unwrap_or_else(|| gate_outcome.digest.raw.clone());
    let _ = tx
        .send(StudioDeployEvent::Gate {
            ok: true,
            output: digest_str.clone(),
        })
        .await;

    let bin_path = artifact_bin_path(&inbox_root, &req.module);
    let bytecode = match read_bytecode(&bin_path).await {
        Ok(bytecode) => bytecode,
        Err(err) => {
            let _ = tx
                .send(StudioDeployEvent::Done {
                    ok: false,
                    record: None,
                    error: Some(err),
                })
                .await;
            return;
        }
    };

    let cast = match resolve_cast() {
        Some(cast) => cast,
        None => {
            let _ = tx
                .send(StudioDeployEvent::Done {
                    ok: false,
                    record: None,
                    error: Some("cast not found (PATH or ~/.foundry/bin)".into()),
                })
                .await;
            return;
        }
    };

    let ctor_encoded = if req.ctor_sig == "-" {
        String::new()
    } else {
        match run_abi_encode(&cast, &req.ctor_sig, &req.ctor_args).await {
            Ok(encoded) => encoded,
            Err(err) => {
                let _ = tx
                    .send(StudioDeployEvent::Done {
                        ok: false,
                        record: None,
                        error: Some(err),
                    })
                    .await;
                return;
            }
        }
    };

    let create_data = format!("0x{bytecode}{ctor_encoded}");

    let _ = tx
        .send(StudioDeployEvent::Sending {
            rpc_url: network.rpc_url.clone(),
        })
        .await;

    let env_name = wallet.env_key_name.as_deref().unwrap_or("");
    let private_key = match std::env::var(env_name) {
        Ok(key) if !key.trim().is_empty() => key,
        Ok(_) => {
            let _ = tx
                .send(StudioDeployEvent::Done {
                    ok: false,
                    record: None,
                    error: Some(format!("env var '{env_name}' is empty")),
                })
                .await;
            return;
        }
        Err(_) => {
            let _ = tx
                .send(StudioDeployEvent::Done {
                    ok: false,
                    record: None,
                    error: Some(format!("env var '{env_name}' is not set")),
                })
                .await;
            return;
        }
    };

    match run_cast_create(&cast, &network.rpc_url, &private_key, &create_data).await {
        Ok((address, tx_hash)) => {
            let record = DeploymentRecord {
                id: Uuid::new_v4().to_string(),
                launch_id: req.launch_id.clone(),
                network_id,
                address,
                ctor: if req.ctor_sig == "-" {
                    None
                } else {
                    Some(req.ctor_sig.clone())
                },
                digest: gate_outcome.digest.output_set_digest.clone(),
                tx_hash,
                ts: Utc::now().to_rfc3339(),
            };
            // Persist before emitting Done so a client disconnect still keeps the record.
            if let Err(err) = store.append(record.clone()) {
                let _ = tx
                    .send(StudioDeployEvent::Done {
                        ok: false,
                        record: None,
                        error: Some(format!(
                            "deployed on-chain but failed to persist record: {err}"
                        )),
                    })
                    .await;
                return;
            }
            let _ = tx
                .send(StudioDeployEvent::Done {
                    ok: true,
                    record: Some(record),
                    error: None,
                })
                .await;
        }
        Err(err) => {
            let _ = tx
                .send(StudioDeployEvent::Done {
                    ok: false,
                    record: None,
                    error: Some(err),
                })
                .await;
        }
    }
}

struct GateOutcome {
    ok: bool,
    digest: StudioGateDigest,
    diagnostics: Option<String>,
}

async fn run_gate(gate: &StudioGate, module: String, source: String) -> GateOutcome {
    let mut stream = gate.run_gate(module, source);
    let mut last_output = None;
    while let Some(event) = stream.next().await {
        match event {
            StudioGateEvent::StageDone { output, .. } if !output.trim().is_empty() => {
                last_output = Some(output);
            }
            StudioGateEvent::Done {
                ok,
                digest,
                stage: _,
                artifacts: _,
            } => {
                return GateOutcome {
                    ok,
                    digest,
                    diagnostics: last_output,
                };
            }
            _ => {}
        }
    }
    GateOutcome {
        ok: false,
        digest: StudioGateDigest::default(),
        diagnostics: last_output,
    }
}

async fn read_bytecode(path: &Path) -> Result<String, String> {
    let raw = tokio::fs::read_to_string(path)
        .await
        .map_err(|err| format!("bin missing or unreadable at {}: {err}", path.display()))?;
    let stripped: String = raw.chars().filter(|c| !c.is_whitespace()).collect();
    if stripped.is_empty() {
        return Err(format!("bin file is empty: {}", path.display()));
    }
    Ok(stripped.trim_start_matches("0x").to_string())
}

pub(crate) fn resolve_cast() -> Option<PathBuf> {
    if let Some(cast) = find_on_path("cast") {
        return Some(cast);
    }
    let home = std::env::var_os("HOME")?;
    let candidate = PathBuf::from(home).join(".foundry/bin/cast");
    is_executable(&candidate).then_some(candidate)
}

async fn run_abi_encode(cast: &Path, ctor_sig: &str, args: &[String]) -> Result<String, String> {
    let mut command = Command::new(cast);
    command.arg("abi-encode").arg(ctor_sig);
    for arg in args {
        command.arg(arg);
    }
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    let output = command
        .output()
        .await
        .map_err(|err| format!("cast abi-encode failed to start: {err}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("cast abi-encode failed: {}", stderr.trim()));
    }
    let encoded = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(encoded.trim_start_matches("0x").to_string())
}

async fn run_cast_create(
    cast: &Path,
    rpc_url: &str,
    private_key: &str,
    create_data: &str,
) -> Result<(String, String), String> {
    let mut command = Command::new(cast);
    command
        .arg("send")
        .arg("--json")
        .arg("--rpc-url")
        .arg(rpc_url)
        .arg("--private-key")
        .arg(private_key)
        .arg("--create")
        .arg(create_data)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command
        .spawn()
        .map_err(|err| format!("cast send failed to start: {err}"))?;
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let out_task = tokio::spawn(read_to_string(stdout));
    let err_task = tokio::spawn(read_to_string(stderr));
    let status = child
        .wait()
        .await
        .map_err(|err| format!("cast send wait failed: {err}"))?;
    let stdout = out_task
        .await
        .map_err(|err| format!("cast stdout read failed: {err}"))?
        .unwrap_or_default();
    let stderr = err_task
        .await
        .map_err(|err| format!("cast stderr read failed: {err}"))?
        .unwrap_or_default();
    if !status.success() {
        let detail = if stderr.trim().is_empty() {
            stdout.trim().to_string()
        } else {
            stderr.trim().to_string()
        };
        return Err(format!(
            "cast send failed: {}",
            detail.replace(private_key, "<redacted>")
        ));
    }
    parse_cast_send_json(&stdout)
}

fn parse_cast_send_json(raw: &str) -> Result<(String, String), String> {
    let value: serde_json::Value = serde_json::from_str(raw)
        .map_err(|err| format!("cast send returned invalid JSON: {err}"))?;
    let address = value
        .get("contractAddress")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    if address.is_empty() || address == "null" {
        return Err(format!("cast send returned no contractAddress: {raw}"));
    }
    let tx_hash = value
        .get("transactionHash")
        .or_else(|| value.get("hash"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    if tx_hash.is_empty() {
        return Err(format!("cast send returned no transaction hash: {raw}"));
    }
    Ok((address, tx_hash))
}

async fn read_to_string<R>(reader: Option<R>) -> std::io::Result<String>
where
    R: tokio::io::AsyncRead + Unpin,
{
    let Some(mut reader) = reader else {
        return Ok(String::new());
    };
    let mut buf = Vec::new();
    reader.read_to_end(&mut buf).await?;
    Ok(String::from_utf8_lossy(&buf).into_owned())
}

fn find_on_path(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join(name))
        .find(|candidate| is_executable(candidate))
}

fn is_executable(path: &Path) -> bool {
    let Ok(meta) = std::fs::metadata(path) else {
        return false;
    };
    if !meta.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        meta.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
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

    fn dev_env_wallet() -> WalletAccount {
        WalletAccount {
            id: "dev".into(),
            label: "dev key".into(),
            address: String::new(),
            source: WalletSource::DevEnvKey,
            env_key_name: Some("PF_XLAYER_KEY".into()),
        }
    }

    #[test]
    fn deploy_store_roundtrip_newest_first() {
        let dir = tempfile::tempdir().unwrap();
        let store = DeployStore::new(dir.path());
        let first = DeploymentRecord {
            id: "d1".into(),
            launch_id: None,
            network_id: "xlayer-testnet".into(),
            address: "0xabc".into(),
            ctor: None,
            digest: Some("digest1".into()),
            tx_hash: "0xtx1".into(),
            ts: "2026-08-12T00:00:00Z".into(),
        };
        let second = DeploymentRecord {
            id: "d2".into(),
            launch_id: Some("launch".into()),
            network_id: "xlayer-testnet".into(),
            address: "0xdef".into(),
            ctor: Some("constructor()".into()),
            digest: None,
            tx_hash: "0xtx2".into(),
            ts: "2026-08-12T01:00:00Z".into(),
        };
        let saved = store.append(first.clone()).unwrap();
        assert_eq!(saved.len(), 1);
        assert_eq!(saved[0].id, "d1");
        let saved = store.append(second.clone()).unwrap();
        assert_eq!(saved.len(), 2);
        assert_eq!(saved[0].id, "d2");
        assert_eq!(saved[1].id, "d1");
        let loaded = store.load().unwrap();
        assert_eq!(loaded, saved);
        assert!(store.path().exists());
    }

    #[test]
    fn preflight_rejects_watch_wallet() {
        let wallet = WalletAccount {
            id: "w".into(),
            label: "watch".into(),
            address: "0x0000000000000000000000000000000000000001".into(),
            source: WalletSource::Watch,
            env_key_name: None,
        };
        let err = preflight(&xlayer_testnet(), &wallet).unwrap_err();
        assert!(err.contains("watch-only"));
        assert!(err.contains("cannot sign"));
    }

    #[test]
    fn preflight_rejects_wallet_connect() {
        let wallet = WalletAccount {
            id: "wc".into(),
            label: "wc".into(),
            address: String::new(),
            source: WalletSource::WalletConnect,
            env_key_name: None,
        };
        let err = preflight(&xlayer_testnet(), &wallet).unwrap_err();
        assert!(err.contains("WalletConnect"));
        assert!(err.contains("not implemented"));
    }

    #[test]
    fn preflight_rejects_mainnet_dev_env_key() {
        let err = preflight(&xlayer_mainnet(), &dev_env_wallet()).unwrap_err();
        assert!(err.contains("mainnet"));
        assert!(err.contains("196"));
    }

    #[test]
    fn preflight_rejects_ethereum_mainnet_dev_env_key() {
        let eth = EvmNetwork {
            id: "eth".into(),
            name: "Ethereum".into(),
            chain_id: 1,
            rpc_url: "https://example.invalid".into(),
            explorer_url: None,
            currency_symbol: "ETH".into(),
            builtin: false,
        };
        let err = preflight(&eth, &dev_env_wallet()).unwrap_err();
        assert!(err.contains("mainnet"));
        assert!(err.contains("1"));
    }

    #[test]
    fn preflight_accepts_xlayer_testnet_dev_env_key() {
        preflight(&xlayer_testnet(), &dev_env_wallet()).unwrap();
    }

    #[test]
    fn artifact_bin_path_matches_gate_layout() {
        let root = PathBuf::from("/tmp/inbox");
        let path = artifact_bin_path(&root, "RwaShareRegistry");
        assert_eq!(
            path,
            root.join("studio-inbox/out-rwashareregistry/RwaShareRegistry.bin")
        );
    }

    #[test]
    fn parse_cast_send_json_reads_address_and_hash() {
        let raw = r#"{"contractAddress":"0xabc","transactionHash":"0xdead"}"#;
        let (addr, hash) = parse_cast_send_json(raw).unwrap();
        assert_eq!(addr, "0xabc");
        assert_eq!(hash, "0xdead");
        let raw = r#"{"contractAddress":"0xabc","hash":"0xbeef"}"#;
        let (_, hash) = parse_cast_send_json(raw).unwrap();
        assert_eq!(hash, "0xbeef");
    }
}
