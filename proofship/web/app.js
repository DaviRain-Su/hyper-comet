import {
  createPublicClient,
  createWalletClient,
  custom,
  http,
  encodeFunctionData,
  decodeFunctionResult,
} from "https://esm.sh/viem@2.23.2";

const els = {
  relay: document.getElementById("relay"),
  session: document.getElementById("session"),
  viewerToken: document.getElementById("viewer-token"),
  connect: document.getElementById("connect"),
  disconnect: document.getElementById("disconnect"),
  cancel: document.getElementById("cancel"),
  prompt: document.getElementById("prompt"),
  sendPrompt: document.getElementById("send-prompt"),
  steer: document.getElementById("steer"),
  status: document.getElementById("status"),
  executorStatus: document.getElementById("executor-status"),
  snapshot: document.getElementById("snapshot"),
  events: document.getElementById("events"),
  transcript: document.getElementById("transcript"),
  deployNetwork: document.getElementById("deploy-network"),
  deployModule: document.getElementById("deploy-module"),
  deployDigest: document.getElementById("deploy-digest"),
  deploy: document.getElementById("deploy"),
  deployStatus: document.getElementById("deploy-status"),
  siweLogin: document.getElementById("siwe-login"),
  siweLogout: document.getElementById("siwe-logout"),
  mintShare: document.getElementById("mint-share"),
  accountStatus: document.getElementById("account-status"),
  shareUrl: document.getElementById("share-url"),
  shareRole: document.getElementById("share-role"),
  orgSelect: document.getElementById("org-select"),
  orgInvite: document.getElementById("org-invite"),
  orgAdd: document.getElementById("org-add"),
  orgMembers: document.getElementById("org-members"),
  claimSession: document.getElementById("claim-session"),
  sendComment: document.getElementById("send-comment"),
};

const params = new URLSearchParams(location.search);
if (params.get("relay")) els.relay.value = params.get("relay");
if (params.get("session") || params.get("launch")) {
  els.session.value = params.get("session") || params.get("launch");
}
const initialToken =
  params.get("shareToken") || params.get("token") || params.get("viewerToken");
if (initialToken) els.viewerToken.value = initialToken;
if (params.get("executor") === "platform") {
  const radio = document.querySelector('input[name="executor"][value="platform"]');
  if (radio) radio.checked = true;
}

const isShareMode =
  params.get("share") === "1" ||
  params.get("share") === "true" ||
  Boolean(params.get("shareToken")) ||
  (Boolean(params.get("token")) && !params.get("viewerToken"));

if (isShareMode) {
  document.body.classList.add("share-mode");
  const badge = document.getElementById("share-badge");
  if (badge) badge.style.display = "inline-block";
  const tag = document.getElementById("top-tag");
  if (tag) {
    tag.textContent =
      "Read-only shared snapshot — observe transcript, gate, artifact, and deployment.";
  }
  const tokenLabel = document.getElementById("token-label");
  if (tokenLabel) tokenLabel.textContent = "Share / Viewer token";
  els.connect.textContent = "Fetch share";
  els.executorStatus.textContent = "Read-only share mode — writer WebSocket disabled.";
  setStatus("Read-only share mode — click Fetch share to load snapshot.");
}

let socket = null;
let lastState = {};

function selectedExecutor() {
  const el = document.querySelector('input[name="executor"]:checked');
  return el?.value === "platform" ? "platform" : "user";
}

function setStatus(text, kind = "") {
  els.status.textContent = text;
  els.status.className = `status${kind ? ` ${kind}` : ""}`;
}

function setDeployStatus(text, kind = "") {
  els.deployStatus.textContent = text;
  els.deployStatus.className = `status${kind ? ` ${kind}` : ""}`;
}

function escapeHtml(s) {
  return String(s)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;");
}

function wsUrl(base, sessionId, viewerToken) {
  const u = new URL(base);
  u.protocol = u.protocol === "https:" ? "wss:" : "ws:";
  u.pathname = `/ws/web/${encodeURIComponent(sessionId)}`;
  const q = new URLSearchParams();
  if (viewerToken) q.set("viewerToken", viewerToken);
  const sessionToken = loadSession()?.token;
  if (sessionToken && !viewerToken) q.set("sessionToken", sessionToken);
  u.search = q.toString() ? `?${q.toString()}` : "";
  u.hash = "";
  return u.toString();
}

const SESSION_KEY = "proofship.siwe.session";

function loadSession() {
  try {
    const raw = sessionStorage.getItem(SESSION_KEY);
    if (!raw) return null;
    const parsed = JSON.parse(raw);
    if (!parsed?.token || !parsed?.address) return null;
    if (parsed.expiresAt && Date.parse(parsed.expiresAt) <= Date.now()) {
      sessionStorage.removeItem(SESSION_KEY);
      return null;
    }
    return parsed;
  } catch {
    return null;
  }
}

function saveSession(session) {
  sessionStorage.setItem(SESSION_KEY, JSON.stringify(session));
}

function clearSession() {
  sessionStorage.removeItem(SESSION_KEY);
}

function setAccountStatus(text, kind = "") {
  if (!els.accountStatus) return;
  els.accountStatus.textContent = text;
  els.accountStatus.className = `status${kind ? ` ${kind}` : ""}`;
}

function refreshAccountUi() {
  const session = loadSession();
  const signedIn = Boolean(session);
  if (els.siweLogin) els.siweLogin.disabled = signedIn || isShareMode;
  if (els.siweLogout) els.siweLogout.disabled = !signedIn;
  if (els.mintShare) els.mintShare.disabled = !signedIn || isShareMode;
  if (els.orgAdd) els.orgAdd.disabled = !signedIn || isShareMode;
  if (els.claimSession) els.claimSession.disabled = !signedIn || isShareMode;
  if (signedIn) {
    setAccountStatus(`Signed in as ${session.address}`, "live");
    void refreshOrgs();
  } else {
    setAccountStatus("Not signed in.");
    if (els.orgSelect) els.orgSelect.innerHTML = "";
    if (els.orgMembers) els.orgMembers.textContent = "";
  }
}

async function authHeaders() {
  const session = loadSession();
  return session?.token ? { authorization: `Bearer ${session.token}` } : {};
}

async function refreshOrgs() {
  const base = els.relay.value.trim().replace(/\/$/, "");
  const session = loadSession();
  if (!base || !session?.token || !els.orgSelect) return;
  try {
    const res = await fetch(`${base}/api/orgs`, { headers: await authHeaders() });
    const body = await res.json();
    if (!res.ok) return;
    els.orgSelect.innerHTML = "";
    for (const org of body.orgs ?? []) {
      const opt = document.createElement("option");
      opt.value = org.id;
      opt.textContent = org.name;
      if (org.id === (body.orgId || session.orgId)) opt.selected = true;
      els.orgSelect.appendChild(opt);
    }
    const orgId = els.orgSelect.value;
    if (orgId) {
      const mem = await fetch(`${base}/api/orgs/${encodeURIComponent(orgId)}/members`, {
        headers: await authHeaders(),
      });
      const memBody = await mem.json();
      if (els.orgMembers) {
        els.orgMembers.textContent = (memBody.members ?? [])
          .map((m) => `${m.address} (${m.role})`)
          .join(" · ");
      }
    }
  } catch {
    /* relay offline */
  }
}

async function siweLogin() {
  const base = els.relay.value.trim().replace(/\/$/, "");
  if (!base) {
    setAccountStatus("Set the relay base URL first.", "err");
    return;
  }
  if (!window.ethereum) {
    setAccountStatus("No window.ethereum — install a browser wallet.", "err");
    return;
  }
  try {
    setAccountStatus("Requesting wallet…");
    const [address] = await window.ethereum.request({ method: "eth_requestAccounts" });
    const chainHex = await window.ethereum.request({ method: "eth_chainId" }).catch(() => "0x7a0");
    const chainId = Number.parseInt(chainHex, 16) || 1952;
    const nonceRes = await fetch(
      `${base}/api/auth/siwe/nonce?address=${encodeURIComponent(address)}&chainId=${chainId}`,
    );
    const nonceBody = await nonceRes.json();
    if (!nonceRes.ok || !nonceBody.message) {
      setAccountStatus(nonceBody.error || `Nonce failed (${nonceRes.status})`, "err");
      return;
    }
    setAccountStatus("Sign the SIWE message in your wallet…");
    const signature = await window.ethereum.request({
      method: "personal_sign",
      params: [nonceBody.message, address],
    });
    const verifyRes = await fetch(`${base}/api/auth/siwe/verify`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ message: nonceBody.message, signature }),
    });
    const session = await verifyRes.json();
    if (!verifyRes.ok || !session.token) {
      setAccountStatus(session.error || `Verify failed (${verifyRes.status})`, "err");
      return;
    }
    saveSession(session);
    refreshAccountUi();
  } catch (err) {
    setAccountStatus(err?.message || String(err), "err");
  }
}

async function siweLogout() {
  const base = els.relay.value.trim().replace(/\/$/, "");
  const session = loadSession();
  if (base && session?.token) {
    await fetch(`${base}/api/auth/logout`, {
      method: "POST",
      headers: { authorization: `Bearer ${session.token}` },
    }).catch(() => {});
  }
  clearSession();
  if (els.shareUrl) els.shareUrl.value = "";
  refreshAccountUi();
}

async function mintShareLink() {
  const base = els.relay.value.trim().replace(/\/$/, "");
  const sessionId = els.session.value.trim();
  const session = loadSession();
  if (!base || !sessionId || !session?.token) {
    setAccountStatus("Sign in and set a session id first.", "err");
    return;
  }
  try {
    const res = await fetch(`${base}/api/sessions/${encodeURIComponent(sessionId)}/share`, {
      method: "POST",
      headers: {
        authorization: `Bearer ${session.token}`,
        "content-type": "application/json",
      },
      body: JSON.stringify({ role: els.shareRole?.value || "readonly" }),
    });
    const body = await res.json();
    if (!res.ok || !body.token) {
      setAccountStatus(body.error || `Mint failed (${res.status})`, "err");
      return;
    }
    const here = new URL(location.href);
    here.searchParams.set("relay", base);
    here.searchParams.set("session", sessionId);
    here.searchParams.set("share", "1");
    here.searchParams.set("shareToken", body.token);
    const url = here.toString();
    if (els.shareUrl) els.shareUrl.value = url;
    setAccountStatus("Read-only share link minted.", "live");
  } catch (err) {
    setAccountStatus(err?.message || String(err), "err");
  }
}

if (els.siweLogin) els.siweLogin.addEventListener("click", () => void siweLogin());
if (els.siweLogout) els.siweLogout.addEventListener("click", () => void siweLogout());
if (els.mintShare) els.mintShare.addEventListener("click", () => void mintShareLink());
if (els.orgAdd) {
  els.orgAdd.addEventListener("click", async () => {
    const base = els.relay.value.trim().replace(/\/$/, "");
    const orgId = els.orgSelect?.value;
    const address = els.orgInvite?.value.trim();
    if (!base || !orgId || !address) {
      setAccountStatus("Pick an org and enter a wallet that has signed in once.", "err");
      return;
    }
    const res = await fetch(`${base}/api/orgs/${encodeURIComponent(orgId)}/members`, {
      method: "POST",
      headers: { ...(await authHeaders()), "content-type": "application/json" },
      body: JSON.stringify({ address, role: "member" }),
    });
    const body = await res.json();
    if (!res.ok) {
      setAccountStatus(body.error || `Invite failed (${res.status})`, "err");
      return;
    }
    if (els.orgInvite) els.orgInvite.value = "";
    setAccountStatus("Member added.", "live");
    await refreshOrgs();
  });
}
if (els.orgSelect) {
  els.orgSelect.addEventListener("change", async () => {
    const base = els.relay.value.trim().replace(/\/$/, "");
    const orgId = els.orgSelect.value;
    if (!base || !orgId) return;
    await fetch(`${base}/api/orgs/${encodeURIComponent(orgId)}/select`, {
      method: "POST",
      headers: await authHeaders(),
    });
    await refreshOrgs();
  });
}
if (els.claimSession) {
  els.claimSession.addEventListener("click", async () => {
    const base = els.relay.value.trim().replace(/\/$/, "");
    const sessionId = els.session.value.trim();
    if (!base || !sessionId) {
      setAccountStatus("Set relay + session id first.", "err");
      return;
    }
    const res = await fetch(`${base}/api/sessions/${encodeURIComponent(sessionId)}/claim`, {
      method: "POST",
      headers: await authHeaders(),
    });
    const body = await res.json();
    setAccountStatus(
      res.ok ? `Claimed for org ${body.grant?.orgId ?? ""}` : body.error || "claim failed",
      res.ok ? "live" : "err",
    );
  });
}

let shareAccess = isShareMode ? { role: "readonly", writeCap: "none" } : null;

function applyShareAccessUi() {
  const cap = shareAccess?.writeCap ?? "none";
  document.body.classList.toggle("share-comment", cap === "comment" || cap === "command");
  document.body.classList.toggle("share-command", cap === "command");
  if (els.sendComment) els.sendComment.disabled = cap !== "comment" && cap !== "command";
  if (shareAccess?.role) {
    setStatus(`Share access: ${shareAccess.role}`, cap === "none" ? "" : "live");
  }
}

async function sendComment() {
  const text = els.prompt?.value.trim();
  if (!text) return;
  if (socket && socket.readyState === WebSocket.OPEN) {
    socket.send(JSON.stringify({ type: "cmd.comment", text }));
    els.prompt.value = "";
    return;
  }
  const base = els.relay.value.trim().replace(/\/$/, "");
  const sessionId = els.session.value.trim();
  const token = params.get("shareToken") || params.get("token") || loadSession()?.token;
  if (!base || !sessionId || !token) {
    setStatus("Need relay, session, and a share/session token to comment.", "err");
    return;
  }
  const res = await fetch(`${base}/api/sessions/${encodeURIComponent(sessionId)}/comments`, {
    method: "POST",
    headers: { authorization: `Bearer ${token}`, "content-type": "application/json" },
    body: JSON.stringify({ text }),
  });
  const body = await res.json();
  if (!res.ok) {
    setStatus(body.error || `Comment failed (${res.status})`, "err");
    return;
  }
  els.prompt.value = "";
  setStatus("Comment posted.", "live");
  if (isShareMode) fetchShare();
}

if (els.sendComment) els.sendComment.addEventListener("click", () => void sendComment());
refreshAccountUi();

function updateExecutorPresence(state) {
  const ex = state?.executors ?? {};
  const user = ex.userOnline ? "online" : "offline";
  const platform = ex.platformOnline ? "online" : "offline";
  const device = ex.userDeviceId ? ` (${ex.userDeviceId})` : "";
  els.executorStatus.textContent = `UserExecutor ${user}${device} · Platform ${platform}`;
  els.executorStatus.className = `status${ex.userOnline || ex.platformOnline ? " live" : ""}`;

  const want = selectedExecutor();
  const online =
    want === "platform" ? Boolean(ex.platformOnline) : Boolean(ex.userOnline);
  const cap = shareAccess?.writeCap;
  const canCommand =
    cap === "command" ||
    (!shareAccess && Boolean(socket && socket.readyState === WebSocket.OPEN && online));
  const canComment = cap === "comment" || canCommand;
  const canWrite = Boolean(socket && socket.readyState === WebSocket.OPEN && online && canCommand);
  els.sendPrompt.disabled = !canWrite;
  els.steer.disabled = !canWrite;
  els.cancel.disabled = !canWrite;
  els.deploy.disabled = !(socket && socket.readyState === WebSocket.OPEN && ex.userOnline && canCommand);
  if (els.sendComment) els.sendComment.disabled = !canComment;

  if (socket?.readyState === WebSocket.OPEN && !online) {
    setStatus(
      want === "platform"
        ? "Connected (read-only) — Platform executor offline. Open Sandbox or switch to desktop."
        : "Connected (read-only) — open desktop ProofShip or choose Platform.",
      "err",
    );
  }
}

function renderSnapshot(state) {
  lastState = state ?? {};
  els.snapshot.textContent = JSON.stringify(lastState, null, 2);
  updateExecutorPresence(lastState);
  maybeAutofillDeploy(lastState);
}

function renderTail(tail) {
  els.events.innerHTML = "";
  for (const event of [...(tail ?? [])].reverse()) {
    appendEventLi(event);
  }
}

function appendEventLi(event) {
  const li = document.createElement("li");
  li.innerHTML = `<strong>#${event.seq ?? "?"} ${escapeHtml(event.kind ?? "event")}</strong><br/>${escapeHtml(
    JSON.stringify(event.payload ?? {}),
  )}`;
  els.events.prepend(li);
  while (els.events.children.length > 80) els.events.lastChild.remove();
}

function transcriptLine(kind, payload) {
  const div = document.createElement("div");
  div.className = `t-line t-${kind.replaceAll(".", "-")}`;
  let body = "";
  if (kind === "session.user") body = payload?.text ?? JSON.stringify(payload);
  else if (kind === "session.agent") body = payload?.text ?? JSON.stringify(payload);
  else if (kind === "session.comment") {
    body = payload?.by ? `${payload.by}: ${payload.text ?? ""}` : (payload?.text ?? JSON.stringify(payload));
  }
  else if (kind === "session.tool") body = JSON.stringify(payload?.call ?? payload);
  else if (kind === "session.done") body = JSON.stringify(payload);
  else body = JSON.stringify(payload);
  div.innerHTML = `<span class="t-kind">${escapeHtml(kind)}</span><pre>${escapeHtml(body)}</pre>`;
  return div;
}

function renderTranscriptFromState(state) {
  els.transcript.innerHTML = "";
  for (const row of state?.transcript ?? []) {
    els.transcript.appendChild(transcriptLine(row.kind, row.payload));
  }
  els.transcript.scrollTop = els.transcript.scrollHeight;
}

function appendTranscriptEvent(event) {
  if (!event?.kind?.startsWith("session.")) return;
  els.transcript.appendChild(transcriptLine(event.kind, event.payload));
  els.transcript.scrollTop = els.transcript.scrollHeight;
}

function maybeAutofillDeploy(state) {
  const draft = state?.draft;
  const gate = state?.gate;
  const artifact = state?.artifact;
  if (draft?.module && !els.deployModule.value) els.deployModule.value = draft.module;
  const digest =
    artifact?.outputSetDigest ??
    gate?.digests?.outputSetDigest ??
    gate?.output;
  if (typeof digest === "string" && digest && !els.deployDigest.value) {
    els.deployDigest.value = digest;
  }
  const addr = state?.deployment?.record?.address ?? state?.deployment?.address;
  if (typeof addr === "string" && addr.startsWith("0x") && !interact.address.value) {
    interact.address.value = addr;
  }
}

function setConnectedUi(connected) {
  els.connect.disabled = connected;
  els.disconnect.disabled = !connected;
  if (!connected) {
    els.sendPrompt.disabled = true;
    els.steer.disabled = true;
    els.cancel.disabled = true;
    els.deploy.disabled = true;
  }
}

function disconnect() {
  if (socket) {
    socket.close();
    socket = null;
  }
  setConnectedUi(false);
  setStatus("Disconnected — open desktop ProofShip or choose Platform.");
  updateExecutorPresence({});
}

function sendCommand(payload) {
  if (!socket || socket.readyState !== WebSocket.OPEN) {
    setStatus("Connect first.", "err");
    return false;
  }
  socket.send(JSON.stringify(payload));
  return true;
}

async function fetchShare() {
  const base = els.relay.value.trim().replace(/\/$/, "");
  const sessionId = els.session.value.trim();
  const token = els.viewerToken.value.trim();
  if (!base || !sessionId) {
    setStatus("Relay base URL and session id are required.", "err");
    return;
  }

  setStatus("Fetching shared snapshot…");
  els.connect.disabled = true;

  try {
    const url = new URL(`${base}/api/share/${encodeURIComponent(sessionId)}`);
    if (token) {
      url.searchParams.set("token", token);
      url.searchParams.set("viewerToken", token);
    }
    const res = await fetch(url.toString());
    els.connect.disabled = false;

    if (!res.ok) {
      if (res.status === 401) {
        setStatus("Unauthorized — invalid share token.", "err");
      } else if (res.status === 404) {
        setStatus(`Session room "${sessionId}" not found.`, "err");
      } else {
        setStatus(`Relay returned status ${res.status}.`, "err");
      }
      return;
    }

    const data = await res.json();
    if (!data || typeof data !== "object") {
      setStatus("Invalid share response from relay.", "err");
      return;
    }

    shareAccess = data.access ?? { role: "readonly", writeCap: "none" };
    applyShareAccessUi();

    const share = data.share ?? {};
    renderSnapshot({
      readonly: true,
      sessionId: data.sessionId ?? sessionId,
      gate: share.gate ?? null,
      artifact: share.artifact ?? null,
      deployment: share.deployment ?? null,
      notes: share.notes ?? [],
    });

    renderTail(data.tail ?? []);
    renderTranscriptFromShare(share.transcript ?? []);

    const isEmpty =
      (!share.transcript || share.transcript.length === 0) &&
      !share.gate &&
      !share.artifact &&
      !share.deployment &&
      (!data.tail || data.tail.length === 0);

    if (isEmpty) {
      setStatus(`Connected to relay — empty room for session ${sessionId}.`, "live");
    } else {
      setStatus(`Loaded shared snapshot for session ${sessionId}.`, "live");
    }
  } catch (err) {
    els.connect.disabled = false;
    setStatus(`Offline relay — could not fetch share: ${err.message || err}`, "err");
  }
}

function renderTranscriptFromShare(transcript) {
  els.transcript.innerHTML = "";
  if (!Array.isArray(transcript) || transcript.length === 0) {
    const emptyDiv = document.createElement("div");
    emptyDiv.className = "status";
    emptyDiv.textContent = "(Empty transcript)";
    els.transcript.appendChild(emptyDiv);
    return;
  }
  for (const row of transcript) {
    els.transcript.appendChild(transcriptLine(row.kind ?? "event", row.payload ?? row));
  }
  els.transcript.scrollTop = els.transcript.scrollHeight;
}

els.disconnect.addEventListener("click", disconnect);

for (const radio of document.querySelectorAll('input[name="executor"]')) {
  radio.addEventListener("change", () => updateExecutorPresence(lastState));
}

els.connect.addEventListener("click", () => {
  if (isShareMode) {
    fetchShare();
    return;
  }
  const base = els.relay.value.trim().replace(/\/$/, "");
  const sessionId = els.session.value.trim();
  const viewerToken = els.viewerToken.value.trim();
  if (!base || !sessionId) {
    setStatus("Relay URL and session id are required.", "err");
    return;
  }
  disconnect();
  setStatus("Connecting…");
  els.connect.disabled = true;

  try {
    socket = new WebSocket(wsUrl(base, sessionId, viewerToken));
  } catch (err) {
    setStatus(String(err), "err");
    els.connect.disabled = false;
    return;
  }

  socket.addEventListener("open", () => {
    setConnectedUi(true);
    setStatus(`Connected to session ${sessionId}`, "live");
    updateExecutorPresence(lastState);
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
      renderTranscriptFromState(msg.state);
    } else if (msg.type === "event" && msg.event) {
      appendEventLi(msg.event);
      appendTranscriptEvent(msg.event);
      if (msg.event.kind?.startsWith("executor.") || msg.event.kind === "deploy.done") {
        const patch = { ...lastState };
        if (msg.event.kind === "executor.online" || msg.event.kind === "executor.offline") {
          const role = msg.event.payload?.role;
          const online = msg.event.kind === "executor.online";
          patch.executors = { ...(patch.executors ?? {}) };
          if (role === "platform") patch.executors.platformOnline = online;
          else {
            patch.executors.userOnline = online;
            if (msg.event.payload?.deviceId) {
              patch.executors.userDeviceId = msg.event.payload.deviceId;
            }
          }
        }
        if (msg.event.kind === "deploy.done") {
          patch.deployment = msg.event.payload;
          if (msg.event.payload?.ok) setDeployStatus("Deploy succeeded on UserExecutor.", "live");
          else setDeployStatus(msg.event.payload?.error || "Deploy failed.", "err");
        }
        if (msg.event.kind === "executor.refused") {
          setDeployStatus(msg.event.payload?.hint || msg.event.payload?.reason || "Refused", "err");
        }
        renderSnapshot(patch);
      } else if (
        msg.event.kind === "draft.ready" ||
        msg.event.kind === "gate.done" ||
        msg.event.kind === "artifact.sealed"
      ) {
        const patch = { ...lastState };
        if (msg.event.kind === "draft.ready") patch.draft = msg.event.payload;
        if (msg.event.kind === "gate.done") patch.gate = msg.event.payload;
        if (msg.event.kind === "artifact.sealed") patch.artifact = msg.event.payload;
        renderSnapshot(patch);
      }
    } else if (msg.type === "error") {
      setStatus(msg.error || "relay error", "err");
    }
  });
  socket.addEventListener("close", () => {
    socket = null;
    setConnectedUi(false);
    setStatus("Socket closed.", "err");
  });
  socket.addEventListener("error", () => setStatus("WebSocket error.", "err"));
});

els.sendPrompt.addEventListener("click", () => {
  const nl = els.prompt.value.trim();
  if (!nl) {
    setStatus("Prompt is empty.", "err");
    return;
  }
  if (
    sendCommand({
      type: "cmd.prompt",
      nl,
      lane: "codex",
      executor: selectedExecutor(),
    })
  ) {
    setStatus(`Prompt queued for ${selectedExecutor()} executor.`, "live");
    els.prompt.value = "";
  }
});

els.steer.addEventListener("click", () => {
  const nl = els.prompt.value.trim();
  if (!nl) {
    setStatus("Steer text is empty.", "err");
    return;
  }
  if (sendCommand({ type: "cmd.steer", nl })) {
    setStatus("Steer sent.", "live");
  }
});

els.cancel.addEventListener("click", () => {
  if (sendCommand({ type: "cmd.cancel" })) setStatus("Cancel sent.", "live");
});

els.deploy.addEventListener("click", () => {
  const networkId = els.deployNetwork.value.trim();
  const module = els.deployModule.value.trim();
  const digest = els.deployDigest.value.trim();
  if (!networkId || !module) {
    setDeployStatus("network id and module are required.", "err");
    return;
  }
  const payload = {
    type: "cmd.deploy",
    networkId,
    module,
    executor: "user",
  };
  if (digest) payload.digest = digest;
  if (sendCommand(payload)) {
    setDeployStatus("Deploy command queued for UserExecutor…", "live");
  }
});

// ---- Interact (viem + window.ethereum) ----
const interact = {
  rpc: document.getElementById("rpc"),
  address: document.getElementById("address"),
  abi: document.getElementById("abi"),
  load: document.getElementById("load-abi"),
  fill: document.getElementById("fill-snapshot"),
  views: document.getElementById("views"),
  status: document.getElementById("call-status"),
};

function setCallStatus(text, err = false) {
  interact.status.textContent = text;
  interact.status.className = `status${err ? " err" : ""}`;
}

function parseAbi() {
  return JSON.parse(interact.abi.value);
}

function fillFromSnapshot() {
  const art = lastState?.artifact;
  const dep = lastState?.deployment;
  const addr =
    dep?.record?.address ??
    dep?.address ??
    art?.address ??
    lastState?.draft?.address;
  if (typeof addr === "string") interact.address.value = addr;

  const abi =
    art?.abi ??
    art?.abiJson ??
    (typeof art?.abiText === "string" ? JSON.parse(art.abiText) : null);
  if (abi) {
    interact.abi.value = JSON.stringify(abi, null, 2);
  }
  if (art?.rpcUrl && !interact.rpc.value) interact.rpc.value = art.rpcUrl;
  setCallStatus(
    abi || addr
      ? "Filled fields from snapshot (paste ABI if still empty)."
      : "Snapshot has no sealed ABI/address yet — paste manually.",
    !(abi || addr),
  );
}

interact.fill.addEventListener("click", () => {
  try {
    fillFromSnapshot();
  } catch (err) {
    setCallStatus(String(err), true);
  }
});

async function ethCall(rpcUrl, to, data) {
  const client = createPublicClient({ transport: http(rpcUrl) });
  return client.call({ to, data });
}

async function walletWrite(to, data) {
  if (!window.ethereum) {
    throw new Error("No window.ethereum — connect a browser wallet or use WalletConnect later");
  }
  const [account] = await window.ethereum.request({ method: "eth_requestAccounts" });
  const wallet = createWalletClient({
    account,
    transport: custom(window.ethereum),
  });
  return wallet.sendTransaction({ account, to, data });
}

interact.load.addEventListener("click", async () => {
  interact.views.innerHTML = "";
  let abi;
  try {
    abi = parseAbi();
  } catch (err) {
    setCallStatus(String(err), true);
    return;
  }
  if (!Array.isArray(abi)) {
    setCallStatus("ABI must be a JSON array.", true);
    return;
  }
  const fns = abi.filter((item) => item.type === "function");
  if (!fns.length) {
    setCallStatus("No functions in ABI.", true);
    return;
  }
  setCallStatus(`${fns.length} function(s) loaded`);
  for (const fn of fns) {
    const card = document.createElement("div");
    card.className = "view-card";
    const mut = fn.stateMutability || "nonpayable";
    const isView = mut === "view" || mut === "pure";
    const sig = `${fn.name}(${(fn.inputs || []).map((i) => i.type).join(",")})`;
    card.innerHTML = `<strong>${escapeHtml(sig)}</strong> <span class="hint">${escapeHtml(mut)}</span>`;
    const inputs = [];
    for (const [i, input] of (fn.inputs || []).entries()) {
      const field = document.createElement("input");
      field.placeholder = `${input.name || `arg${i}`} (${input.type})`;
      card.appendChild(field);
      inputs.push(field);
    }
    const btn = document.createElement("button");
    btn.type = "button";
    btn.textContent = isView ? "eth_call" : "send (wallet)";
    const out = document.createElement("div");
    out.className = "status";
    btn.addEventListener("click", async () => {
      try {
        const args = inputs.map((el) => el.value.trim());
        const data = encodeFunctionData({ abi, functionName: fn.name, args });
        const to = interact.address.value.trim();
        if (!to) throw new Error("contract address required");
        if (isView) {
          const rpc = interact.rpc.value.trim();
          if (!rpc) throw new Error("RPC URL required for views");
          const res = await ethCall(rpc, to, data);
          if (!res.data) {
            out.textContent = "(empty)";
            return;
          }
          const decoded = decodeFunctionResult({
            abi,
            functionName: fn.name,
            data: res.data,
          });
          out.textContent = Array.isArray(decoded)
            ? decoded.map(String).join(", ")
            : String(decoded);
          out.className = "status live";
        } else {
          const hash = await walletWrite(to, data);
          out.textContent = `tx ${hash}`;
          out.className = "status live";
        }
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

// ---- ProofForge MCP (web agents) ----
const DEFAULT_PF_MCP = "https://proof-forge-mcp.davirain-yin.workers.dev/mcp";
const pfMcp = {
  input: document.getElementById("pf-mcp"),
  copy: document.getElementById("copy-pf-mcp"),
  health: document.getElementById("pf-mcp-health"),
  snippet: document.getElementById("pf-mcp-snippet"),
  status: document.getElementById("pf-mcp-status"),
};

function refreshPfMcpUi() {
  const url = (pfMcp.input.value || DEFAULT_PF_MCP).trim().replace(/\/$/, "");
  pfMcp.input.value = url;
  const health = url.replace(/\/mcp\/?$/, "") + "/health";
  pfMcp.health.href = health;
  pfMcp.snippet.textContent = [
    "# Codex",
    `codex mcp add proof-forge-mcp --url ${url}`,
    "",
    "# Claude Code",
    `claude mcp add --transport http proof-forge-mcp ${url}`,
    "",
    "# Cursor / generic (mcp-remote)",
    `npx -y mcp-remote ${url}`,
  ].join("\n");
}

if (params.get("pfMcp")) pfMcp.input.value = params.get("pfMcp");
else pfMcp.input.value = DEFAULT_PF_MCP;
refreshPfMcpUi();
pfMcp.input.addEventListener("change", refreshPfMcpUi);
pfMcp.input.addEventListener("input", refreshPfMcpUi);
pfMcp.copy.addEventListener("click", async () => {
  try {
    await navigator.clipboard.writeText(pfMcp.input.value.trim());
    pfMcp.status.textContent = "Copied MCP URL.";
    pfMcp.status.className = "status live";
  } catch (err) {
    pfMcp.status.textContent = String(err);
    pfMcp.status.className = "status err";
  }
});

if (isShareMode && els.relay.value.trim() && els.session.value.trim()) {
  fetchShare();
}
