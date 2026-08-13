//! WalletConnect / Reown connect + signing bridge (desktop).
//!
//! Serves a local HTML page that runs the WalletConnect Ethereum Provider in
//! the system browser. After connect, the tab stays open and polls for pending
//! `eth_sendTransaction` requests from the engine. **Session material stays in
//! the browser — never on disk.** Address-book rows only store label + address.

use std::collections::{HashMap, VecDeque};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use serde_json::json;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::{Mutex, oneshot};
use uuid::Uuid;

use comet_proto::WalletAccount;

use crate::wallets::WalletStore;

const SIGN_TIMEOUT: Duration = Duration::from_secs(300);

#[derive(Clone, Default)]
pub struct WalletConnectBridge {
    inner: Arc<Mutex<Option<ActiveBridge>>>,
}

impl std::fmt::Debug for WalletConnectBridge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WalletConnectBridge").finish_non_exhaustive()
    }
}

struct ActiveBridge {
    url: String,
    #[allow(dead_code)]
    label: String,
    address: Arc<Mutex<Option<String>>>,
    pending: Arc<Mutex<VecDeque<PendingTx>>>,
    shutdown: Option<oneshot::Sender<()>>,
}

struct PendingTx {
    id: String,
    from: String,
    tx: serde_json::Value,
    response: oneshot::Sender<Result<String, String>>,
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

    pub async fn connected_address(&self) -> Option<String> {
        let guard = self.inner.lock().await;
        let active = guard.as_ref()?;
        active.address.lock().await.clone()
    }

    pub async fn stop(&self) {
        let mut guard = self.inner.lock().await;
        if let Some(mut active) = guard.take() {
            // Fail any pending signers.
            let mut pending = active.pending.lock().await;
            while let Some(item) = pending.pop_front() {
                let _ = item
                    .response
                    .send(Err("WalletConnect bridge stopped".into()));
            }
            drop(pending);
            if let Some(tx) = active.shutdown.take() {
                let _ = tx.send(());
            }
        }
    }

    /// Start (or replace) the connect/signing page. `project_id` is required by Reown.
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
        let html = session_html(&project_id, &label);
        let address = Arc::new(Mutex::new(None::<String>));
        let pending = Arc::new(Mutex::new(VecDeque::<PendingTx>::new()));

        tokio::spawn(serve_bridge(
            listener,
            html,
            wallet_store,
            label.clone(),
            Arc::clone(&address),
            Arc::clone(&pending),
            shutdown_rx,
        ));

        *self.inner.lock().await = Some(ActiveBridge {
            url: url.clone(),
            label: label.clone(),
            address,
            pending,
            shutdown: Some(shutdown_tx),
        });

        Ok(WalletConnectStart { url, label })
    }

    /// Ask the open browser session to `eth_sendTransaction`. Returns tx hash.
    pub async fn request_send_transaction(
        &self,
        from: &str,
        tx: serde_json::Value,
    ) -> Result<String, String> {
        let (id, rx) = {
            let guard = self.inner.lock().await;
            let active = guard
                .as_ref()
                .ok_or_else(|| {
                    "WalletConnect bridge not running — open Settings → Wallets → Connect first"
                        .to_string()
                })?;
            let connected = active.address.lock().await.clone();
            match connected {
                Some(ref addr) if addr.eq_ignore_ascii_case(from) => {}
                Some(ref addr) => {
                    return Err(format!(
                        "WalletConnect session is for {addr}, but wallet row is {from}"
                    ));
                }
                None => {
                    return Err(
                        "WalletConnect session has no address yet — approve connect in the browser tab"
                            .into(),
                    );
                }
            }
            let id = Uuid::new_v4().to_string();
            let (tx_resp, rx) = oneshot::channel();
            active.pending.lock().await.push_back(PendingTx {
                id: id.clone(),
                from: from.to_string(),
                tx,
                response: tx_resp,
            });
            (id, rx)
        };

        match tokio::time::timeout(SIGN_TIMEOUT, rx).await {
            Ok(Ok(Ok(hash))) => Ok(hash),
            Ok(Ok(Err(err))) => Err(err),
            Ok(Err(_)) => Err(format!("WalletConnect signer dropped for request {id}")),
            Err(_) => Err(format!(
                "WalletConnect signing timed out after {}s — keep the bridge tab open and approve the tx",
                SIGN_TIMEOUT.as_secs()
            )),
        }
    }
}

fn session_html(project_id: &str, label: &str) -> String {
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
  .card {{ width:min(460px,92vw); padding:24px; border-radius:14px; border:1px solid #2a3040; background:#171a21; }}
  h1 {{ margin:0 0 8px; font-size:18px; }}
  p {{ margin:0 0 16px; color:#8b93a7; }}
  button {{ appearance:none; border:1px solid #166534; background:#134e3a; color:#ecfdf5;
    border-radius:8px; padding:10px 14px; font:13px/1 inherit; cursor:pointer; width:100%; }}
  button:disabled {{ opacity:.5; cursor:not-allowed; }}
  .status {{ margin-top:14px; font-family:ui-monospace,monospace; font-size:12px; color:#6ee7b7; word-break:break-all; }}
  .err {{ color:#f87171; }}
  .banner {{ margin-top:12px; padding:10px; border-radius:8px; border:1px solid #2a3040; color:#fbbf24; font-size:12px; }}
</style>
</head>
<body>
  <div class="card">
    <h1>WalletConnect session</h1>
    <p>Label: <strong>{label}</strong>. Keep this tab open so ProofShip can request signatures.</p>
    <button id="connect" type="button">Connect with WalletConnect</button>
    <div id="status" class="status">Ready</div>
    <div class="banner">After connect, leave this page open. Signing requests from ProofShip will prompt here.</div>
  </div>
<script type="module">
const PROJECT_ID = '{pid}';
const status = document.getElementById('status');
const btn = document.getElementById('connect');
let provider = null;
let account = null;
let polling = false;

function setStatus(text, err) {{
  status.textContent = text;
  status.className = 'status' + (err ? ' err' : '');
}}

async function postSession(address) {{
  const res = await fetch('/session', {{
    method: 'POST',
    headers: {{ 'content-type': 'application/json' }},
    body: JSON.stringify({{ address }})
  }});
  if (!res.ok) throw new Error(await res.text());
}}

async function pollPending() {{
  if (polling) return;
  polling = true;
  try {{
    while (provider && account) {{
      const res = await fetch('/pending');
      if (!res.ok) {{
        await new Promise(r => setTimeout(r, 1200));
        continue;
      }}
      const body = await res.json();
      if (!body || !body.id) {{
        await new Promise(r => setTimeout(r, 1200));
        continue;
      }}
      setStatus('Approve tx ' + body.id.slice(0, 8) + '…');
      try {{
        const hash = await provider.request({{
          method: 'eth_sendTransaction',
          params: [body.tx]
        }});
        await fetch('/result', {{
          method: 'POST',
          headers: {{ 'content-type': 'application/json' }},
          body: JSON.stringify({{ id: body.id, ok: true, hash }})
        }});
        setStatus('Signed: ' + hash);
      }} catch (err) {{
        await fetch('/result', {{
          method: 'POST',
          headers: {{ 'content-type': 'application/json' }},
          body: JSON.stringify({{
            id: body.id,
            ok: false,
            error: String(err && err.message ? err.message : err)
          }})
        }});
        setStatus(String(err && err.message ? err.message : err), true);
      }}
    }}
  }} finally {{
    polling = false;
  }}
}}

btn.addEventListener('click', async () => {{
  btn.disabled = true;
  setStatus('Loading WalletConnect…');
  try {{
    const {{ EthereumProvider }} = await import('https://esm.sh/@walletconnect/ethereum-provider@2.17.3');
    provider = await EthereumProvider.init({{
      projectId: PROJECT_ID,
      showQrModal: true,
      chains: [1952, 196, 11155111, 84532, 1],
      optionalChains: [8453, 42161, 10, 137],
      methods: ['eth_sendTransaction', 'eth_signTransaction', 'personal_sign', 'eth_signTypedData'],
      events: ['chainChanged', 'accountsChanged'],
      metadata: {{
        name: 'ProofShip',
        description: 'Local-first desktop for shipping Web3 products',
        url: 'https://proofship.dev',
        icons: ['https://avatars.githubusercontent.com/u/37784886']
      }}
    }});
    setStatus('Approve in your wallet…');
    await provider.enable();
    const accounts = provider.accounts || [];
    account = accounts[0];
    if (!account) throw new Error('No account returned');
    await postSession(account);
    setStatus('Connected: ' + account + ' — listening for txs');
    btn.textContent = 'Connected';
    pollPending();
    provider.on('accountsChanged', async (accs) => {{
      account = accs[0] || null;
      if (account) {{
        await postSession(account);
        setStatus('Connected: ' + account);
        pollPending();
      }}
    }});
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
    address: Arc<Mutex<Option<String>>>,
    pending: Arc<Mutex<VecDeque<PendingTx>>>,
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
                        let address = Arc::clone(&address);
                        let pending = Arc::clone(&pending);
                        tokio::spawn(async move {
                            let mut buf = vec![0u8; 16384];
                            let n = stream.read(&mut buf).await.unwrap_or(0);
                            let req = String::from_utf8_lossy(&buf[..n]);
                            let (status, body, ctype) = route_request(
                                &req, &html, &store, &label, &address, &pending,
                            )
                            .await;
                            let status_text = match status {
                                200 => "OK",
                                400 => "Bad Request",
                                404 => "Not Found",
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

async fn route_request(
    req: &str,
    html: &str,
    store: &WalletStore,
    label: &str,
    address: &Mutex<Option<String>>,
    pending: &Mutex<VecDeque<PendingTx>>,
) -> (u16, String, &'static str) {
    let line = req.lines().next().unwrap_or("");
    if line.starts_with("POST /session") {
        return match handle_session_post(req, store, label, address).await {
            Ok(_) => (200, r#"{"ok":true}"#.into(), "application/json"),
            Err(err) => (400, err, "text/plain"),
        };
    }
    if line.starts_with("GET /pending") {
        let queue = pending.lock().await;
        if let Some(front) = queue.front() {
            let body = json!({
                "id": front.id,
                "from": front.from,
                "tx": front.tx,
            })
            .to_string();
            return (200, body, "application/json");
        }
        return (200, "null".into(), "application/json");
    }
    if line.starts_with("POST /result") {
        return match handle_result_post(req, pending).await {
            Ok(()) => (200, r#"{"ok":true}"#.into(), "application/json"),
            Err(err) => (400, err, "text/plain"),
        };
    }
    if line.starts_with("GET /") || line.starts_with("GET / ") {
        return (200, html.to_string(), "text/html; charset=utf-8");
    }
    (404, "not found".into(), "text/plain")
}

async fn handle_session_post(
    req: &str,
    store: &WalletStore,
    label: &str,
    address_slot: &Mutex<Option<String>>,
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
    *address_slot.lock().await = Some(address.clone());

    // Upsert by address so reconnect refreshes the same bookkeeping row.
    let mut wallets = store.load().map_err(|e| e.to_string())?;
    if let Some(existing) = wallets
        .iter_mut()
        .find(|w| w.source == comet_proto::WalletSource::WalletConnect && w.address.eq_ignore_ascii_case(&address))
    {
        existing.label = label.to_string();
        existing.address = address.clone();
        let wallet = existing.clone();
        store.save(&wallets).map_err(|e| e.to_string())?;
        return Ok(wallet);
    }

    let wallet = WalletAccount {
        id: format!("wc-{}", &Uuid::new_v4().to_string()[..8]),
        label: label.to_string(),
        address,
        source: comet_proto::WalletSource::WalletConnect,
        env_key_name: None,
    };
    store.upsert(wallet.clone()).map_err(|e| e.to_string())?;
    Ok(wallet)
}

async fn handle_result_post(
    req: &str,
    pending: &Mutex<VecDeque<PendingTx>>,
) -> Result<(), String> {
    let body = req.split("\r\n\r\n").nth(1).unwrap_or("").trim();
    let value: serde_json::Value =
        serde_json::from_str(body).map_err(|e| format!("bad json: {e}"))?;
    let id = value
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "missing id".to_string())?;
    let mut queue = pending.lock().await;
    let ix = queue
        .iter()
        .position(|p| p.id == id)
        .ok_or_else(|| format!("unknown pending id {id}"))?;
    let item = queue.remove(ix).expect("index checked");
    let ok = value.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
    if ok {
        let hash = value
            .get("hash")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if hash.is_empty() {
            let _ = item.response.send(Err("empty tx hash".into()));
        } else {
            let _ = item.response.send(Ok(hash));
        }
    } else {
        let err = value
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("wallet rejected")
            .to_string();
        let _ = item.response.send(Err(err));
    }
    Ok(())
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

/// Wait for a contract-creation receipt and return the deployed address.
pub async fn wait_contract_address(rpc_url: &str, tx_hash: &str) -> Result<String, String> {
    let client = reqwest::Client::new();
    for _ in 0..60 {
        let body = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "eth_getTransactionReceipt",
            "params": [tx_hash]
        });
        let resp = client
            .post(rpc_url)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("receipt rpc: {e}"))?;
        let value: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("receipt json: {e}"))?;
        if let Some(result) = value.get("result")
            && !result.is_null()
        {
            if let Some(addr) = result.get("contractAddress").and_then(|v| v.as_str())
                && addr.starts_with("0x")
            {
                return Ok(addr.to_string());
            }
            return Err(format!("receipt missing contractAddress for {tx_hash}"));
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
    Err(format!("timed out waiting for receipt {tx_hash}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_project_id_prefers_explicit() {
        assert_eq!(resolve_project_id(Some(" abc ")).as_deref(), Some("abc"));
    }

    #[test]
    fn session_html_embeds_project_id_and_pending_poll() {
        let html = session_html("pid-123", "My WC");
        assert!(html.contains("pid-123"));
        assert!(html.contains("My WC"));
        assert!(html.contains("/pending"));
        assert!(html.contains("eth_sendTransaction"));
    }
}
