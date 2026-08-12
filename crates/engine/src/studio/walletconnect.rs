//! WalletConnect / Reown connect bridge (desktop).
//!
//! Serves a local HTML page that runs the WalletConnect Ethereum Provider in
//! the system browser. On success the page POSTs `{address}` back; the engine
//! upserts an address-book row. **Session material stays in the browser tab /
//! memory — never on disk.** Signing through the session is a follow-up slice.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::{Mutex, oneshot};

use comet_proto::WalletAccount;
use uuid::Uuid;

use crate::studio::wallets::WalletStore;

#[derive(Clone, Default)]
pub struct WalletConnectBridge {
    inner: Arc<Mutex<Option<ActiveBridge>>>,
}

struct ActiveBridge {
    url: String,
    #[allow(dead_code)]
    label: String,
    shutdown: Option<oneshot::Sender<()>>,
}

#[derive(Debug, Clone)]
pub struct WalletConnectStart {
    pub url: String,
    pub label: String,
}

impl WalletConnectBridge {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn active_url(&self) -> Option<String> {
        self.inner.lock().await.as_ref().map(|b| b.url.clone())
    }

    pub async fn stop(&self) {
        let mut guard = self.inner.lock().await;
        if let Some(mut active) = guard.take()
            && let Some(tx) = active.shutdown.take()
        {
            let _ = tx.send(());
        }
    }

    /// Start (or replace) the connect page. `project_id` is required by Reown.
    pub async fn start(
        &self,
        project_id: String,
        label: String,
        wallet_store: WalletStore,
    ) -> Result<WalletConnectStart, String> {
        let project_id = project_id.trim().to_string();
        if project_id.is_empty() {
            return Err(
                "WalletConnect project id required (set PROOFSHIP_WC_PROJECT_ID or pass projectId)"
                    .into(),
            );
        }
        let label = if label.trim().is_empty() {
            "WalletConnect".into()
        } else {
            label.trim().to_string()
        };

        self.stop().await;

        let listener = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0)))
            .await
            .map_err(|e| format!("bind WalletConnect bridge: {e}"))?;
        let port = listener
            .local_addr()
            .map_err(|e| format!("wc bridge local_addr: {e}"))?
            .port();
        let url = format!("http://127.0.0.1:{port}/");
        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
        let html = connect_html(&project_id, &label);
        let done = Arc::new(Mutex::new(None::<WalletAccount>));

        tokio::spawn(serve_bridge(
            listener,
            html,
            wallet_store,
            label.clone(),
            done,
            shutdown_rx,
        ));

        *self.inner.lock().await = Some(ActiveBridge {
            url: url.clone(),
            label: label.clone(),
            shutdown: Some(shutdown_tx),
        });

        Ok(WalletConnectStart { url, label })
    }
}

fn connect_html(project_id: &str, label: &str) -> String {
    let pid = project_id
        .replace('\\', "\\\\")
        .replace('\'', "\\'")
        .replace('\n', "");
    let label = label
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");
    format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8"/>
<meta name="viewport" content="width=device-width, initial-scale=1"/>
<title>ProofShip · WalletConnect</title>
<style>
  body {{ margin:0; min-height:100vh; display:grid; place-items:center;
    font:14px/1.45 ui-sans-serif,system-ui,sans-serif; color:#e8ecf4;
    background:radial-gradient(900px 500px at 20% 0%,#1a2438,#0f1115); }}
  .card {{ width:min(420px,92vw); padding:24px; border-radius:14px; border:1px solid #2a3040; background:#171a21; }}
  h1 {{ margin:0 0 8px; font-size:18px; }}
  p {{ margin:0 0 16px; color:#8b93a7; }}
  button {{ appearance:none; border:1px solid #166534; background:#134e3a; color:#ecfdf5;
    border-radius:8px; padding:10px 14px; font:13px/1 inherit; cursor:pointer; width:100%; }}
  button:disabled {{ opacity:.5; cursor:not-allowed; }}
  .status {{ margin-top:14px; font-family:ui-monospace,monospace; font-size:12px; color:#6ee7b7; word-break:break-all; }}
  .err {{ color:#f87171; }}
</style>
</head>
<body>
  <div class="card">
    <h1>Connect wallet</h1>
    <p>Label: <strong>{label}</strong>. Scan the QR with a mobile wallet or approve in your extension. Keys never enter ProofShip.</p>
    <button id="connect" type="button">Connect with WalletConnect</button>
    <div id="status" class="status">Ready</div>
  </div>
<script type="module">
const PROJECT_ID = '{pid}';
const status = document.getElementById('status');
const btn = document.getElementById('connect');
function setStatus(text, err) {{
  status.textContent = text;
  status.className = 'status' + (err ? ' err' : '');
}}
btn.addEventListener('click', async () => {{
  btn.disabled = true;
  setStatus('Loading WalletConnect…');
  try {{
    const {{ EthereumProvider }} = await import('https://esm.sh/@walletconnect/ethereum-provider@2.17.3');
    const provider = await EthereumProvider.init({{
      projectId: PROJECT_ID,
      showQrModal: true,
      chains: [1952, 196, 1],
      optionalChains: [8453, 42161, 10, 137],
      methods: ['eth_sendTransaction', 'eth_signTransaction', 'personal_sign', 'eth_signTypedData'],
      events: ['chainChanged', 'accountsChanged'],
      metadata: {{
        name: 'ProofShip',
        description: 'Local-first web3 contract studio',
        url: 'https://proofship.dev',
        icons: ['https://avatars.githubusercontent.com/u/37784886']
      }}
    }});
    setStatus('Approve in your wallet…');
    await provider.enable();
    const accounts = provider.accounts || [];
    const address = accounts[0];
    if (!address) throw new Error('No account returned');
    const res = await fetch('/session', {{
      method: 'POST',
      headers: {{ 'content-type': 'application/json' }},
      body: JSON.stringify({{ address }})
    }});
    if (!res.ok) throw new Error(await res.text());
    setStatus('Connected: ' + address + ' — you can return to ProofShip.');
  }} catch (err) {{
    setStatus(String(err && err.message ? err.message : err), true);
    btn.disabled = false;
  }}
}});
</script>
</body>
</html>
"##,
        label = label,
        pid = pid,
    )
}

async fn serve_bridge(
    listener: TcpListener,
    html: String,
    wallet_store: WalletStore,
    label: String,
    done: Arc<Mutex<Option<WalletAccount>>>,
    mut shutdown: oneshot::Receiver<()>,
) {
    let html = Arc::new(html);
    loop {
        tokio::select! {
            _ = &mut shutdown => break,
            accepted = listener.accept() => {
                match accepted {
                    Ok((mut stream, _)) => {
                        let html = Arc::clone(&html);
                        let store = wallet_store.clone();
                        let label = label.clone();
                        let done = Arc::clone(&done);
                        tokio::spawn(async move {
                            let mut buf = vec![0u8; 8192];
                            let n = stream.read(&mut buf).await.unwrap_or(0);
                            let req = String::from_utf8_lossy(&buf[..n]);
                            let (status, body, ctype) = if req.starts_with("POST /session") {
                                match handle_session_post(&req, &store, &label).await {
                                    Ok(wallet) => {
                                        *done.lock().await = Some(wallet);
                                        (200, r#"{"ok":true}"#.to_string(), "application/json")
                                    }
                                    Err(err) => (400, err, "text/plain"),
                                }
                            } else {
                                (200, (*html).clone(), "text/html; charset=utf-8")
                            };
                            let status_text = match status {
                                200 => "OK",
                                400 => "Bad Request",
                                _ => "Error",
                            };
                            let header = format!(
                                "HTTP/1.1 {status} {status_text}\r\n\
                                 Content-Type: {ctype}\r\n\
                                 Content-Length: {}\r\n\
                                 Cache-Control: no-store\r\n\
                                 Connection: close\r\n\
                                 Access-Control-Allow-Origin: *\r\n\
                                 \r\n",
                                body.len()
                            );
                            let _ = stream.write_all(header.as_bytes()).await;
                            let _ = stream.write_all(body.as_bytes()).await;
                            let _ = stream.shutdown().await;
                        });
                    }
                    Err(_) => break,
                }
            }
        }
    }
}

async fn handle_session_post(
    req: &str,
    store: &WalletStore,
    label: &str,
) -> Result<WalletAccount, String> {
    let body = req.split("\r\n\r\n").nth(1).unwrap_or("").trim();
    let value: serde_json::Value =
        serde_json::from_str(body).map_err(|e| format!("bad json: {e}"))?;
    let address = value
        .get("address")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    if !address.starts_with("0x") || address.len() != 42 {
        return Err("invalid address".into());
    }
    let wallet = WalletAccount {
        id: format!("wc-{}", &Uuid::new_v4().to_string()[..8]),
        label: label.to_string(),
        address,
        source: comet_proto::WalletSource::WalletConnect,
        env_key_name: None,
    };
    store
        .upsert(wallet.clone())
        .map_err(|e| e.to_string())?;
    Ok(wallet)
}

/// Resolve project id: explicit param → env.
pub fn resolve_project_id(explicit: Option<&str>) -> Option<String> {
    if let Some(id) = explicit.map(str::trim).filter(|s| !s.is_empty()) {
        return Some(id.to_string());
    }
    for key in ["PROOFSHIP_WC_PROJECT_ID", "REOWN_PROJECT_ID", "WC_PROJECT_ID"] {
        if let Ok(v) = std::env::var(key) {
            let v = v.trim().to_string();
            if !v.is_empty() {
                return Some(v);
            }
        }
    }
    None
}

/// Parse a trivial HTTP header map (unused helper kept for tests).
#[allow(dead_code)]
fn parse_headers(req: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for line in req.lines().skip(1) {
        if line.is_empty() {
            break;
        }
        if let Some((k, v)) = line.split_once(':') {
            map.insert(k.trim().to_ascii_lowercase(), v.trim().to_string());
        }
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_project_id_prefers_explicit() {
        assert_eq!(
            resolve_project_id(Some(" abc ")).as_deref(),
            Some("abc")
        );
    }

    #[test]
    fn connect_html_embeds_project_id() {
        let html = connect_html("pid-123", "My WC");
        assert!(html.contains("pid-123"));
        assert!(html.contains("My WC"));
        assert!(html.contains("@walletconnect/ethereum-provider"));
    }
}
