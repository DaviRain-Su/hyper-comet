const els = {
  relay: document.getElementById("relay"),
  launch: document.getElementById("launch"),
  connect: document.getElementById("connect"),
  disconnect: document.getElementById("disconnect"),
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
    setStatus("Socket closed.", "err");
  });
  socket.addEventListener("error", () => setStatus("WebSocket error.", "err"));
});
