/**
 * ProofShip Relay (W1+) — coordinate Sessions-shaped rooms between web viewers
 * and executors (user desktop/VPS or platform sandbox).
 *
 * Room key: sessionId (alias: launchId for R0 URLs).
 * Engine auth: per-device token via DEVICE_TOKENS JSON map or DEVICE_TOKEN / ENGINE_TOKEN.
 * Viewer auth: optional VIEWER_TOKEN query param when set.
 * Keys never transit this Worker.
 */
import { DurableObject } from "cloudflare:workers";
import {
  accessFor,
  getAccountStore,
  handleAuth,
  shareAllowed,
  viewerAllowed,
  withCors,
} from "./auth";
import { commandAllowedForCap, type WriteCap } from "./policy";
import {
  PLATFORM_DEPLOY_REFUSAL,
  authorizeEngine,
  authorizeShare,
  authorizeViewer,
  eventStatePatch,
  isRecord,
  overlayLiveExecutors,
  parseViewerCommand,
  redactSharePayload,
  resolveExecutor,
  shouldRefusePlatformDeploy,
  type CommandMessage,
  type EngineEventMessage,
  type SessionState,
  type StoredEvent,
} from "./contract";

// Re-export contract helpers for consumers / tests that import the Worker entry.
export {
  PLATFORM_DEPLOY_REFUSAL,
  authorizeEngine,
  authorizeShare,
  authorizeViewer,
  eventStatePatch,
  overlayLiveExecutors,
  parseViewerCommand,
  redactSharePayload,
  resolveExecutor,
  shouldRefusePlatformDeploy,
} from "./contract";

export interface Env {
  SESSION_ROOM: DurableObjectNamespace<SessionRoom>;
  /** Shared fallback (R0). Prefer DEVICE_TOKENS. */
  ENGINE_TOKEN?: string;
  DEVICE_TOKEN?: string;
  /** JSON object: { "device-id": "token", ... } */
  DEVICE_TOKENS?: string;
  /** When set, viewers must pass ?viewerToken= */
  VIEWER_TOKEN?: string;
  /**
   * Optional read-only share token (Phase 4.4 stub).
   * When set, `GET /api/share/:sessionId?token=` must match.
   * When unset, share uses the same auth as viewer endpoints.
   */
  SHARE_TOKEN?: string;
  /** Optional D1 accounts database. Memory store is used when unset. */
  DB?: D1Database;
  /** Host the SIWE message must name (defaults to Origin / request host). */
  SIWE_DOMAIN?: string;
  SIWE_URI?: string;
}

type Role = "engine" | "viewer" | "platform";

interface QueuedCommand {
  id: string;
  ts: string;
  expiresAt: string;
  command: CommandMessage;
}

interface SocketAttachment {
  role: Role;
  deviceId?: string;
  writeCap?: WriteCap;
}

const MAX_EVENTS = 500;
const SNAPSHOT_TAIL = 80;
const COMMAND_TTL_MS = 15 * 60 * 1000;

function json(data: unknown, init: ResponseInit = {}): Response {
  return new Response(JSON.stringify(data), {
    ...init,
    headers: {
      "content-type": "application/json; charset=utf-8",
      ...init.headers,
    },
  });
}

function badRequest(message: string): Response {
  return json({ ok: false, error: message }, { status: 400 });
}

function unauthorized(message = "unauthorized"): Response {
  return json({ ok: false, error: message }, { status: 401 });
}

function notFound(): Response {
  return json({ ok: false, error: "not found" }, { status: 404 });
}

function extractRoomId(pathname: string, prefix: string): string | null {
  if (!pathname.startsWith(prefix)) return null;
  const rest = pathname.slice(prefix.length);
  if (rest.length === 0 || rest.includes("/")) return null;
  try {
    return decodeURIComponent(rest);
  } catch {
    return null;
  }
}

function isUpgrade(request: Request): boolean {
  return request.headers.get("Upgrade")?.toLowerCase() === "websocket";
}

function parseEngineEvent(raw: unknown): EngineEventMessage | null {
  if (!isRecord(raw) || raw.type !== "event" || typeof raw.kind !== "string") {
    return null;
  }
  return { type: "event", kind: raw.kind, payload: raw.payload };
}

function parseJsonMessage(message: string | ArrayBuffer): unknown | null {
  if (typeof message !== "string") return null;
  try {
    return JSON.parse(message) as unknown;
  } catch {
    return null;
  }
}

function sendJson(ws: WebSocket, data: unknown): void {
  try {
    ws.send(JSON.stringify(data));
  } catch {
    ws.close(1011, "send failed");
  }
}

function newId(): string {
  return crypto.randomUUID();
}

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const url = new URL(request.url);

    if (request.method === "GET" && url.pathname === "/health") {
      return withCors(
        request,
        json({
          ok: true,
          contract: "proofship-relay-w1-accounts",
          dualExecutor: true,
          accounts: true,
          siwe: true,
          orgs: true,
          shareRoles: ["readonly", "comment", "command"],
        }),
      );
    }

    const store = getAccountStore(env);
    const authResponse = await handleAuth(request, env, store);
    if (authResponse) return withCors(request, authResponse);

    // Prefer /ws/engine/:sessionId; keep /ws/engine/:launchId alias.
    for (const prefix of ["/ws/engine/", "/ws/session/engine/"]) {
      const roomId = extractRoomId(url.pathname, prefix);
      if (request.method === "GET" && roomId !== null) {
        if (!isUpgrade(request)) return badRequest("expected WebSocket upgrade");
        const auth = authorizeEngine(env, url);
        if (!auth.ok) return unauthorized("invalid device token");
        const roleParam = url.searchParams.get("role");
        const role: Role = roleParam === "platform" ? "platform" : "engine";
        return forwardToRoom(request, env, roomId, role, auth.deviceId);
      }
    }

    for (const prefix of ["/ws/web/", "/ws/session/web/"]) {
      const roomId = extractRoomId(url.pathname, prefix);
      if (request.method === "GET" && roomId !== null) {
        if (!isUpgrade(request)) return badRequest("expected WebSocket upgrade");
        const access = await accessFor(
          request,
          url,
          store,
          roomId,
          Date.now(),
          authorizeViewer(env, url),
        );
        if (!access.ok) return unauthorized("invalid viewer token");
        return forwardToRoom(request, env, roomId, "viewer", undefined, access.writeCap);
      }
    }

    const stateMatch =
      url.pathname.match(/^\/api\/sessions\/([^/]+)\/state$/u) ??
      url.pathname.match(/^\/api\/launches\/([^/]+)\/state$/u);
    if (request.method === "GET" && stateMatch !== null) {
      const roomId = decodeURIComponent(stateMatch[1] ?? "");
      if (!roomId) return badRequest("missing session id");
      if (
        !(await viewerAllowed(request, url, store, roomId, Date.now(), (u) =>
          authorizeViewer(env, u),
        ))
      ) {
        return unauthorized("invalid viewer token");
      }
      const id = env.SESSION_ROOM.idFromName(roomId);
      const room = env.SESSION_ROOM.get(id);
      return withCors(
        request,
        await room.fetch(new Request(new URL("/state", request.url), { method: "GET" })),
      );
    }

    // Read-only share: minted SIWE share token, or SHARE_TOKEN / viewer fallback.
    const shareMatch = url.pathname.match(/^\/api\/share\/([^/]+)$/u);
    if (request.method === "GET" && shareMatch !== null) {
      const roomId = decodeURIComponent(shareMatch[1] ?? "");
      if (!roomId) return badRequest("missing session id");
      if (
        !(await shareAllowed(request, url, store, roomId, Date.now(), (u) =>
          authorizeShare(env, u),
        ))
      ) {
        return unauthorized("invalid share token");
      }
      const access = await accessFor(
        request,
        url,
        store,
        roomId,
        Date.now(),
        authorizeShare(env, url),
      );
      const id = env.SESSION_ROOM.idFromName(roomId);
      const room = env.SESSION_ROOM.get(id);
      const shared = await room.fetch(new Request(new URL("/share", request.url), { method: "GET" }));
      const payload = (await shared.json()) as Record<string, unknown>;
      payload.access = { role: access.role ?? "readonly", writeCap: access.writeCap };
      return withCors(request, json(payload));
    }

    const commentMatch = url.pathname.match(/^\/api\/sessions\/([^/]+)\/comments$/u);
    if (request.method === "POST" && commentMatch) {
      const roomId = decodeURIComponent(commentMatch[1] ?? "");
      if (!roomId) return badRequest("missing session id");
      const access = await accessFor(
        request,
        url,
        store,
        roomId,
        Date.now(),
        authorizeViewer(env, url),
      );
      if (!access.ok || !commandAllowedForCap(access.writeCap, "cmd.comment")) {
        return unauthorized("comment role required");
      }
      let text = "";
      try {
        const body = (await request.json()) as { text?: unknown };
        if (typeof body.text === "string") text = body.text.trim();
      } catch {
        return badRequest("invalid json");
      }
      if (!text) return badRequest("text required");
      const id = env.SESSION_ROOM.idFromName(roomId);
      const room = env.SESSION_ROOM.get(id);
      return withCors(
        request,
        await room.fetch(
          new Request(new URL("/comment", request.url), {
            method: "POST",
            headers: { "content-type": "application/json" },
            body: JSON.stringify({ text, by: access.address ?? "share" }),
          }),
        ),
      );
    }

    return notFound();
  },
};

async function forwardToRoom(
  request: Request,
  env: Env,
  roomId: string,
  role: Role,
  deviceId?: string,
  writeCap?: WriteCap,
): Promise<Response> {
  if (!roomId) return badRequest("missing session id");
  const id = env.SESSION_ROOM.idFromName(roomId);
  const room = env.SESSION_ROOM.get(id);
  const url = new URL(request.url);
  url.pathname = "/ws";
  const q = new URLSearchParams({ role });
  if (deviceId) q.set("deviceId", deviceId);
  if (writeCap) q.set("writeCap", writeCap);
  url.search = `?${q.toString()}`;
  return room.fetch(new Request(url, request));
}

export class SessionRoom extends DurableObject<Env> {
  private loaded = false;
  private events: StoredEvent[] = [];
  private state: SessionState = {};
  private queue: QueuedCommand[] = [];
  private nextSeq = 1;
  private sessionId = "";

  async fetch(request: Request): Promise<Response> {
    await this.ensureLoaded();
    const url = new URL(request.url);

    if (request.method === "GET" && url.pathname === "/state") {
      const state = this.liveState();
      return json({
        state,
        tail: this.events.slice(-SNAPSHOT_TAIL),
        queueDepth: this.queue.length,
        presence: state.executors ?? {},
      });
    }

    if (request.method === "GET" && url.pathname === "/share") {
      return json(redactSharePayload(this.state, this.events, this.sessionId, SNAPSHOT_TAIL));
    }

    if (request.method === "POST" && url.pathname === "/comment") {
      let text = "";
      let by = "share";
      try {
        const body = (await request.json()) as { text?: unknown; by?: unknown };
        if (typeof body.text === "string") text = body.text.trim();
        if (typeof body.by === "string" && body.by.trim()) by = body.by.trim();
      } catch {
        return badRequest("invalid json");
      }
      if (!text) return badRequest("text required");
      await this.appendEvent({
        type: "event",
        kind: "session.comment",
        payload: { text, by },
      });
      return json({ ok: true });
    }

    if (request.method === "GET" && url.pathname === "/ws") {
      if (!isUpgrade(request)) return badRequest("expected WebSocket upgrade");
      const role = url.searchParams.get("role");
      if (role !== "engine" && role !== "viewer" && role !== "platform") {
        return badRequest("invalid role");
      }
      const deviceId = url.searchParams.get("deviceId") ?? undefined;
      const writeRaw = url.searchParams.get("writeCap");
      const writeCap: WriteCap =
        writeRaw === "none" || writeRaw === "comment" || writeRaw === "command"
          ? writeRaw
          : "command";

      const pair = new WebSocketPair();
      const [client, server] = Object.values(pair) as [WebSocket, WebSocket];
      server.serializeAttachment({ role, deviceId, writeCap } satisfies SocketAttachment);

      if (role === "engine" || role === "platform") {
        for (const socket of this.socketsFor(role)) {
          socket.close(1012, `${role} replaced`);
        }
      }

      this.ctx.acceptWebSocket(server);

      if (role === "viewer") {
        sendJson(server, {
          type: "snapshot",
          state: this.liveState(),
          tail: this.events.slice(-SNAPSHOT_TAIL),
          queueDepth: this.queue.length,
        });
      } else {
        await this.appendEvent({
          type: "event",
          kind: "executor.online",
          payload: { role, deviceId },
        });
        await this.drainQueueToExecutors();
      }

      return new Response(null, { status: 101, webSocket: client });
    }

    return notFound();
  }

  async webSocketMessage(ws: WebSocket, message: string | ArrayBuffer): Promise<void> {
    await this.ensureLoaded();
    const attachment = this.attachmentFor(ws);
    const parsed = parseJsonMessage(message);

    if (attachment.role === "engine" || attachment.role === "platform") {
      if (isRecord(parsed) && parsed.type === "cmd.ack" && typeof parsed.id === "string") {
        this.queue = this.queue.filter((q) => q.id !== parsed.id);
        await this.ctx.storage.put({ queue: this.queue });
        return;
      }
      const engineEvent = parseEngineEvent(parsed);
      if (engineEvent === null) {
        sendJson(ws, { type: "error", error: "invalid engine event" });
        return;
      }
      await this.appendEvent(engineEvent);
      return;
    }

    const command = parseViewerCommand(parsed);
    if (command === null) {
      sendJson(ws, { type: "error", error: "invalid viewer command" });
      return;
    }
    const cap = attachment.writeCap ?? "command";
    if (!commandAllowedForCap(cap, command.type)) {
      sendJson(ws, { type: "error", error: "share role cannot send that command" });
      return;
    }
    if (command.type === "cmd.comment") {
      await this.appendEvent({
        type: "event",
        kind: "session.comment",
        payload: { text: command.text },
      });
      return;
    }
    await this.enqueueCommand(command);
  }

  webSocketClose(ws: WebSocket): void {
    const attachment = this.attachmentFor(ws);
    if (attachment.role === "engine" || attachment.role === "platform") {
      void this.appendEvent({
        type: "event",
        kind: "executor.offline",
        payload: { role: attachment.role, deviceId: attachment.deviceId },
      });
    }
  }

  webSocketError(ws: WebSocket, error: unknown): void {
    void error;
    ws.close(1011, "websocket error");
  }

  private async ensureLoaded(): Promise<void> {
    if (this.loaded) return;
    const [events, state, queue, nextSeq, sessionId] = await Promise.all([
      this.ctx.storage.get<StoredEvent[]>("events"),
      this.ctx.storage.get<SessionState>("state"),
      this.ctx.storage.get<QueuedCommand[]>("queue"),
      this.ctx.storage.get<number>("nextSeq"),
      this.ctx.storage.get<string>("sessionId"),
    ]);
    this.events = events ?? [];
    this.state = state ?? {};
    this.queue = (queue ?? []).filter((q) => Date.parse(q.expiresAt) > Date.now());
    this.nextSeq = nextSeq ?? (this.events.at(-1)?.seq ?? 0) + 1;
    this.sessionId = sessionId ?? "";
    this.loaded = true;
  }

  private attachmentFor(ws: WebSocket): SocketAttachment {
    const attachment = ws.deserializeAttachment() as Partial<SocketAttachment> | undefined;
    if (
      attachment?.role === "engine" ||
      attachment?.role === "viewer" ||
      attachment?.role === "platform"
    ) {
      return {
        role: attachment.role,
        deviceId: attachment.deviceId,
        writeCap: attachment.writeCap,
      };
    }
    return { role: "viewer" };
  }

  private socketsFor(role: Role): WebSocket[] {
    return this.ctx.getWebSockets().filter((s) => this.attachmentFor(s).role === role);
  }

  /** Persisted flags can lag a missed close; live sockets are the source of truth. */
  private liveState(): SessionState {
    const userSockets = this.socketsFor("engine");
    const userDeviceId =
      userSockets
        .map((socket) => this.attachmentFor(socket).deviceId)
        .find((id): id is string => Boolean(id)) ?? this.state.executors?.userDeviceId;
    return overlayLiveExecutors(this.state, {
      userOnline: userSockets.length > 0,
      platformOnline: this.socketsFor("platform").length > 0,
      userDeviceId,
      viewerCount: this.socketsFor("viewer").length,
    });
  }

  private async appendEvent(message: EngineEventMessage): Promise<void> {
    const event: StoredEvent = {
      seq: this.nextSeq,
      ts: new Date().toISOString(),
      kind: message.kind,
      payload: message.payload,
    };
    this.nextSeq += 1;
    this.events = [...this.events, event].slice(-MAX_EVENTS);
    this.state = eventStatePatch(this.state, event);
    if (!this.state.sessionId && this.sessionId) this.state.sessionId = this.sessionId;

    await this.ctx.storage.put({
      events: this.events,
      state: this.state,
      nextSeq: this.nextSeq,
    });

    for (const viewer of this.socketsFor("viewer")) {
      sendJson(viewer, { type: "event", event });
    }
  }

  private async enqueueCommand(command: CommandMessage): Promise<void> {
    const target = resolveExecutor(command, this.state.preferredExecutor);

    // Defensive: unreachable while resolveExecutor maps deploy → user.
    if (shouldRefusePlatformDeploy(command, target)) {
      await this.appendEvent({
        type: "event",
        kind: "executor.refused",
        payload: { ...PLATFORM_DEPLOY_REFUSAL },
      });
      return;
    }

    if (command.type === "cmd.deploy") {
      const userOnline = this.socketsFor("engine").length > 0;
      if (!userOnline) {
        await this.appendEvent({
          type: "event",
          kind: "executor.refused",
          payload: {
            ...PLATFORM_DEPLOY_REFUSAL,
            reason: "user_executor_offline_for_deploy",
            hint: "Deploy needs a connected UserExecutor (desktop/VPS). Platform never holds deploy keys.",
          },
        });
        return;
      }
    }

    if (command.type === "cmd.prompt" || command.type === "cmd.steer") {
      this.state.preferredExecutor = target;
    }

    const queued: QueuedCommand = {
      id: newId(),
      ts: new Date().toISOString(),
      expiresAt: new Date(Date.now() + COMMAND_TTL_MS).toISOString(),
      command,
    };
    this.queue = [...this.queue, queued];
    await this.ctx.storage.put({ queue: this.queue, state: this.state });
    await this.drainQueueToExecutors();
  }

  private async drainQueueToExecutors(): Promise<void> {
    if (this.queue.length === 0) return;

    const remaining: QueuedCommand[] = [];
    for (const item of this.queue) {
      if (Date.parse(item.expiresAt) <= Date.now()) continue;
      const target = resolveExecutor(item.command, this.state.preferredExecutor);
      const role: Role = target === "platform" ? "platform" : "engine";
      const [socket] = this.socketsFor(role);
      if (!socket) {
        remaining.push(item);
        continue;
      }
      sendJson(socket, { ...item.command, id: item.id });
      // Optimistic remove; executor may cmd.ack — if disconnect, viewer can resend.
    }
    this.queue = remaining;
    await this.ctx.storage.put({ queue: this.queue });

    if (remaining.length > 0) {
      await this.appendEvent({
        type: "event",
        kind: "note",
        payload: {
          text: `${remaining.length} command(s) queued — open desktop ProofShip or Platform executor.`,
        },
      });
    }
  }
}
