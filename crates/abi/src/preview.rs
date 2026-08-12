//! Self-contained HTML dapp preview generated from an ABI + deployment.

use crate::{AbiFormSchema, schema_from_abi_json};

/// Inputs for a local dapp HTML preview page.
#[derive(Debug, Clone)]
pub struct DappPreviewConfig {
    pub title: String,
    pub module: String,
    pub address: String,
    pub chain_id: u64,
    pub rpc_url: String,
    pub explorer_url: Option<String>,
    pub currency_symbol: String,
    /// Raw Solidity JSON ABI array (string).
    pub abi_json: String,
}

/// Build a single-file HTML dapp that can call views via JSON-RPC and
/// request writes through `window.ethereum` when present.
pub fn render_dapp_html(config: &DappPreviewConfig) -> Result<String, String> {
    let schema = schema_from_abi_json(&config.abi_json).map_err(|e| e.to_string())?;
    Ok(render_html(config, &schema))
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn escape_js_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 8);
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '\'' => out.push_str("\\'"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\u{2028}' => out.push_str("\\u2028"),
            '\u{2029}' => out.push_str("\\u2029"),
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

fn render_html(config: &DappPreviewConfig, schema: &AbiFormSchema) -> String {
    let title = escape_html(&config.title);
    let module = escape_html(&config.module);
    let address = escape_html(&config.address);
    let symbol = escape_html(&config.currency_symbol);
    let explorer = config
        .explorer_url
        .as_deref()
        .map(|u| {
            format!(
                r#"<a class="link" href="{}/address/{}" target="_blank" rel="noreferrer">Explorer</a>"#,
                escape_html(u.trim_end_matches('/')),
                escape_html(&config.address)
            )
        })
        .unwrap_or_default();

    let mut view_cards = String::new();
    for (ix, func) in schema.views.iter().enumerate() {
        view_cards.push_str(&fn_card("view", ix, func));
    }
    let mut entry_cards = String::new();
    for (ix, func) in schema.entries.iter().enumerate() {
        entry_cards.push_str(&fn_card("entry", ix, func));
    }

    let rpc_js = escape_js_string(&config.rpc_url);
    let addr_js = escape_js_string(&config.address);
    // Prevent `</script>` from closing the JSON carrier tag early.
    let abi_raw = config.abi_json.replace("</", "<\\/");

    format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8"/>
<meta name="viewport" content="width=device-width, initial-scale=1"/>
<title>{title} · ProofShip Preview</title>
<style>
  :root {{
    --bg: #0f1115;
    --panel: #171a21;
    --line: #2a3040;
    --text: #e8ecf4;
    --muted: #8b93a7;
    --accent: #6ee7b7;
    --warn: #fbbf24;
    --danger: #f87171;
    --font: ui-sans-serif, system-ui, -apple-system, "Segoe UI", sans-serif;
    --mono: ui-monospace, "SF Mono", Menlo, Consolas, monospace;
  }}
  * {{ box-sizing: border-box; }}
  body {{
    margin: 0; min-height: 100vh; background: radial-gradient(1200px 600px at 10% -10%, #1a2438, var(--bg));
    color: var(--text); font: 14px/1.45 var(--font);
  }}
  header {{
    padding: 20px 24px 12px; border-bottom: 1px solid var(--line);
    display: flex; flex-wrap: wrap; gap: 12px 20px; align-items: baseline; justify-content: space-between;
  }}
  h1 {{ margin: 0; font-size: 18px; font-weight: 600; letter-spacing: -0.02em; }}
  .meta {{ color: var(--muted); font-size: 12px; display: flex; flex-wrap: wrap; gap: 10px; }}
  .mono {{ font-family: var(--mono); }}
  .link {{ color: var(--accent); text-decoration: none; }}
  .link:hover {{ text-decoration: underline; }}
  main {{ padding: 20px 24px 48px; max-width: 920px; margin: 0 auto; display: grid; gap: 18px; }}
  section h2 {{ margin: 0 0 10px; font-size: 12px; text-transform: uppercase; letter-spacing: 0.08em; color: var(--muted); }}
  .card {{
    background: color-mix(in srgb, var(--panel) 92%, transparent); border: 1px solid var(--line);
    border-radius: 12px; padding: 14px 16px; display: grid; gap: 10px;
  }}
  .card h3 {{ margin: 0; font-size: 14px; font-weight: 600; }}
  .sig {{ color: var(--muted); font-size: 12px; font-family: var(--mono); }}
  .fields {{ display: grid; gap: 8px; }}
  label {{ display: grid; gap: 4px; font-size: 12px; color: var(--muted); }}
  input {{
    background: #0c0e13; border: 1px solid var(--line); color: var(--text);
    border-radius: 8px; padding: 8px 10px; font: 13px/1.3 var(--mono);
  }}
  input:focus {{ outline: 1px solid var(--accent); border-color: transparent; }}
  .row {{ display: flex; flex-wrap: wrap; gap: 8px; align-items: center; }}
  button {{
    appearance: none; border: 1px solid var(--line); background: #222836; color: var(--text);
    border-radius: 8px; padding: 8px 12px; font: 13px/1 var(--font); cursor: pointer;
  }}
  button.primary {{ background: #134e3a; border-color: #166534; color: #ecfdf5; }}
  button:hover {{ filter: brightness(1.08); }}
  button:disabled {{ opacity: 0.5; cursor: not-allowed; }}
  .out {{
    min-height: 1.2em; font-family: var(--mono); font-size: 12px; color: var(--accent);
    white-space: pre-wrap; word-break: break-all;
  }}
  .out.err {{ color: var(--danger); }}
  .banner {{
    margin: 0 24px; margin-top: 16px; padding: 10px 12px; border-radius: 10px;
    border: 1px solid var(--line); background: #1a1f2b; color: var(--muted); font-size: 12px;
  }}
  .banner strong {{ color: var(--warn); font-weight: 600; }}
</style>
</head>
<body>
  <header>
    <div>
      <h1>{title}</h1>
      <div class="meta">
        <span>{module}</span>
        <span class="mono">{address}</span>
        <span>chain {chain_id} · {symbol}</span>
        {explorer}
      </div>
    </div>
    <div class="row">
      <button id="btn-wallet" type="button">Connect wallet</button>
      <span id="wallet-status" class="meta mono">not connected</span>
    </div>
  </header>
  <p class="banner"><strong>ProofShip preview</strong> — views use the configured RPC; writes need a browser wallet on this chain. Keys never leave your wallet.</p>
  <script id="abi-json" type="application/json">{abi_raw}</script>
  <main>
    <section>
      <h2>Views</h2>
      <div class="grid" style="display:grid;gap:12px">{view_cards}</div>
    </section>
    <section>
      <h2>Writes</h2>
      <div class="grid" style="display:grid;gap:12px">{entry_cards}</div>
    </section>
  </main>
<script>
const CONFIG = {{
  address: '{addr_js}',
  chainId: {chain_id},
  rpcUrl: '{rpc_js}',
  abi: null
}};
CONFIG.abi = JSON.parse(document.getElementById('abi-json').textContent);

let account = null;

function $(id) {{ return document.getElementById(id); }}

function setOut(el, text, err) {{
  el.textContent = text || '';
  el.className = 'out' + (err ? ' err' : '');
}}

async function rpc(method, params) {{
  const res = await fetch(CONFIG.rpcUrl, {{
    method: 'POST',
    headers: {{ 'content-type': 'application/json' }},
    body: JSON.stringify({{ jsonrpc: '2.0', id: 1, method, params }})
  }});
  const body = await res.json();
  if (body.error) throw new Error(body.error.message || JSON.stringify(body.error));
  return body.result;
}}

function collectArgs(kind, ix, arity) {{
  const args = [];
  for (let i = 0; i < arity; i++) {{
    const input = document.querySelector(`[data-arg="${{kind}}-${{ix}}-${{i}}"]`);
    args.push(input ? input.value.trim() : '');
  }}
  return args;
}}

async function ensureEthers() {{
  if (window.ethers) return window.ethers;
  await new Promise((resolve, reject) => {{
    const s = document.createElement('script');
    s.src = 'https://cdn.jsdelivr.net/npm/ethers@6.13.5/dist/ethers.umd.min.js';
    s.onload = resolve;
    s.onerror = () => reject(new Error('failed to load ethers'));
    document.head.appendChild(s);
  }});
  return window.ethers;
}}

async function readCall(sig, args) {{
  const ethers = await ensureEthers();
  const iface = new ethers.Interface(CONFIG.abi);
  const data = iface.encodeFunctionData(sig.split('(')[0], args);
  const result = await rpc('eth_call', [{{ to: CONFIG.address, data }}, 'latest']);
  const decoded = iface.decodeFunctionResult(sig.split('(')[0], result);
  return decoded.map(v => String(v)).join(', ');
}}

async function writeCall(sig, args) {{
  if (!window.ethereum) throw new Error('No browser wallet (window.ethereum)');
  const ethers = await ensureEthers();
  const provider = new ethers.BrowserProvider(window.ethereum);
  const network = await provider.getNetwork();
  if (Number(network.chainId) !== CONFIG.chainId) {{
    throw new Error(`Wallet on chain ${{Number(network.chainId)}}; preview expects ${{CONFIG.chainId}}`);
  }}
  const signer = await provider.getSigner();
  const iface = new ethers.Interface(CONFIG.abi);
  const data = iface.encodeFunctionData(sig.split('(')[0], args);
  const tx = await signer.sendTransaction({{ to: CONFIG.address, data }});
  return tx.hash;
}}

document.querySelectorAll('[data-action]').forEach(btn => {{
  btn.addEventListener('click', async () => {{
    const kind = btn.getAttribute('data-kind');
    const ix = btn.getAttribute('data-ix');
    const sig = btn.getAttribute('data-sig');
    const arity = Number(btn.getAttribute('data-arity') || '0');
    const out = document.querySelector(`[data-out="${{kind}}-${{ix}}"]`);
    const args = collectArgs(kind, ix, arity);
    btn.disabled = true;
    setOut(out, '…', false);
    try {{
      if (kind === 'view') {{
        const text = await readCall(sig, args);
        setOut(out, text || '(empty)', false);
      }} else {{
        const hash = await writeCall(sig, args);
        setOut(out, hash, false);
      }}
    }} catch (err) {{
      setOut(out, String(err && err.message ? err.message : err), true);
    }} finally {{
      btn.disabled = false;
    }}
  }});
}});

$('btn-wallet').addEventListener('click', async () => {{
  try {{
    if (!window.ethereum) throw new Error('No browser wallet');
    const accounts = await window.ethereum.request({{ method: 'eth_requestAccounts' }});
    account = accounts[0] || null;
    $('wallet-status').textContent = account || 'not connected';
  }} catch (err) {{
    $('wallet-status').textContent = String(err && err.message ? err.message : err);
  }}
}});
</script>
</body>
</html>
"##,
        title = title,
        module = module,
        address = address,
        chain_id = config.chain_id,
        symbol = symbol,
        explorer = explorer,
        view_cards = view_cards,
        entry_cards = entry_cards,
        addr_js = addr_js,
        rpc_js = rpc_js,
        abi_raw = abi_raw,
    )
}

fn fn_card(kind: &str, ix: usize, func: &crate::AbiFormFn) -> String {
    let sig = escape_html(&func.signature());
    let name = if func.name.is_empty() {
        "constructor".to_string()
    } else {
        escape_html(&func.name)
    };
    let mut fields = String::new();
    for (ai, param) in func.inputs.iter().enumerate() {
        let label = if param.name.is_empty() {
            format!("arg{ai} ({})", escape_html(&param.sol_type))
        } else {
            format!(
                "{} ({})",
                escape_html(&param.name),
                escape_html(&param.sol_type)
            )
        };
        fields.push_str(&format!(
            r#"<label>{label}<input data-arg="{kind}-{ix}-{ai}" placeholder="{ph}" spellcheck="false"/></label>"#,
            label = label,
            kind = kind,
            ix = ix,
            ai = ai,
            ph = escape_html(&param.sol_type),
        ));
    }
    let action = if kind == "view" { "Call" } else { "Send" };
    let btn_class = if kind == "view" { "" } else { "primary" };
    format!(
        r#"<article class="card">
  <div>
    <h3>{name}</h3>
    <div class="sig">{sig}</div>
  </div>
  <div class="fields">{fields}</div>
  <div class="row">
    <button class="{btn_class}" type="button" data-action="1" data-kind="{kind}" data-ix="{ix}" data-sig="{sig}" data-arity="{arity}">{action}</button>
    <div class="out" data-out="{kind}-{ix}"></div>
  </div>
</article>"#,
        name = name,
        sig = sig,
        fields = fields,
        btn_class = btn_class,
        kind = kind,
        ix = ix,
        arity = func.inputs.len(),
        action = action,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_html_includes_address_and_views() {
        let abi = include_str!("../tests/fixtures/rwa_share_registry.abi.json");
        let html = render_dapp_html(&DappPreviewConfig {
            title: "RWA Registry".into(),
            module: "RwaShareRegistry".into(),
            address: "0xabc123".into(),
            chain_id: 1952,
            rpc_url: "https://example.invalid".into(),
            explorer_url: Some("https://explorer.example".into()),
            currency_symbol: "OKB".into(),
            abi_json: abi.into(),
        })
        .unwrap();
        assert!(html.contains("0xabc123"));
        assert!(html.contains("totalSupply"));
        assert!(html.contains("ProofShip preview"));
        assert!(html.contains("chain 1952"));
    }
}
