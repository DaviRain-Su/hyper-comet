//! Engine → Cloudflare relay WebSocket client (Phase 3.2).
//!
//! Mirrors `proofship/bridge/server.mjs` relay behavior: the local engine is the
//! sole writer. Env:
//! - `PROOFSHIP_RELAY` — Worker base URL (required to enable)
//! - `PROOFSHIP_DEVICE_TOKEN` / `ENGINE_TOKEN` — shared spike token
//! - `PROOFSHIP_LAUNCH_ID` — room id (default `default`)

use std::sync::{Arc, Mutex};
use std::time::Duration;

use futures::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message};

const BUFFER_CAP: usize = 200;
const MAX_TEXT: usize = 4096;

#[derive(Clone, Default)]
pub struct StudioRelay {
    inner: Arc<Mutex<Option<RelayState>>>,
}

struct RelayState {
    tx: mpsc::UnboundedSender<String>,
}

#[derive(Debug, Clone)]
pub struct RelayCommand {
    pub kind: RelayCommandKind,
    pub nl: Option<String>,
    pub lane: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelayCommandKind {
    Prompt,
    Cancel,
}

impl StudioRelay {
    pub fn new() -> Self {
        Self::default()
    }

    /// Spawn the reconnecting client when `PROOFSHIP_RELAY` is set.
    /// Returns a command receiver for web→engine prompts (optional consumers).
    /// No-ops (returns None) if already started or env is unset.
    pub fn start_from_env(&self) -> Option<mpsc::UnboundedReceiver<RelayCommand>> {
        {
            let guard = self.inner.lock().unwrap_or_else(|e| e.into_inner());
            if guard.is_some() {
                return None;
            }
        }
        let base = std::env::var("PROOFSHIP_RELAY").ok()?;
        let base = base.trim().trim_end_matches('/').to_string();
        if base.is_empty() {
            return None;
        }
        let token = std::env::var("PROOFSHIP_DEVICE_TOKEN")
            .or_else(|_| std::env::var("ENGINE_TOKEN"))
            .unwrap_or_default();
        let launch_id = std::env::var("PROOFSHIP_LAUNCH_ID").unwrap_or_else(|_| "default".into());
        let (out_tx, out_rx) = mpsc::unbounded_channel::<String>();
        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel::<RelayCommand>();
        *self.inner.lock().unwrap_or_else(|e| e.into_inner()) = Some(RelayState { tx: out_tx });
        tokio::spawn(run_client(base, token, launch_id, out_rx, cmd_tx));
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

    pub fn note(&self, text: &str) {
        let trimmed: String = text.chars().take(MAX_TEXT).collect();
        self.publish("note", serde_json::json!({ "text": trimmed }));
    }
}

fn socket_url(base: &str, launch_id: &str, token: &str) -> String {
    let mut u = base.replace("https://", "wss://").replace("http://", "ws://");
    if u.ends_with('/') {
        u.pop();
    }
    format!(
        "{u}/ws/engine/{}?token={}",
        urlencoding_encode(launch_id),
        urlencoding_encode(token)
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
    launch_id: String,
    mut out_rx: mpsc::UnboundedReceiver<String>,
    cmd_tx: mpsc::UnboundedSender<RelayCommand>,
) {
    let mut backoff = Duration::from_secs(1);
    let mut queue: Vec<String> = Vec::new();
    loop {
        let url = socket_url(&base, &launch_id, &token);
        match connect_async(&url).await {
            Ok((ws, _)) => {
                tracing::info!(%url, "studio relay connected");
                backoff = Duration::from_secs(1);
                let (mut write, mut read) = ws.split();
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
                tracing::warn!(error = %err, "studio relay connect failed");
            }
        }
        tracing::warn!(?backoff, "studio relay reconnecting");
        tokio::time::sleep(backoff).await;
        backoff = (backoff * 2).min(Duration::from_secs(30));
    }
}

fn parse_command(text: &str) -> Option<RelayCommand> {
    let value: serde_json::Value = serde_json::from_str(text).ok()?;
    let ty = value.get("type")?.as_str()?;
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
                kind: RelayCommandKind::Prompt,
                nl: Some(nl.chars().take(4000).collect()),
                lane,
            })
        }
        "cmd.cancel" => Some(RelayCommand {
            kind: RelayCommandKind::Cancel,
            nl: None,
            lane: None,
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn socket_url_rewrites_https() {
        let url = socket_url("https://example.workers.dev", "launch/1", "tok");
        assert!(url.starts_with("wss://example.workers.dev/ws/engine/"));
        assert!(url.contains("token=tok"));
        assert!(url.contains("launch%2F1"));
    }

    #[test]
    fn parse_prompt_command() {
        let cmd = parse_command(r#"{"type":"cmd.prompt","nl":"hi","lane":"codex"}"#).unwrap();
        assert_eq!(cmd.kind, RelayCommandKind::Prompt);
        assert_eq!(cmd.nl.as_deref(), Some("hi"));
        assert_eq!(cmd.lane.as_deref(), Some("codex"));
    }
}
