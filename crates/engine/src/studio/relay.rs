//! Engine → Cloudflare relay WebSocket client (web coordinator).
//!
//! The local engine is a **UserExecutor**. Env:
//! - `PROOFSHIP_RELAY` — Worker base URL. **Unset defaults to the hosted
//!   ProofShip relay.** Set `off` / `0` / `-` to disable.
//! - `PROOFSHIP_DEVICE_TOKEN` / `DEVICE_TOKEN` / `ENGINE_TOKEN` — device auth
//! - `PROOFSHIP_DEVICE_ID` — device id (default: this install's engine id)
//! - `PROOFSHIP_SESSION_ID` / `PROOFSHIP_LAUNCH_ID` — room id
//!   (default: `desktop-{deviceId}` so machines do not collide)
//! - `PROOFSHIP_RELAY_CHAT_ID` — Sessions chat used for web prompts (default `proofship-relay`)

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use futures::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message};

const BUFFER_CAP: usize = 200;
const MAX_TEXT: usize = 4096;

/// Hosted ProofShip coordinator (Cloudflare Worker, 2026-08-13).
pub const DEFAULT_PROOFSHIP_RELAY: &str = "https://proofship-relay.davirain-yin.workers.dev";
/// Hosted Sessions viewer (Cloudflare Pages).
pub const DEFAULT_PROOFSHIP_WEB: &str = "https://proofship-web.pages.dev";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelayIdentity {
    pub base: String,
    pub device_id: String,
    pub session_id: String,
}

impl RelayIdentity {
    pub fn web_url(&self) -> String {
        format!(
            "{}/?relay={}&session={}",
            DEFAULT_PROOFSHIP_WEB.trim_end_matches('/'),
            urlencoding_encode(&self.base),
            urlencoding_encode(&self.session_id)
        )
    }
}

/// Resolve the Worker base. Unset → hosted default. `off`/`0`/`-`/`false` → disabled.
pub fn resolve_relay_base(raw: Option<&str>) -> Option<String> {
    match raw {
        None => Some(DEFAULT_PROOFSHIP_RELAY.trim_end_matches('/').to_string()),
        Some(value) => {
            let trimmed = value.trim();
            if trimmed.is_empty() || is_relay_off(trimmed) {
                None
            } else {
                Some(trimmed.trim_end_matches('/').to_string())
            }
        }
    }
}

fn is_relay_off(value: &str) -> bool {
    matches!(
        value.to_ascii_lowercase().as_str(),
        "off" | "0" | "-" | "false" | "disable" | "disabled" | "none" | "local"
    )
}

/// Device + room used when env vars are omitted.
pub fn resolve_relay_identity(
    base: &str,
    default_device_id: &str,
    device_override: Option<&str>,
    session_override: Option<&str>,
) -> RelayIdentity {
    let device_id = device_override
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(default_device_id)
        .to_string();
    let session_id = session_override
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| format!("desktop-{device_id}"));
    RelayIdentity {
        base: base.to_string(),
        device_id,
        session_id,
    }
}

/// Env token wins; otherwise a stable file under `{data_dir}/studio/relay-token`.
pub fn resolve_device_token(data_dir: &Path) -> String {
    for key in ["PROOFSHIP_DEVICE_TOKEN", "DEVICE_TOKEN", "ENGINE_TOKEN"] {
        if let Ok(value) = std::env::var(key) {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                return trimmed.to_string();
            }
        }
    }
    load_or_create_relay_token(data_dir)
}

fn load_or_create_relay_token(data_dir: &Path) -> String {
    let path = data_dir.join("studio").join("relay-token");
    if let Ok(existing) = std::fs::read_to_string(&path) {
        let trimmed = existing.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }
    let token = format!("ps_{}", uuid::Uuid::new_v4().simple());
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(&path, &token);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = std::fs::metadata(&path) {
            let mut perms = meta.permissions();
            perms.set_mode(0o600);
            let _ = std::fs::set_permissions(&path, perms);
        }
    }
    token
}

fn identity_from_env(base: &str, default_device_id: &str) -> RelayIdentity {
    let device = std::env::var("PROOFSHIP_DEVICE_ID").ok();
    let session = std::env::var("PROOFSHIP_SESSION_ID")
        .or_else(|_| std::env::var("PROOFSHIP_LAUNCH_ID"))
        .ok();
    resolve_relay_identity(
        base,
        default_device_id,
        device.as_deref(),
        session.as_deref(),
    )
}

#[derive(Clone, Default)]
pub struct StudioRelay {
    inner: Arc<Mutex<Option<RelayState>>>,
    default_device: Arc<Mutex<String>>,
    connected: Arc<AtomicBool>,
    catalog: Arc<Mutex<serde_json::Value>>,
}

struct RelayState {
    tx: mpsc::UnboundedSender<String>,
}

#[derive(Debug, Clone)]
pub struct RelayCommand {
    pub id: Option<String>,
    pub kind: RelayCommandKind,
    pub nl: Option<String>,
    pub lane: Option<String>,
    pub chat_id: Option<String>,
    pub network_id: Option<String>,
    pub module: Option<String>,
    pub digest: Option<String>,
    pub executor: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelayCommandKind {
    Prompt,
    Cancel,
    Steer,
    Deploy,
}

impl StudioRelay {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_default_device(&self, device_id: &str) {
        *self.default_device.lock().unwrap_or_else(|e| e.into_inner()) = device_id.to_string();
    }

    pub fn set_harness_catalog(&self, catalog: serde_json::Value) {
        *self.catalog.lock().unwrap_or_else(|e| e.into_inner()) = catalog.clone();
        self.publish("harness.catalog", catalog);
    }

    pub fn default_device(&self) -> String {
        let id = self
            .default_device
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        if id.is_empty() {
            "desktop".into()
        } else {
            id
        }
    }

    pub fn status(&self) -> comet_proto::StudioRelayStatus {
        let device = self.default_device();
        match resolve_relay_base(std::env::var("PROOFSHIP_RELAY").ok().as_deref()) {
            Some(base) => {
                let id = identity_from_env(&base, &device);
                comet_proto::StudioRelayStatus {
                    enabled: true,
                    connected: self.connected.load(Ordering::Relaxed),
                    web_url: Some(id.web_url()),
                    base: Some(id.base),
                    device_id: id.device_id,
                    session_id: id.session_id,
                }
            }
            None => comet_proto::StudioRelayStatus {
                enabled: false,
                connected: false,
                base: None,
                device_id: device,
                session_id: String::new(),
                web_url: None,
            },
        }
    }

    /// Spawn the reconnecting client. Hosted relay is the default; `PROOFSHIP_RELAY=off` disables.
    pub fn start_from_env(
        &self,
        default_device_id: &str,
        data_dir: &Path,
    ) -> Option<mpsc::UnboundedReceiver<RelayCommand>> {
        {
            let guard = self.inner.lock().unwrap_or_else(|e| e.into_inner());
            if guard.is_some() {
                return None;
            }
        }
        let base = resolve_relay_base(std::env::var("PROOFSHIP_RELAY").ok().as_deref())?;
        self.set_default_device(default_device_id);
        let identity = identity_from_env(&base, default_device_id);
        let token = resolve_device_token(data_dir);
        let (out_tx, out_rx) = mpsc::unbounded_channel::<String>();
        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel::<RelayCommand>();
        *self.inner.lock().unwrap_or_else(|e| e.into_inner()) = Some(RelayState { tx: out_tx });
        tracing::info!(
            base = %identity.base,
            device = %identity.device_id,
            session = %identity.session_id,
            web = %identity.web_url(),
            "proofship relay starting (user executor)"
        );
        tokio::spawn(run_client(
            identity.base,
            token,
            identity.device_id,
            identity.session_id,
            out_rx,
            cmd_tx,
            self.connected.clone(),
            self.catalog.clone(),
        ));
        Some(cmd_rx)
    }

    pub fn publish(&self, kind: &str, payload: serde_json::Value) {
        let msg = serde_json::json!({
            "type": "event",
            "kind": kind,
            "payload": payload,
        })
        .to_string();
        if let Ok(guard) = self.inner.lock()
            && let Some(state) = guard.as_ref()
        {
            let _ = state.tx.send(msg);
        }
    }

    pub fn ack(&self, id: &str) {
        let msg = serde_json::json!({ "type": "cmd.ack", "id": id }).to_string();
        if let Ok(guard) = self.inner.lock()
            && let Some(state) = guard.as_ref()
        {
            let _ = state.tx.send(msg);
        }
    }

    pub fn note(&self, text: &str) {
        let trimmed: String = text.chars().take(MAX_TEXT).collect();
        self.publish("note", serde_json::json!({ "text": trimmed }));
    }
}

fn socket_url(base: &str, session_id: &str, token: &str, device_id: &str) -> String {
    let mut u = base.replace("https://", "wss://").replace("http://", "ws://");
    if u.ends_with('/') {
        u.pop();
    }
    format!(
        "{u}/ws/engine/{}?token={}&deviceId={}&role=engine",
        urlencoding_encode(session_id),
        urlencoding_encode(token),
        urlencoding_encode(device_id)
    )
}

fn urlencoding_encode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

async fn run_client(
    base: String,
    token: String,
    device_id: String,
    session_id: String,
    mut out_rx: mpsc::UnboundedReceiver<String>,
    cmd_tx: mpsc::UnboundedSender<RelayCommand>,
    connected: Arc<AtomicBool>,
    catalog: Arc<Mutex<serde_json::Value>>,
) {
    let mut backoff = Duration::from_secs(1);
    let mut queue: Vec<String> = Vec::new();
    loop {
        let url = socket_url(&base, &session_id, &token, &device_id);
        match connect_async(&url).await {
            Ok((ws, _)) => {
                tracing::info!(%url, "proofship relay connected (user executor)");
                connected.store(true, Ordering::Relaxed);
                backoff = Duration::from_secs(1);
                let (mut write, mut read) = ws.split();
                // Announce session to viewers (relay also emits executor.online).
                let catalog =
                    catalog.lock().unwrap_or_else(|e| e.into_inner()).clone();
                let mut open_payload = serde_json::json!({
                    "sessionId": session_id,
                    "deviceId": device_id,
                    "role": "engine",
                });
                if let Some(obj) = catalog.as_object() {
                    for (key, value) in obj {
                        open_payload[key] = value.clone();
                    }
                }
                let open = serde_json::json!({
                    "type": "event",
                    "kind": "session.open",
                    "payload": open_payload,
                })
                .to_string();
                let _ = write.send(Message::Text(open)).await;
                for msg in queue.drain(..) {
                    if write.send(Message::Text(msg)).await.is_err() {
                        break;
                    }
                }
                loop {
                    tokio::select! {
                        outgoing = out_rx.recv() => {
                            match outgoing {
                                Some(msg) => {
                                    if write.send(Message::Text(msg.clone())).await.is_err() {
                                        queue.push(msg);
                                        while queue.len() > BUFFER_CAP {
                                            queue.remove(0);
                                        }
                                        break;
                                    }
                                }
                                None => return,
                            }
                        }
                        incoming = read.next() => {
                            match incoming {
                                Some(Ok(Message::Text(text))) => {
                                    if let Some(cmd) = parse_command(&text) {
                                        let _ = cmd_tx.send(cmd);
                                    }
                                }
                                Some(Ok(Message::Ping(p))) => {
                                    let _ = write.send(Message::Pong(p)).await;
                                }
                                Some(Ok(Message::Close(_))) | None => break,
                                Some(Err(_)) => break,
                                _ => {}
                            }
                        }
                    }
                }
            }
            Err(err) => {
                tracing::warn!(error = %err, "proofship relay connect failed");
            }
        }
        connected.store(false, Ordering::Relaxed);
        tracing::warn!(?backoff, "proofship relay reconnecting");
        tokio::time::sleep(backoff).await;
        backoff = (backoff * 2).min(Duration::from_secs(30));
    }
}

fn parse_command(text: &str) -> Option<RelayCommand> {
    let value: serde_json::Value = serde_json::from_str(text).ok()?;
    let ty = value.get("type")?.as_str()?;
    let id = value
        .get("id")
        .and_then(|v| v.as_str())
        .map(str::to_string);
    let chat_id = value
        .get("chatId")
        .and_then(|v| v.as_str())
        .map(str::to_string);
    let executor = value
        .get("executor")
        .and_then(|v| v.as_str())
        .map(str::to_string);
    match ty {
        "cmd.prompt" => {
            let nl = value.get("nl")?.as_str()?.to_string();
            if nl.trim().is_empty() {
                return None;
            }
            let lane = value
                .get("lane")
                .and_then(|v| v.as_str())
                .map(str::to_string);
            Some(RelayCommand {
                id,
                kind: RelayCommandKind::Prompt,
                nl: Some(nl.chars().take(4000).collect()),
                lane,
                chat_id,
                network_id: None,
                module: None,
                digest: None,
                executor,
            })
        }
        "cmd.steer" => {
            let nl = value.get("nl")?.as_str()?.to_string();
            if nl.trim().is_empty() {
                return None;
            }
            Some(RelayCommand {
                id,
                kind: RelayCommandKind::Steer,
                nl: Some(nl.chars().take(4000).collect()),
                lane: None,
                chat_id,
                network_id: None,
                module: None,
                digest: None,
                executor,
            })
        }
        "cmd.cancel" => Some(RelayCommand {
            id,
            kind: RelayCommandKind::Cancel,
            nl: None,
            lane: None,
            chat_id,
            network_id: None,
            module: None,
            digest: None,
            executor,
        }),
        "cmd.deploy" => {
            let network_id = value.get("networkId")?.as_str()?.to_string();
            let module = value.get("module")?.as_str()?.to_string();
            let digest = value
                .get("digest")
                .and_then(|v| v.as_str())
                .map(str::to_string);
            Some(RelayCommand {
                id,
                kind: RelayCommandKind::Deploy,
                nl: None,
                lane: None,
                chat_id,
                network_id: Some(network_id),
                module: Some(module),
                digest,
                executor,
            })
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unset_relay_defaults_to_hosted_worker() {
        let base = resolve_relay_base(None).expect("default");
        assert_eq!(base, DEFAULT_PROOFSHIP_RELAY);
        assert!(resolve_relay_base(Some("off")).is_none());
        assert!(resolve_relay_base(Some("LOCAL")).is_none());
        assert_eq!(
            resolve_relay_base(Some(" https://custom.example/ ")).as_deref(),
            Some("https://custom.example")
        );
    }

    #[test]
    fn relay_token_persists_and_skips_empty_env() {
        let dir = tempfile::tempdir().unwrap();
        let first = resolve_device_token(dir.path());
        let second = resolve_device_token(dir.path());
        assert_eq!(first, second);
        assert!(first.starts_with("ps_"));
        let mode = std::fs::metadata(dir.path().join("studio/relay-token"))
            .unwrap()
            .permissions();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(mode.mode() & 0o777, 0o600);
        }
    }

    #[test]
    fn identity_defaults_to_per_device_room() {
        let id = resolve_relay_identity(DEFAULT_PROOFSHIP_RELAY, "abc-123", None, None);
        assert_eq!(id.device_id, "abc-123");
        assert_eq!(id.session_id, "desktop-abc-123");
        assert!(id.web_url().contains("session=desktop-abc-123"));
        assert!(id.web_url().starts_with(DEFAULT_PROOFSHIP_WEB));
    }

    #[test]
    fn socket_url_rewrites_https() {
        let url = socket_url("https://example.workers.dev", "launch/1", "tok", "dev-a");
        assert!(url.starts_with("wss://example.workers.dev/ws/engine/"));
        assert!(url.contains("token=tok"));
        assert!(url.contains("deviceId=dev-a"));
        assert!(url.contains("launch%2F1"));
    }

    #[test]
    fn parse_prompt_command() {
        let cmd = parse_command(
            r#"{"type":"cmd.prompt","nl":"hi","lane":"codex","id":"1","executor":"user"}"#,
        )
        .unwrap();
        assert_eq!(cmd.kind, RelayCommandKind::Prompt);
        assert_eq!(cmd.nl.as_deref(), Some("hi"));
        assert_eq!(cmd.lane.as_deref(), Some("codex"));
        assert_eq!(cmd.id.as_deref(), Some("1"));
    }

    #[test]
    fn parse_deploy_command() {
        let cmd = parse_command(
            r#"{"type":"cmd.deploy","networkId":"xlayer-testnet","module":"Escrow","digest":"abc"}"#,
        )
        .unwrap();
        assert_eq!(cmd.kind, RelayCommandKind::Deploy);
        assert_eq!(cmd.network_id.as_deref(), Some("xlayer-testnet"));
        assert_eq!(cmd.module.as_deref(), Some("Escrow"));
    }
}
