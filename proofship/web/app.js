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
  inviteUrl: document.getElementById("invite-url"),
  claimSession: document.getElementById("claim-session"),
  sendComment: document.getElementById("send-comment"),
  copySession: document.getElementById("copy-session"),
  copyDevice: document.getElementById("copy-device"),
  deviceLine: document.getElementById("device-line"),
  deviceId: document.getElementById("device-id"),
  roomList: document.getElementById("room-list"),
  inviteList: document.getElementById("invite-list"),
  lampDesktop: document.getElementById("lamp-desktop"),
  lampPlatform: document.getElementById("lamp-platform"),
  lampRelay: document.getElementById("lamp-relay"),
  pillDesktop: document.getElementById("pill-desktop"),
  pillPlatform: document.getElementById("pill-platform"),
  pillRelay: document.getElementById("pill-relay"),
};

const DEFAULT_RELAY = "https://proofship-relay.davirain-yin.workers.dev";
const params = new URLSearchParams(location.search);
if (params.get("relay")) els.relay.value = params.get("relay");
else if (els.relay && !els.relay.value.trim()) els.relay.value = DEFAULT_RELAY;
const LAST_SESSION_KEY = "proofship.lastSession";
if (params.get("session") || params.get("launch")) {
  els.session.value = params.get("session") || params.get("launch");
} else {
  try {
    const last = localStorage.getItem(LAST_SESSION_KEY);
    if (last && els.session && !els.session.value.trim()) els.session.value = last;
  } catch {
    /* private mode */
  }
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
  setLamp(els.lampDesktop, false, true);
  setLamp(els.lampPlatform, false, true);
  setLamp(els.lampRelay, false, false);
  setPillText(els.pillDesktop, "share");
  setPillText(els.pillPlatform, "share");
  setPillText(els.pillRelay, "idle");
}

let socket = null;
let lastState = {};
let lastQueueDepth = 0;
let presencePoll = null;
let presenceInFlight = false;
let lastPresenceKey = "";

function selectedExecutor() {
  const el = document.querySelector('input[name="executor"]:checked');
  return el?.value === "platform" ? "platform" : "user";
}

function setStatus(text, kind = "") {
  els.status.textContent = text;
  els.status.className = `status${kind ? ` ${kind}` : ""}`;
}

function setLamp(el, on, unknown = false) {
  if (!el) return;
  el.classList.toggle("on", Boolean(on) && !unknown);
  el.classList.toggle("off", !on && !unknown);
  el.classList.toggle("unknown", Boolean(unknown));
}

function setPillText(el, text) {
  if (el) el.textContent = text;
}

function persistSessionId() {
  const value = els.session?.value.trim();
  if (!value) return;
  try {
    localStorage.setItem(LAST_SESSION_KEY, value);
  } catch {
    /* private mode */
  }
}

function formatAgo(iso) {
  if (!iso) return "";
  const ms = Date.now() - Date.parse(iso);
  if (!Number.isFinite(ms) || ms < 0) return "";
  if (ms < 15_000) return "just now";
  if (ms < 60_000) return `${Math.max(1, Math.round(ms / 1000))}s ago`;
  if (ms < 3_600_000) return `${Math.max(1, Math.round(ms / 60_000))}m ago`;
  if (ms < 86_400_000) return `${Math.max(1, Math.round(ms / 3_600_000))}h ago`;
  return `${Math.max(1, Math.round(ms / 86_400_000))}d ago`;
}

function wsOpen() {
  return Boolean(socket && socket.readyState === WebSocket.OPEN);
}

async function copyText(value, okMessage) {
  const text = String(value || "").trim();
  if (!text) return;
  try {
    await navigator.clipboard.writeText(text);
    setStatus(okMessage, "live");
  } catch {
    setStatus("Clipboard blocked — select the field and copy.", "err");
  }
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
    await refreshRooms();
    await refreshInvites();
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
    const inviteToken = params.get("invite") || "";
    const verifyRes = await fetch(`${base}/api/auth/siwe/verify`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({
        message: nonceBody.message,
        signature,
        inviteToken: inviteToken || undefined,
      }),
    });
    const session = await verifyRes.json();
    if (!verifyRes.ok || !session.token) {
      setAccountStatus(session.error || `Verify failed (${verifyRes.status})`, "err");
      return;
    }
    saveSession(session);
    const joined = (session.joinedOrgs ?? []).map((o) => o.name).filter(Boolean);
    refreshAccountUi();
    if (joined.length) {
      setAccountStatus(`Signed in as ${session.address} · joined ${joined.join(", ")}`, "live");
    }
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
    if (!base || !orgId) {
      setAccountStatus("Pick an org first.", "err");
      return;
    }
    const path = address
      ? `/api/orgs/${encodeURIComponent(orgId)}/members`
      : `/api/orgs/${encodeURIComponent(orgId)}/invites`;
    const res = await fetch(`${base}${path}`, {
      method: "POST",
      headers: { ...(await authHeaders()), "content-type": "application/json" },
      body: JSON.stringify(address ? { address, role: "member" } : { role: "member" }),
    });
    const body = await res.json();
    if (!res.ok) {
      setAccountStatus(body.error || `Invite failed (${res.status})`, "err");
      return;
    }
    if (els.orgInvite) els.orgInvite.value = "";
    if (body.token) {
      const here = new URL(location.href);
      here.searchParams.set("relay", base);
      here.searchParams.set("invite", body.token);
      here.searchParams.delete("share");
      here.searchParams.delete("shareToken");
      const url = here.toString();
      if (els.inviteUrl) els.inviteUrl.value = url;
      setAccountStatus(
        body.joined ? "Already a member. Invite link also minted." : "Invite link ready — send it.",
        "live",
      );
    } else {
      setAccountStatus("Member added.", "live");
    }
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
    if (res.ok) await refreshRooms();
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
if (els.copySession) {
  els.copySession.addEventListener("click", () => {
    void copyText(els.session.value, "Session id copied.");
  });
}
if (els.copyDevice) {
  els.copyDevice.addEventListener("click", () => {
    void copyText(els.deviceId?.textContent, "Device id copied.");
  });
}
if (els.session) {
  els.session.addEventListener("change", () => {
    persistSessionId();
    startPresencePoll();
  });
}
if (els.relay) {
  els.relay.addEventListener("change", () => startPresencePoll());
}
refreshAccountUi();

if (params.get("invite") && !isShareMode) {
  const base = (els.relay.value || DEFAULT_RELAY).replace(/\/$/, "");
  fetch(`${base}/api/invites/${encodeURIComponent(params.get("invite"))}`)
    .then((r) => r.json())
    .then((body) => {
      if (body?.ok) {
        setAccountStatus(
          `Invite to ${body.orgName} as ${body.role}${body.address ? ` for ${body.address}` : ""}. Sign in to join.`,
          "live",
        );
      } else {
        setAccountStatus(body.error || "Invite is invalid or expired.", "err");
      }
    })
    .catch(() => {});
}

function updateExecutorPresence(state) {
  const ex = state?.executors ?? {};
  const connected = wsOpen();
  const userOn = Boolean(ex.userOnline);
  const platOn = Boolean(ex.platformOnline);
  const sessionId = els.session?.value.trim() ?? "";
  const unknown = !isShareMode && !sessionId;
  const device = typeof ex.userDeviceId === "string" ? ex.userDeviceId : "";

  setLamp(els.lampDesktop, userOn, unknown);
  setLamp(els.lampPlatform, platOn, unknown);
  setLamp(els.lampRelay, connected, false);

  const userSeen = formatAgo(ex.userLastSeenAt);
  const platSeen = formatAgo(ex.platformLastSeenAt);
  setPillText(
    els.pillDesktop,
    unknown
      ? "—"
      : userOn
        ? device
          ? `online · ${device}`
          : "online"
        : userSeen
          ? `offline · ${userSeen}`
          : "offline",
  );
  setPillText(
    els.pillPlatform,
    unknown ? "—" : platOn ? "online" : platSeen ? `offline · ${platSeen}` : "offline",
  );
  setPillText(els.pillRelay, connected ? "live" : "idle");

  if (els.deviceLine && els.deviceId) {
    if (device) {
      els.deviceLine.hidden = false;
      els.deviceId.textContent = device;
    } else {
      els.deviceLine.hidden = true;
      els.deviceId.textContent = "";
    }
  }

  const extras = [];
  if (typeof ex.viewerCount === "number") {
    extras.push(`${ex.viewerCount} viewer${ex.viewerCount === 1 ? "" : "s"}`);
  }
  if (lastQueueDepth > 0) extras.push(`${lastQueueDepth} queued`);

  const want = selectedExecutor();
  const online = want === "platform" ? platOn : userOn;
  const cap = shareAccess?.writeCap;
  const canCommand =
    cap === "command" || (!shareAccess && Boolean(connected && online));
  const canComment = cap === "comment" || canCommand;
  const canWrite = Boolean(connected && online && canCommand);
  els.sendPrompt.disabled = !canWrite;
  els.steer.disabled = !canWrite;
  els.cancel.disabled = !canWrite;
  els.deploy.disabled = !(connected && userOn && canCommand);
  if (els.sendComment) els.sendComment.disabled = !canComment;

  let copy;
  let kind = "";
  if (isShareMode) {
    copy = "Read-only share mode — writer WebSocket disabled.";
  } else if (unknown) {
    copy = "Enter the session id from desktop Settings → Networks (desktop-{deviceId}).";
  } else if (connected && online) {
    copy =
      want === "platform"
        ? "Live with Platform. Prompts run in the sandbox — deploy still needs a desktop."
        : `Live with desktop${device ? ` ${device}` : ""}. Prompts go to this machine.`;
    kind = "live";
  } else if (connected && !online) {
    copy =
      want === "platform"
        ? "Relay connected (read-only). Platform is offline — open Sandbox or switch to desktop."
        : "Relay connected (read-only). Desktop is not attached — open ProofShip on that machine.";
    kind = "err";
  } else if (online) {
    copy =
      want === "platform"
        ? "Platform is online. Connect to send sandbox jobs."
        : "Desktop is online. Connect to watch and send prompts.";
    kind = "live";
  } else {
    const seen = want === "platform" ? platSeen : userSeen;
    copy =
      want === "platform"
        ? `Platform is offline${seen ? ` (last seen ${seen})` : ""}.`
        : `Desktop is offline${seen ? ` (last seen ${seen})` : ""}. Open ProofShip on that machine.`;
  }
  if (extras.length) copy += ` · ${extras.join(" · ")}`;
  els.executorStatus.textContent = copy;
  els.executorStatus.className = `status${kind ? ` ${kind}` : ""}`;

  const presenceKey = `${connected}:${online}:${want}`;
  if (presenceKey !== lastPresenceKey) {
    lastPresenceKey = presenceKey;
    if (connected && !online) {
      setStatus(
        want === "platform"
          ? "Connected (read-only) — Platform executor offline. Open Sandbox or switch to desktop."
          : "Connected (read-only) — desktop not attached. Open ProofShip or choose Platform.",
        "err",
      );
    } else if (connected && online) {
      setStatus(`Connected to session ${sessionId}`, "live");
    }
  }
}

function stopPresencePoll() {
  if (presencePoll) {
    clearInterval(presencePoll);
    presencePoll = null;
  }
}

function startPresencePoll() {
  stopPresencePoll();
  if (isShareMode) return;
  void fetchSessionState();
  presencePoll = setInterval(() => void fetchSessionState(), 4000);
}

async function fetchSessionState() {
  if (isShareMode) return;
  const base = els.relay.value.trim().replace(/\/$/, "");
  const sessionId = els.session.value.trim();
  if (!base || !sessionId) {
    updateExecutorPresence(lastState);
    return;
  }
  if (presenceInFlight) return;
  presenceInFlight = true;
  persistSessionId();
  try {
    const url = new URL(`${base}/api/sessions/${encodeURIComponent(sessionId)}/state`);
    const viewer = els.viewerToken.value.trim();
    if (viewer) url.searchParams.set("viewerToken", viewer);
    const res = await fetch(url.toString(), { headers: await authHeaders() });
    if (!res.ok) {
      if (res.status === 401) {
        els.executorStatus.textContent = "Need a viewer token or sign in to read this room.";
        els.executorStatus.className = "status err";
      }
      return;
    }
    const data = await res.json();
    lastQueueDepth = Number(data.queueDepth) || 0;
    const incoming = data.state && typeof data.state === "object" ? data.state : {};
    if (wsOpen()) {
      lastState = { ...lastState, executors: incoming.executors ?? lastState.executors };
      updateExecutorPresence(lastState);
    } else {
      renderSnapshot(incoming);
      if (Array.isArray(incoming.transcript) && incoming.transcript.length) {
        renderTranscriptFromState(incoming);
      }
    }
  } catch {
    if (!wsOpen()) {
      els.executorStatus.textContent = "Relay unreachable — check the base URL.";
      els.executorStatus.className = "status err";
    }
  } finally {
    presenceInFlight = false;
  }
}

async function decorateRoomChip(base, sessionId, button) {
  try {
    const res = await fetch(`${base}/api/sessions/${encodeURIComponent(sessionId)}/state`, {
      headers: await authHeaders(),
    });
    if (!res.ok) return;
    const data = await res.json();
    const on = Boolean(data.state?.executors?.userOnline || data.state?.executors?.platformOnline);
    button.classList.toggle("live", on);
    const lamp = button.querySelector(".lamp");
    setLamp(lamp, on, false);
  } catch {
    /* ignore */
  }
}

async function refreshRooms() {
  if (!els.roomList || isShareMode) return;
  const base = els.relay.value.trim().replace(/\/$/, "");
  const session = loadSession();
  const orgId = els.orgSelect?.value;
  if (!base || !session?.token || !orgId) {
    els.roomList.hidden = true;
    els.roomList.innerHTML = "";
    return;
  }
  try {
    const res = await fetch(`${base}/api/orgs/${encodeURIComponent(orgId)}/rooms`, {
      headers: await authHeaders(),
    });
    const body = await res.json();
    const rooms = body.rooms ?? [];
    if (!res.ok || rooms.length === 0) {
      els.roomList.hidden = true;
      els.roomList.innerHTML = "";
      return;
    }
    els.roomList.hidden = false;
    els.roomList.innerHTML = "";
    const heading = document.createElement("div");
    heading.className = "hint";
    heading.textContent = "Claimed rooms";
    els.roomList.appendChild(heading);
    for (const room of rooms.slice(0, 8)) {
      const button = document.createElement("button");
      button.type = "button";
      button.className = "room-chip ghost";
      button.innerHTML = `<span class="lamp off"></span><span>${escapeHtml(room.sessionId)}</span>`;
      button.addEventListener("click", () => {
        els.session.value = room.sessionId;
        persistSessionId();
        startPresencePoll();
      });
      els.roomList.appendChild(button);
      void decorateRoomChip(base, room.sessionId, button);
    }
  } catch {
    /* relay offline */
  }
}

async function refreshInvites() {
  if (!els.inviteList || isShareMode) return;
  const base = els.relay.value.trim().replace(/\/$/, "");
  const session = loadSession();
  const orgId = els.orgSelect?.value;
  if (!base || !session?.token || !orgId) {
    els.inviteList.hidden = true;
    els.inviteList.innerHTML = "";
    return;
  }
  try {
    const res = await fetch(`${base}/api/orgs/${encodeURIComponent(orgId)}/invites`, {
      headers: await authHeaders(),
    });
    const body = await res.json();
    const invites = body.invites ?? [];
    if (!res.ok || invites.length === 0) {
      els.inviteList.hidden = true;
      els.inviteList.innerHTML = "";
      return;
    }
    els.inviteList.hidden = false;
    els.inviteList.innerHTML = "";
    const heading = document.createElement("div");
    heading.className = "hint";
    heading.textContent = "Pending invites";
    els.inviteList.appendChild(heading);
    for (const invite of invites) {
      const row = document.createElement("div");
      row.className = "invite-row";
      const who = invite.address || "open link";
      const expMs = Date.parse(invite.expiresAt);
      const expired = !Number.isFinite(expMs) || expMs <= Date.now();
      const exp = expired ? "expired" : `expires ${new Date(expMs).toISOString().slice(0, 10)}`;
      row.innerHTML = `<span>${escapeHtml(who)} · ${escapeHtml(invite.role)} · ${escapeHtml(exp)}</span>`;
      const btn = document.createElement("button");
      btn.type = "button";
      btn.className = "ghost tiny";
      btn.textContent = "Revoke";
      if (!invite.tokenHash) continue;
      btn.addEventListener("click", async () => {
        const del = await fetch(
          `${base}/api/orgs/${encodeURIComponent(orgId)}/invites/${encodeURIComponent(invite.tokenHash)}`,
          { method: "DELETE", headers: await authHeaders() },
        );
        const delBody = await del.json().catch(() => ({}));
        setAccountStatus(
          del.ok ? "Invite revoked." : delBody.error || "revoke failed",
          del.ok ? "live" : "err",
        );
        if (del.ok && els.inviteUrl) els.inviteUrl.value = "";
        await refreshInvites();
      });
      row.appendChild(btn);
      els.inviteList.appendChild(row);
    }
  } catch {
    /* relay offline */
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
  lastPresenceKey = "";
  updateExecutorPresence(lastState);
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
  persistSessionId();
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
      lastQueueDepth = Number(msg.queueDepth) || lastQueueDepth;
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
} else if (!isShareMode) {
  startPresencePoll();
}
