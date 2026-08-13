/** Official ProofShip relay — thin pipe. Compute stays on the desktop. */

export const DEFAULT_RELAY = "https://proofship-relay.davirain-yin.workers.dev";
const RELAY_KEY = "proofship.relay";
const ROOM_KEY = "proofship.lastRoom";
const HARNESS_KEY = "proofship.harness";

export type ExecutorKind = "user" | "platform";

export type RelayEvent = {
  kind: string;
  payload: Record<string, unknown>;
  seq?: number;
  ts?: number;
};

export type RelayExecutor = {
  role: string;
  online: boolean;
  deviceId?: string;
};

export type Harness = {
  id: string;
  name: string;
  installed?: boolean;
  enabled?: boolean;
};

export function looksLikeDeviceRoom(id: string) {
  return /^desktop-[a-z0-9-]+$/i.test(id.trim());
}

export type RelaySnapshot = {
  state?: Record<string, unknown>;
  tail?: RelayEvent[];
  transcript?: RelayEvent[];
  queueDepth?: number;
  harnesses?: Harness[];
  preferredLane?: string;
  executors?:
    | RelayExecutor[]
    | Record<string, RelayExecutor | unknown>
    | {
        userOnline?: boolean;
        userDeviceId?: string;
        userLastSeenAt?: string;
        platformOnline?: boolean;
        viewerCount?: number;
      };
  launch?: {
    sessionId?: string;
    deviceId?: string;
    role?: string;
    harnesses?: Harness[];
    defaultId?: string;
  };
  preferredExecutor?: string;
};

export const DEFAULT_HARNESSES: Harness[] = [
  { id: "claude-code", name: "Claude Code" },
  { id: "codex", name: "Codex" },
  { id: "cursor", name: "Cursor" },
  { id: "open-code", name: "OpenCode" },
  { id: "grok", name: "Grok" },
  { id: "pi", name: "Pi" },
];

export function loadRelayUrl(): string {
  try {
    return localStorage.getItem(RELAY_KEY) || DEFAULT_RELAY;
  } catch {
    return DEFAULT_RELAY;
  }
}

export function saveRelayUrl(url: string) {
  try {
    localStorage.setItem(RELAY_KEY, url);
  } catch {
    /* private mode */
  }
}

export function loadLastRoom(): string {
  try {
    return localStorage.getItem(ROOM_KEY) || "";
  } catch {
    return "";
  }
}

export function saveLastRoom(id: string) {
  try {
    if (id) localStorage.setItem(ROOM_KEY, id);
  } catch {
    /* private mode */
  }
}

export function loadHarness(): string {
  try {
    return localStorage.getItem(HARNESS_KEY) || "codex";
  } catch {
    return "codex";
  }
}

export function saveHarness(id: string) {
  try {
    localStorage.setItem(HARNESS_KEY, id);
  } catch {
    /* private mode */
  }
}

export function httpBase(relay: string) {
  return relay.replace(/\/+$/, "");
}

export function wsUrl(relay: string, sessionId: string, viewerToken?: string) {
  const u = new URL(httpBase(relay));
  u.protocol = u.protocol === "https:" ? "wss:" : "ws:";
  u.pathname = `/ws/web/${encodeURIComponent(sessionId)}`;
  const q = new URLSearchParams();
  if (viewerToken) q.set("viewerToken", viewerToken);
  u.search = q.toString() ? `?${q.toString()}` : "";
  return u.toString();
}

export function mcpUrl(relay: string) {
  return `${httpBase(relay)}/mcp`;
}

export function healthUrl(relay: string) {
  return `${httpBase(relay)}/health`;
}

function asRecord(v: unknown): Record<string, unknown> | null {
  return v && typeof v === "object" && !Array.isArray(v) ? (v as Record<string, unknown>) : null;
}

export function normalizeExecutors(raw: RelaySnapshot["executors"]): RelayExecutor[] {
  if (!raw) return [];
  if (Array.isArray(raw)) {
    return raw.map((e) => ({
      role: String(e.role ?? ""),
      online: Boolean(e.online),
      deviceId: e.deviceId ? String(e.deviceId) : undefined,
    }));
  }
  const rec = asRecord(raw);
  if (!rec) return [];
  if ("userOnline" in rec || "platformOnline" in rec || "userDeviceId" in rec) {
    const out: RelayExecutor[] = [
      {
        role: "engine",
        online: Boolean(rec.userOnline),
        deviceId: rec.userDeviceId ? String(rec.userDeviceId) : undefined,
      },
      { role: "platform", online: Boolean(rec.platformOnline) },
    ];
    return out;
  }
  return Object.entries(rec).map(([role, e]) => {
    const item = asRecord(e);
    return {
      role: String(item?.role || role),
      online: Boolean(item?.online),
      deviceId: item?.deviceId ? String(item.deviceId) : undefined,
    };
  });
}

export function desktopFrom(snapshot: RelaySnapshot | null): RelayExecutor | null {
  const list = normalizeExecutors(snapshot?.executors);
  return (
    list.find((e) => e.role === "engine" || e.role === "user" || e.role === "desktop") ??
    list.find((e) => e.online && e.role !== "platform" && e.role !== "viewer") ??
    null
  );
}

/** GET /state wraps the room as `{ state, tail, presence }`. WS snapshots are the inner state. */
export function unwrapRelayPayload(raw: unknown): RelaySnapshot {
  const rec = asRecord(raw);
  if (!rec) return {};
  const inner = asRecord(rec.state);
  const hoisted = inner ? { ...rec, ...inner } : rec;
  const executors = hoisted.executors ?? rec.presence ?? inner?.executors;
  return {
    ...hoisted,
    state: rec.state ?? rec,
    tail: Array.isArray(rec.tail) ? (rec.tail as RelayEvent[]) : hoisted.tail,
    executors: (executors as RelaySnapshot["executors"]) ?? undefined,
    launch: (hoisted.launch ?? inner?.launch) as RelaySnapshot["launch"],
  };
}

export function platformFrom(snapshot: RelaySnapshot | null): RelayExecutor | null {
  return normalizeExecutors(snapshot?.executors).find((e) => e.role === "platform") ?? null;
}

export function viewerCount(snapshot: RelaySnapshot | null): number {
  const rec = asRecord(snapshot?.executors);
  if (rec && typeof rec.viewerCount === "number") return rec.viewerCount;
  return 0;
}

export function harnessesFrom(snapshot: RelaySnapshot | null): Harness[] {
  if (snapshot?.launch?.harnesses?.length) return snapshot.launch.harnesses;
  if (snapshot?.harnesses?.length) return snapshot.harnesses;
  const inner = asRecord(snapshot?.state);
  const launch = inner?.launch as RelaySnapshot["launch"] | undefined;
  if (launch?.harnesses?.length) return launch.harnesses;
  const nested = inner?.harnesses;
  if (Array.isArray(nested) && nested.length) return nested as Harness[];
  return DEFAULT_HARNESSES;
}

export function defaultHarness(snapshot: RelaySnapshot | null): string {
  if (snapshot?.preferredLane) return snapshot.preferredLane;
  const launch = snapshot?.launch ?? (asRecord(snapshot?.state)?.launch as RelaySnapshot["launch"]);
  return launch?.defaultId || loadHarness();
}

export function eventsFromSnapshot(snapshot: RelaySnapshot | null): RelayEvent[] {
  if (!snapshot) return [];
  const src = snapshot.transcript ?? snapshot.tail ?? [];
  return src
    .map((e) => ({
      kind: String(e.kind ?? ""),
      payload: (e.payload && typeof e.payload === "object"
        ? e.payload
        : {}) as Record<string, unknown>,
      seq: typeof e.seq === "number" ? e.seq : undefined,
      ts: typeof e.ts === "number" ? e.ts : typeof e.ts === "string" ? Date.parse(e.ts) : undefined,
    }))
    .filter((e) => e.kind);
}

export async function fetchSessionState(
  relay: string,
  sessionId: string,
): Promise<RelaySnapshot> {
  const res = await fetch(
    `${httpBase(relay)}/api/sessions/${encodeURIComponent(sessionId)}/state`,
    { credentials: "omit" },
  );
  if (!res.ok) throw new Error(`relay ${res.status}`);
  return unwrapRelayPayload(await res.json());
}

export type ViewerCommand =
  | { type: "cmd.prompt"; nl: string; lane: string; executor: ExecutorKind }
  | { type: "cmd.steer"; nl: string }
  | { type: "cmd.comment"; text: string }
  | { type: "cmd.cancel" }
  | {
      type: "cmd.deploy";
      networkId: string;
      module: string;
      executor: ExecutorKind;
      digest?: string;
    };

export type SessionSearch = {
  relay?: string;
  session?: string;
};

export function sessionSearchSchema(s: Record<string, unknown>): SessionSearch {
  const out: SessionSearch = {};
  if (typeof s.relay === "string") out.relay = s.relay;
  if (typeof s.session === "string") out.session = s.session;
  return out;
}
