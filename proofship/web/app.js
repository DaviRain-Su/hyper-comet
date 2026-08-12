const els = {
  relay: document.getElementById("relay"),
  launch: document.getElementById("launch"),
  connect: document.getElementById("connect"),
  disconnect: document.getElementById("disconnect"),
  prompt: document.getElementById("prompt"),
  sendPrompt: document.getElementById("send-prompt"),
  status: document.getElementById("status"),
  snapshot: document.getElementById("snapshot"),
  events: document.getElementById("events"),
};

const params = new URLSearchParams(location.search);
if (params.get("relay")) els.relay.value = params.get("relay");
if (params.get("launch")) els.launch.value = params.get("launch");

let socket = null;

function setStatus(text, kind = "") {
  els.status.textContent = text;
  els.status.className = `status${kind ? ` ${kind}` : ""}`;
}

function renderSnapshot(state) {
  els.snapshot.textContent = JSON.stringify(state ?? {}, null, 2);
}

function renderTail(tail) {
  els.events.innerHTML = "";
  for (const event of tail ?? []) {
    const li = document.createElement("li");
    li.innerHTML = `<strong>#${event.seq ?? "?"} ${event.kind ?? "event"}</strong><br/>${escapeHtml(
      JSON.stringify(event.payload ?? {}),
    )}`;
    els.events.appendChild(li);
  }
}

function escapeHtml(s) {
  return s
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;");
}

function wsUrl(base, launchId) {
  const u = new URL(base);
  u.protocol = u.protocol === "https:" ? "wss:" : "ws:";
  u.pathname = `/ws/web/${encodeURIComponent(launchId)}`;
  u.search = "";
  u.hash = "";
  return u.toString();
}

function disconnect() {
  if (socket) {
    socket.close();
    socket = null;
  }
  els.connect.disabled = false;
  els.disconnect.disabled = true;
  els.sendPrompt.disabled = true;
  setStatus("Disconnected — without a local engine this page is read-only.");
}

els.disconnect.addEventListener("click", disconnect);

els.connect.addEventListener("click", () => {
  const base = els.relay.value.trim().replace(/\/$/, "");
  const launchId = els.launch.value.trim();
  if (!base || !launchId) {
    setStatus("Relay URL and launch id are required.", "err");
    return;
  }
  disconnect();
  setStatus("Connecting…");
  els.connect.disabled = true;

  try {
    socket = new WebSocket(wsUrl(base, launchId));
  } catch (err) {
    setStatus(String(err), "err");
    els.connect.disabled = false;
    return;
  }

  socket.addEventListener("open", () => {
    els.disconnect.disabled = false;
    els.sendPrompt.disabled = false;
    setStatus(`Connected to ${launchId}`, "live");
  });
  socket.addEventListener("message", (ev) => {
    let msg;
    try {
      msg = JSON.parse(ev.data);
    } catch {
      return;
    }
    if (msg.type === "snapshot") {
      renderSnapshot(msg.state);
      renderTail(msg.tail);
    } else if (msg.type === "event" && msg.event) {
      const li = document.createElement("li");
      li.innerHTML = `<strong>#${msg.event.seq ?? "?"} ${msg.event.kind ?? "event"}</strong><br/>${escapeHtml(
        JSON.stringify(msg.event.payload ?? {}),
      )}`;
      els.events.prepend(li);
    }
  });
  socket.addEventListener("close", () => {
    socket = null;
    els.connect.disabled = false;
    els.disconnect.disabled = true;
    els.sendPrompt.disabled = true;
    setStatus("Socket closed.", "err");
  });
  socket.addEventListener("error", () => setStatus("WebSocket error.", "err"));
});

els.sendPrompt.addEventListener("click", () => {
  if (!socket || socket.readyState !== WebSocket.OPEN) {
    setStatus("Connect the viewer first.", "err");
    return;
  }
  const nl = els.prompt.value.trim();
  if (!nl) {
    setStatus("Prompt is empty.", "err");
    return;
  }
  socket.send(JSON.stringify({ type: "cmd.prompt", nl, lane: "codex" }));
  setStatus("Prompt sent to local engine.", "live");
});

// ---- Phase 3.3 interact stub ----
const interact = {
  rpc: document.getElementById("rpc"),
  address: document.getElementById("address"),
  abi: document.getElementById("abi"),
  load: document.getElementById("load-abi"),
  views: document.getElementById("views"),
  status: document.getElementById("call-status"),
};

function setCallStatus(text, err = false) {
  interact.status.textContent = text;
  interact.status.className = `status${err ? " err" : ""}`;
}

async function ensureEthers() {
  if (window.ethers) return window.ethers;
  await new Promise((resolve, reject) => {
    const s = document.createElement("script");
    s.src = "https://cdn.jsdelivr.net/npm/ethers@6.13.5/dist/ethers.umd.min.js";
    s.onload = resolve;
    s.onerror = () => reject(new Error("failed to load ethers"));
    document.head.appendChild(s);
  });
  return window.ethers;
}

interact.load.addEventListener("click", async () => {
  interact.views.innerHTML = "";
  let abi;
  try {
    abi = JSON.parse(interact.abi.value);
  } catch (err) {
    setCallStatus(String(err), true);
    return;
  }
  const views = abi.filter(
    (item) =>
      item.type === "function" &&
      (item.stateMutability === "view" || item.stateMutability === "pure"),
  );
  if (!views.length) {
    setCallStatus("No view/pure functions in ABI.", true);
    return;
  }
  setCallStatus(`${views.length} view(s) loaded`);
  for (const fn of views) {
    const card = document.createElement("div");
    card.className = "view-card";
    const sig = `${fn.name}(${(fn.inputs || []).map((i) => i.type).join(",")})`;
    card.innerHTML = `<strong>${sig}</strong>`;
    const inputs = [];
    for (const [i, input] of (fn.inputs || []).entries()) {
      const field = document.createElement("input");
      field.placeholder = `${input.name || `arg${i}`} (${input.type})`;
      card.appendChild(field);
      inputs.push(field);
    }
    const btn = document.createElement("button");
    btn.type = "button";
    btn.textContent = "eth_call";
    const out = document.createElement("div");
    out.className = "status";
    btn.addEventListener("click", async () => {
      try {
        const ethers = await ensureEthers();
        const iface = new ethers.Interface(abi);
        const args = inputs.map((el) => el.value.trim());
        const data = iface.encodeFunctionData(fn.name, args);
        const res = await fetch(interact.rpc.value.trim(), {
          method: "POST",
          headers: { "content-type": "application/json" },
          body: JSON.stringify({
            jsonrpc: "2.0",
            id: 1,
            method: "eth_call",
            params: [{ to: interact.address.value.trim(), data }, "latest"],
          }),
        });
        const body = await res.json();
        if (body.error) throw new Error(body.error.message || JSON.stringify(body.error));
        const decoded = iface.decodeFunctionResult(fn.name, body.result);
        out.textContent = decoded.map(String).join(", ");
      } catch (err) {
        out.textContent = String(err && err.message ? err.message : err);
        out.className = "status err";
      }
    });
    card.appendChild(btn);
    card.appendChild(out);
    interact.views.appendChild(card);
  }
});
