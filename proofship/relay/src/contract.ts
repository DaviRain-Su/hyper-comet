/**
 * Pure relay contract helpers — unit-testable without Durable Object runtime.
 */

export type EventKind =
  | "session.open"
  | "session.user"
  | "session.agent"
  | "session.tool"
  | "session.done"
  | "draft.ready"
  | "gate.start"
  | "gate.done"
  | "artifact.sealed"
  | "executor.online"
  | "executor.offline"
  | "executor.refused"
  | "deploy.done"
  | "session.comment"
  | "harness.catalog"
  | "note";

export interface EngineEventMessage {
  type: "event";
  kind: EventKind | string;
  payload: unknown;
}

export interface StoredEvent {
  seq: number;
  ts: string;
  kind: string;
  payload: unknown;
}

export interface PromptCommand {
  type: "cmd.prompt";
  nl: string;
  lane?: string;
  chatId?: string;
  executor?: "user" | "platform";
}

export interface CancelCommand {
  type: "cmd.cancel";
  chatId?: string;
}

export interface SteerCommand {
  type: "cmd.steer";
  nl: string;
  chatId?: string;
}

export interface DeployCommand {
  type: "cmd.deploy";
  networkId: string;
  module: string;
  digest?: string;
  chatId?: string;
  /** Ignored if platform — always refused for keyed deploy. */
  executor?: "user" | "platform";
}

export interface CommentCommand {
  type: "cmd.comment";
  text: string;
}

export type CommandMessage =
  | PromptCommand
  | CancelCommand
  | SteerCommand
  | DeployCommand
  | CommentCommand;

export interface ComputerInfo {
  deviceId: string;
  roomId: string;
  hostname?: string;
  os?: string;
}

export interface SessionState {
  sessionId?: string;
  preferredExecutor?: "user" | "platform";
  /** Bound computer. Same install always owns `desktop-{deviceId}`. */
  computer?: ComputerInfo;
  executors?: {
    userOnline?: boolean;
    platformOnline?: boolean;
    userDeviceId?: string;
    userLastSeenAt?: string;
    platformLastSeenAt?: string;
    viewerCount?: number;
  };
  launch?: unknown;
  draft?: unknown;
  gate?: "running" | Record<string, unknown>;
  artifact?: unknown;
  deployment?: unknown;
  transcript?: unknown[];
  notes?: unknown[];
  harnesses?: Array<{
    id?: string;
    name?: string;
    installed?: boolean;
    enabled?: boolean | null;
  }>;
  preferredLane?: string;
}

/** Subset of Worker env used for device/engine token checks. */
export interface EngineAuthEnv {
  ENGINE_TOKEN?: string;
  DEVICE_TOKEN?: string;
  DEVICE_TOKENS?: string;
}

/** Subset of Worker env used for viewer and share token checks. */
export interface ViewerAuthEnv {
  VIEWER_TOKEN?: string;
  SHARE_TOKEN?: string;
}

export const MAX_NOTES = 40;
export const MAX_TRANSCRIPT = 100;

export const PLATFORM_DEPLOY_REFUSAL = {
  reason: "platform_executor_cannot_hold_deploy_keys",
  hint: "Connect a user desktop/VPS executor and deploy there (wallet or DevEnvKey).",
} as const;

export function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

export function parseDeviceTokens(env: EngineAuthEnv): Map<string, string> {
  const map = new Map<string, string>();
  if (env.DEVICE_TOKENS) {
    try {
      const obj = JSON.parse(env.DEVICE_TOKENS) as Record<string, string>;
      for (const [k, v] of Object.entries(obj)) {
        if (typeof v === "string" && v.length > 0) map.set(k, v);
      }
    } catch {
      /* ignore bad JSON */
    }
  }
  const shared = env.DEVICE_TOKEN ?? env.ENGINE_TOKEN;
  if (shared) map.set("*", shared);
  return map;
}

export function authorizeEngine(
  env: EngineAuthEnv,
  url: URL,
): { ok: true; deviceId: string } | { ok: false } {
  const tokens = parseDeviceTokens(env);
  const token = url.searchParams.get("token") ?? "";
  const deviceId = url.searchParams.get("deviceId") ?? "default";
  if (tokens.size === 0) {
    // Local spike: accept any token when nothing configured.
    return { ok: true, deviceId };
  }
  const expected = tokens.get(deviceId) ?? tokens.get("*");
  if (expected !== undefined && token === expected) {
    return { ok: true, deviceId };
  }
  return { ok: false };
}

export function authorizeViewer(env: ViewerAuthEnv, url: URL): boolean {
  if (!env.VIEWER_TOKEN) return true;
  const token = url.searchParams.get("viewerToken") ?? url.searchParams.get("token");
  return token === env.VIEWER_TOKEN;
}

export function authorizeShare(env: ViewerAuthEnv, url: URL): boolean {
  if (env.SHARE_TOKEN) {
    const token = url.searchParams.get("token") ?? url.searchParams.get("viewerToken");
    return token === env.SHARE_TOKEN;
  }
  return authorizeViewer(env, url);
}

export interface SharePayload {
  readonly: true;
  sessionId: string;
  share: {
    gate: unknown;
    artifact: unknown;
    deployment: unknown;
    transcript: unknown[];
    notes: unknown[];
  };
  tail: StoredEvent[];
}

export const SHARE_ALLOWED_EVENT_KINDS = [
  "session.user",
  "session.agent",
  "session.tool",
  "session.done",
  "gate.done",
  "artifact.sealed",
  "deploy.done",
  "session.comment",
  "note",
] as const;

export function redactSharePayload(
  state: SessionState,
  events: StoredEvent[] = [],
  sessionIdFallback = "",
  tailLimit = 80,
): SharePayload {
  const gate = state.gate === "running" ? { status: "running" } : state.gate ?? null;
  const tail = events
    .filter((e) => (SHARE_ALLOWED_EVENT_KINDS as readonly string[]).includes(e.kind))
    .slice(-tailLimit);

  return {
    readonly: true,
    sessionId: state.sessionId ?? sessionIdFallback,
    share: {
      gate,
      artifact: state.artifact ?? null,
      deployment: state.deployment ?? null,
      transcript: Array.isArray(state.transcript) ? state.transcript : [],
      notes: Array.isArray(state.notes) ? state.notes : [],
    },
    tail,
  };
}

export function parseViewerCommand(raw: unknown): CommandMessage | null {
  if (!isRecord(raw) || typeof raw.type !== "string") return null;
  if (raw.type === "cmd.prompt") {
    if (typeof raw.nl !== "string") return null;
    const command: PromptCommand = { type: "cmd.prompt", nl: raw.nl };
    if (typeof raw.lane === "string") command.lane = raw.lane;
    if (typeof raw.chatId === "string") command.chatId = raw.chatId;
    if (raw.executor === "user" || raw.executor === "platform") command.executor = raw.executor;
    return command;
  }
  if (raw.type === "cmd.cancel") {
    const command: CancelCommand = { type: "cmd.cancel" };
    if (typeof raw.chatId === "string") command.chatId = raw.chatId;
    return command;
  }
  if (raw.type === "cmd.steer") {
    if (typeof raw.nl !== "string") return null;
    const command: SteerCommand = { type: "cmd.steer", nl: raw.nl };
    if (typeof raw.chatId === "string") command.chatId = raw.chatId;
    return command;
  }
  if (raw.type === "cmd.deploy") {
    if (typeof raw.networkId !== "string" || typeof raw.module !== "string") return null;
    const command: DeployCommand = {
      type: "cmd.deploy",
      networkId: raw.networkId,
      module: raw.module,
    };
    if (typeof raw.digest === "string") command.digest = raw.digest;
    if (typeof raw.chatId === "string") command.chatId = raw.chatId;
    if (raw.executor === "user" || raw.executor === "platform") command.executor = raw.executor;
    return command;
  }
  if (raw.type === "cmd.comment") {
    if (typeof raw.text !== "string" || !raw.text.trim()) return null;
    return { type: "cmd.comment", text: raw.text };
  }
  return null;
}

/**
 * Pick executor for a viewer command.
 * Deploy always routes to user (keys must not live on platform).
 */
export function resolveExecutor(
  command: CommandMessage,
  preferredExecutor?: "user" | "platform",
): "user" | "platform" {
  if (command.type === "cmd.comment") {
    return "user";
  }
  if (command.type === "cmd.deploy") {
    return "user";
  }
  if ("executor" in command && (command.executor === "user" || command.executor === "platform")) {
    return command.executor;
  }
  return preferredExecutor === "platform" ? "platform" : "user";
}

/**
 * Defensive gate: refuse deploy when target is platform.
 * Unreachable when paired with {@link resolveExecutor} (deploy → user),
 * but kept so enqueue stays safe if routing ever changes.
 */
export function shouldRefusePlatformDeploy(
  command: CommandMessage,
  target: "user" | "platform",
): boolean {
  return command.type === "cmd.deploy" && target === "platform";
}

export function eventStatePatch(state: SessionState, event: StoredEvent): SessionState {
  const next: SessionState = {
    ...state,
    executors: { ...(state.executors ?? {}) },
  };
  switch (event.kind) {
    case "session.open":
      next.launch = event.payload;
      applyComputer(next, event.payload);
      applyHarnessCatalog(next, event.payload);
      break;
    case "harness.catalog":
      applyHarnessCatalog(next, event.payload);
      break;
    case "session.user":
    case "session.agent":
    case "session.tool":
    case "session.done":
    case "session.comment": {
      const transcript = Array.isArray(next.transcript) ? [...next.transcript] : [];
      transcript.push({ kind: event.kind, payload: event.payload, ts: event.ts });
      next.transcript = transcript.slice(-MAX_TRANSCRIPT);
      if (
        event.kind === "session.user" &&
        isRecord(event.payload) &&
        typeof event.payload.harness === "string" &&
        event.payload.harness
      ) {
        next.preferredLane = event.payload.harness;
      }
      break;
    }
    case "draft.ready":
      next.draft = event.payload;
      break;
    case "gate.start":
      next.gate = "running";
      break;
    case "gate.done":
      next.gate = isRecord(event.payload) ? { ...event.payload } : {};
      break;
    case "artifact.sealed":
      next.artifact = event.payload;
      break;
    case "deploy.done":
      next.deployment = event.payload;
      break;
    case "executor.online":
      if (isRecord(event.payload)) {
        if (event.payload.role === "platform") {
          next.executors!.platformOnline = true;
          next.executors!.platformLastSeenAt = event.ts;
        } else {
          next.executors!.userOnline = true;
          next.executors!.userLastSeenAt = event.ts;
          if (typeof event.payload.deviceId === "string") {
            next.executors!.userDeviceId = event.payload.deviceId;
            applyComputer(next, event.payload);
          }
        }
      }
      break;
    case "executor.offline":
      if (isRecord(event.payload)) {
        if (event.payload.role === "platform") {
          next.executors!.platformOnline = false;
          next.executors!.platformLastSeenAt = event.ts;
        } else {
          next.executors!.userOnline = false;
          next.executors!.userLastSeenAt = event.ts;
        }
      }
      break;
    case "note": {
      const notes = Array.isArray(next.notes) ? [...next.notes] : [];
      notes.push(event.payload);
      next.notes = notes.slice(-MAX_NOTES);
      break;
    }
    default:
      break;
  }
  return next;
}

export function computerFromPayload(payload: unknown): ComputerInfo | undefined {
  if (!isRecord(payload)) return undefined;
  const deviceId =
    typeof payload.deviceId === "string" && payload.deviceId.trim()
      ? payload.deviceId.trim()
      : "";
  if (!deviceId) return undefined;
  const roomId =
    (typeof payload.roomId === "string" && payload.roomId.trim()) ||
    (typeof payload.sessionId === "string" && payload.sessionId.startsWith("desktop-")
      ? payload.sessionId.trim()
      : "") ||
    `desktop-${deviceId}`;
  const computer: ComputerInfo = { deviceId, roomId };
  if (typeof payload.hostname === "string" && payload.hostname.trim()) {
    computer.hostname = payload.hostname.trim();
  }
  if (typeof payload.os === "string" && payload.os.trim()) {
    computer.os = payload.os.trim();
  }
  return computer;
}

function applyComputer(state: SessionState, payload: unknown): void {
  const computer = computerFromPayload(payload);
  if (!computer) return;
  state.computer = { ...(state.computer ?? {}), ...computer };
  state.sessionId = computer.roomId;
}

function applyHarnessCatalog(state: SessionState, payload: unknown): void {
  if (!isRecord(payload)) return;
  if (Array.isArray(payload.harnesses)) {
    state.harnesses = payload.harnesses.filter((row) => isRecord(row)) as SessionState["harnesses"];
  }
  if (typeof payload.defaultId === "string" && payload.defaultId) {
    state.preferredLane = payload.defaultId;
  }
}

/** Live sockets win over persisted online flags (stale after a missed close). */
export interface LiveExecutorSockets {
  userOnline: boolean;
  platformOnline: boolean;
  userDeviceId?: string;
  viewerCount?: number;
}

export function overlayLiveExecutors(
  state: SessionState,
  live: LiveExecutorSockets,
): SessionState {
  const userDeviceId = live.userDeviceId || state.executors?.userDeviceId;
  const computer =
    state.computer ??
    (userDeviceId
      ? { deviceId: userDeviceId, roomId: `desktop-${userDeviceId}` }
      : undefined);
  return {
    ...state,
    computer,
    sessionId: state.sessionId || computer?.roomId,
    executors: {
      ...(state.executors ?? {}),
      userOnline: live.userOnline,
      platformOnline: live.platformOnline,
      userDeviceId,
      viewerCount: live.viewerCount,
    },
  };
}
