//! ABI schema load + `cast call` / `cast send` for the Studio interact panel.

use std::path::{Path, PathBuf};
use std::process::Stdio;

use comet_proto::{
    EvmNetwork, StudioAbiResponse, StudioCallKind, StudioCallRequest, StudioCallResponse,
    StudioLogEntry, StudioLogsRequest, StudioLogsResponse, WalletAccount, WalletSource,
};
use tokio::process::Command;

use super::deploy::{preflight, resolve_cast};
use super::WalletConnectBridge;

#[derive(Debug, Clone)]
pub struct StudioInteract {
    inbox_root: PathBuf,
    wallet_connect: WalletConnectBridge,
}

impl StudioInteract {
    pub fn new(inbox_root: PathBuf, wallet_connect: WalletConnectBridge) -> Self {
        Self {
            inbox_root,
            wallet_connect,
        }
    }

    pub fn inbox_root(&self) -> &Path {
        &self.inbox_root
    }

    pub async fn load_abi(&self, module: &str) -> Result<StudioAbiResponse, String> {
        let path = find_abi_path(&self.inbox_root, module).await?;
        let abi_json = tokio::fs::read_to_string(&path)
            .await
            .map_err(|err| format!("abi unreadable at {}: {err}", path.display()))?;
        let _ = comet_abi::schema_from_abi_json(&abi_json)
            .map_err(|err| format!("abi parse failed: {err}"))?;
        Ok(StudioAbiResponse {
            module: module.to_string(),
            abi_json,
        })
    }

    /// Relay-safe sealed artifact meta for web W3 Fill-from-snapshot.
    pub fn sealed_for_relay(
        &self,
        module: &str,
        output_set_digest: Option<&str>,
        address: Option<&str>,
    ) -> serde_json::Value {
        sealed_artifact_payload(&self.inbox_root, module, output_set_digest, address)
    }

    pub async fn call(
        &self,
        req: StudioCallRequest,
        network: EvmNetwork,
        wallet: Option<WalletAccount>,
    ) -> StudioCallResponse {
        match call_inner(req, network, wallet, &self.wallet_connect).await {
            Ok(resp) => resp,
            Err(err) => StudioCallResponse {
                ok: false,
                output: err,
                tx_hash: None,
            },
        }
    }

    pub async fn logs(&self, req: StudioLogsRequest, network: EvmNetwork) -> StudioLogsResponse {
        match logs_inner(req, network).await {
            Ok(resp) => resp,
            Err(err) => StudioLogsResponse {
                ok: false,
                logs: Vec::new(),
                output: err,
            },
        }
    }
}

pub fn artifact_out_dir(inbox_root: &Path, module: &str) -> PathBuf {
    inbox_root
        .join("studio-inbox")
        .join(format!("out-{}", module.to_lowercase()))
}

/// Relay-safe `artifact.sealed` payload for web Fill-from-snapshot (no private keys).
pub fn sealed_artifact_payload(
    inbox_root: &Path,
    module: &str,
    output_set_digest: Option<&str>,
    address: Option<&str>,
) -> serde_json::Value {
    let out_dir = artifact_out_dir(inbox_root, module);
    let mut files = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&out_dir) {
        for entry in rd.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
            files.push(serde_json::json!({ "name": name, "size": size }));
        }
    }
    files.sort_by(|a, b| {
        a.get("name")
            .and_then(|v| v.as_str())
            .cmp(&b.get("name").and_then(|v| v.as_str()))
    });

    let digest = output_set_digest
        .map(str::to_string)
        .or_else(|| {
            let report_path = out_dir.join(comet_proto::StudioGateReport::FILE_NAME);
            std::fs::read_to_string(report_path)
                .ok()
                .and_then(|raw| serde_json::from_str::<comet_proto::StudioGateReport>(&raw).ok())
                .and_then(|r| r.output_set_digest)
        });

    let abi = find_abi_path_sync(inbox_root, module)
        .ok()
        .and_then(|path| std::fs::read_to_string(path).ok())
        .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok());

    let mut payload = serde_json::json!({
        "module": module,
        "files": files,
    });
    if let Some(d) = digest {
        payload["outputSetDigest"] = serde_json::json!(d);
    }
    if let Some(abi) = abi {
        payload["abi"] = abi;
    }
    if let Some(addr) = address.filter(|a| !a.is_empty()) {
        payload["address"] = serde_json::json!(addr);
    }
    payload
}

fn find_abi_path_sync(inbox_root: &Path, module: &str) -> Result<PathBuf, String> {
    let out_dir = artifact_out_dir(inbox_root, module);
    let preferred = out_dir.join(format!("{module}.abi.json"));
    if preferred.is_file() {
        return Ok(preferred);
    }
    let rd = std::fs::read_dir(&out_dir).map_err(|err| {
        format!(
            "no gate artifacts for {module} under {}: {err}",
            out_dir.display()
        )
    })?;
    for entry in rd.flatten() {
        let name = entry.file_name();
        if name.to_string_lossy().ends_with(".abi.json") {
            return Ok(entry.path());
        }
    }
    Err(format!(
        "no *.abi.json in {} — run the gate first",
        out_dir.display()
    ))
}

async fn find_abi_path(inbox_root: &Path, module: &str) -> Result<PathBuf, String> {
    find_abi_path_sync(inbox_root, module)
}

async fn call_inner(
    req: StudioCallRequest,
    network: EvmNetwork,
    wallet: Option<WalletAccount>,
    wallet_connect: &WalletConnectBridge,
) -> Result<StudioCallResponse, String> {
    if !req.address.starts_with("0x") || req.address.len() != 42 {
        return Err("address must be 0x + 40 hex chars".into());
    }
    if req.signature.trim().is_empty() {
        return Err("signature must not be empty".into());
    }
    let cast =
        resolve_cast().ok_or_else(|| "cast not found (PATH or ~/.foundry/bin)".to_string())?;

    match req.kind {
        StudioCallKind::View => {
            let mut prefix = vec![
                "call".into(),
                "--rpc-url".into(),
                network.rpc_url,
                req.address,
                req.signature,
            ];
            prefix.extend(req.args);
            let output = run_cast(&cast, &prefix, &[], None).await?;
            Ok(StudioCallResponse {
                ok: true,
                output,
                tx_hash: None,
            })
        }
        StudioCallKind::Send => {
            let wallet = wallet.ok_or_else(|| "walletId required for send".to_string())?;
            preflight(&network, &wallet)?;
            match wallet.source {
                WalletSource::DevEnvKey => {
                    let env_name = wallet
                        .env_key_name
                        .as_deref()
                        .filter(|s| !s.is_empty())
                        .ok_or_else(|| "DevEnvKey wallet missing env_key_name".to_string())?;
                    let key = std::env::var(env_name)
                        .map_err(|_| format!("env var '{env_name}' is not set"))?;
                    if key.trim().is_empty() {
                        return Err(format!("env var '{env_name}' is empty"));
                    }
                    let mut prefix = vec![
                        "send".into(),
                        "--json".into(),
                        "--rpc-url".into(),
                        network.rpc_url,
                        "--private-key".into(),
                        key,
                        req.address,
                        req.signature,
                    ];
                    prefix.extend(req.args);
                    let output = run_cast(&cast, &prefix, &[], Some("--private-key")).await?;
                    let tx_hash = parse_tx_hash(&output);
                    Ok(StudioCallResponse {
                        ok: true,
                        output,
                        tx_hash,
                    })
                }
                WalletSource::WalletConnect => {
                    let mut calldata_args = vec!["calldata".into(), req.signature.clone()];
                    calldata_args.extend(req.args.clone());
                    let data = run_cast(&cast, &calldata_args, &[], None).await?;
                    let data = data.trim().to_string();
                    if !data.starts_with("0x") {
                        return Err(format!("cast calldata returned unexpected output: {data}"));
                    }
                    let from = wallet.address.clone();
                    let tx_obj = serde_json::json!({
                        "from": from,
                        "to": req.address,
                        "data": data,
                        "chainId": format!("0x{:x}", network.chain_id),
                    });
                    let tx_hash = wallet_connect
                        .request_send_transaction(&from, tx_obj)
                        .await?;
                    Ok(StudioCallResponse {
                        ok: true,
                        output: tx_hash.clone(),
                        tx_hash: Some(tx_hash),
                    })
                }
                WalletSource::Watch => Err("watch-only wallets cannot sign".into()),
            }
        }
    }
}

async fn logs_inner(
    req: StudioLogsRequest,
    network: EvmNetwork,
) -> Result<StudioLogsResponse, String> {
    if !req.address.starts_with("0x") || req.address.len() != 42 {
        return Err("address must be 0x + 40 hex chars".into());
    }
    let cast =
        resolve_cast().ok_or_else(|| "cast not found (PATH or ~/.foundry/bin)".to_string())?;

    let to_block = req
        .to_block
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "latest".into());
    let from_block = if let Some(fb) = req.from_block.filter(|s| !s.trim().is_empty()) {
        fb
    } else {
        recent_from_block(&cast, &network.rpc_url).await?
    };

    let mut args = vec![
        "logs".into(),
        "--json".into(),
        "--rpc-url".into(),
        network.rpc_url.clone(),
        "--address".into(),
        req.address.clone(),
        "--from-block".into(),
        from_block,
        "--to-block".into(),
        to_block,
    ];
    if let Some(sig) = req
        .event_signature
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        args.push(sig.to_string());
    }

    let output = run_cast(&cast, &args, &[], None).await?;
    let logs = parse_log_entries(&output, req.event_signature.as_deref());
    Ok(StudioLogsResponse {
        ok: true,
        logs,
        output,
    })
}

async fn recent_from_block(cast: &Path, rpc_url: &str) -> Result<String, String> {
    let raw = run_cast(
        cast,
        &[
            "block-number".into(),
            "--rpc-url".into(),
            rpc_url.to_string(),
        ],
        &[],
        None,
    )
    .await?;
    let tip: u64 = raw
        .trim()
        .parse()
        .map_err(|_| format!("cast block-number returned unexpected output: {raw}"))?;
    Ok(tip.saturating_sub(10_000).to_string())
}

fn parse_log_entries(raw: &str, event_signature: Option<&str>) -> Vec<StudioLogEntry> {
    let event_name = event_signature.and_then(|sig| sig.split('(').next()).map(str::to_string);
    let Ok(value) = serde_json::from_str::<serde_json::Value>(raw) else {
        return Vec::new();
    };
    let Some(arr) = value.as_array() else {
        return Vec::new();
    };
    arr.iter()
        .filter_map(|item| {
            let block_number = item
                .get("blockNumber")
                .or_else(|| item.get("block_number"))
                .and_then(|v| match v {
                    serde_json::Value::String(s) => Some(s.clone()),
                    serde_json::Value::Number(n) => Some(n.to_string()),
                    _ => None,
                });
            let tx_hash = item
                .get("transactionHash")
                .or_else(|| item.get("transaction_hash"))
                .and_then(|v| v.as_str())
                .map(str::to_string);
            let data = item
                .get("data")
                .and_then(|v| v.as_str())
                .map(str::to_string);
            let topics = item
                .get("topics")
                .and_then(|v| v.as_array())
                .map(|topics| {
                    topics
                        .iter()
                        .filter_map(|t| t.as_str().map(str::to_string))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            Some(StudioLogEntry {
                block_number,
                tx_hash,
                topics,
                data,
                event_name: event_name.clone(),
            })
        })
        .collect()
}

/// `redact_flag` names an argv flag whose following value must never appear in errors.
async fn run_cast(
    cast: &Path,
    prefix: &[String],
    args: &[String],
    redact_flag: Option<&str>,
) -> Result<String, String> {
    let mut command = Command::new(cast);
    for part in prefix {
        command.arg(part);
    }
    for arg in args {
        command.arg(arg);
    }
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    let output = command
        .output()
        .await
        .map_err(|err| format!("cast spawn failed: {err}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if !output.status.success() {
        let combined = if stderr.is_empty() { stdout } else { stderr };
        return Err(redact_secret(&combined, prefix, redact_flag));
    }
    Ok(stdout)
}

fn redact_secret(text: &str, prefix: &[String], redact_flag: Option<&str>) -> String {
    let Some(flag) = redact_flag else {
        return text.to_string();
    };
    let Some(ix) = prefix.iter().position(|p| p == flag) else {
        return text.to_string();
    };
    let Some(secret) = prefix.get(ix + 1) else {
        return text.to_string();
    };
    if secret.is_empty() {
        return text.to_string();
    }
    text.replace(secret, "<redacted>")
}

fn parse_tx_hash(json: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(json).ok()?;
    value
        .get("transactionHash")
        .or_else(|| value.get("hash"))
        .and_then(|v| v.as_str())
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redact_replaces_private_key_in_error_text() {
        let prefix = vec![
            "send".into(),
            "--private-key".into(),
            "0xabc123secret".into(),
        ];
        let out = redact_secret("boom 0xabc123secret failed", &prefix, Some("--private-key"));
        assert!(!out.contains("0xabc123secret"));
        assert!(out.contains("<redacted>"));
    }

    #[test]
    fn parse_tx_hash_reads_cast_json() {
        let json = r#"{"transactionHash":"0xdead","status":"0x1"}"#;
        assert_eq!(parse_tx_hash(json).as_deref(), Some("0xdead"));
    }

    #[test]
    fn parse_log_entries_reads_cast_array() {
        let raw = r#"[{
            "address":"0xabc",
            "topics":["0x111"],
            "data":"0x222",
            "blockNumber":"0x10",
            "transactionHash":"0xdead"
        }]"#;
        let logs = parse_log_entries(raw, Some("Claimed(uint64)"));
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].tx_hash.as_deref(), Some("0xdead"));
        assert_eq!(logs[0].event_name.as_deref(), Some("Claimed"));
        assert_eq!(logs[0].topics, vec!["0x111".to_string()]);
    }

    #[tokio::test]
    async fn load_abi_reads_module_abi_json() {
        let dir = tempfile::tempdir().unwrap();
        let out = artifact_out_dir(dir.path(), "RwaShareRegistry");
        tokio::fs::create_dir_all(&out).await.unwrap();
        let abi = include_str!("../../../abi/tests/fixtures/rwa_share_registry.abi.json");
        tokio::fs::write(out.join("RwaShareRegistry.abi.json"), abi)
            .await
            .unwrap();
        let interact = StudioInteract::new(dir.path().to_path_buf(), WalletConnectBridge::new());
        let resp = interact.load_abi("RwaShareRegistry").await.unwrap();
        let schema = comet_abi::schema_from_abi_json(&resp.abi_json).unwrap();
        assert!(schema.constructor.is_some());
        assert!(schema.entries.iter().any(|f| f.name == "issue"));
    }

    #[test]
    fn sealed_artifact_payload_includes_abi_and_digest() {
        let dir = tempfile::tempdir().unwrap();
        let out = artifact_out_dir(dir.path(), "RwaShareRegistry");
        std::fs::create_dir_all(&out).unwrap();
        let abi = include_str!("../../../abi/tests/fixtures/rwa_share_registry.abi.json");
        std::fs::write(out.join("RwaShareRegistry.abi.json"), abi).unwrap();
        std::fs::write(out.join("RwaShareRegistry.bin"), b"00").unwrap();
        let payload = sealed_artifact_payload(
            dir.path(),
            "RwaShareRegistry",
            Some("deadbeef"),
            Some("0xabc"),
        );
        assert_eq!(payload["module"], "RwaShareRegistry");
        assert_eq!(payload["outputSetDigest"], "deadbeef");
        assert_eq!(payload["address"], "0xabc");
        assert!(payload["abi"].is_array());
        let files = payload["files"].as_array().unwrap();
        assert!(files.iter().any(|f| f["name"] == "RwaShareRegistry.abi.json"));
        assert!(!payload.to_string().contains("private"));
    }
}
