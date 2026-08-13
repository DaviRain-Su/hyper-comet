import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { MessageRow } from "@/lib/sessions";
import {
  DEFAULT_RELAY,
  defaultHarness,
  desktopFrom,
  eventsFromSnapshot,
  fetchSessionState,
  harnessesFrom,
  loadHarness,
  loadLastRoom,
  loadRelayUrl,
  looksLikeDeviceRoom,
  platformFrom,
  saveHarness,
  saveLastRoom,
  saveRelayUrl,
  unwrapRelayPayload,
  viewerCount,
  wsUrl,
  type ExecutorKind,
  type RelayEvent,
  type RelaySnapshot,
  type ViewerCommand,
} from "@/lib/relay";
import { extractLean } from "@/lib/gate";

export type LinkStatus = "idle" | "connecting" | "live" | "error";
export type PromptMode = "prompt" | "steer" | "comment";

function textOf(payload: Record<string, unknown>) {
  if (typeof payload.text === "string") return payload.text;
  if (typeof payload.nl === "string") return payload.nl;
  if (typeof payload.message === "string") return payload.message;
  if (typeof payload.error === "string") return payload.error;
  try {
    return JSON.stringify(payload);
  } catch {
    return "";
  }
}

function eventToMessage(event: RelayEvent, idx: number): MessageRow | null {
  const kind = event.kind;
  const payload = event.payload;
  const createdAt = event.ts
    ? new Date(event.ts).toISOString()
    : new Date().toISOString();
  const id = `relay-${event.seq ?? idx}-${kind}`;

  if (kind === "session.user") {
    return { id, role: "user", kind: "text", content: textOf(payload), meta: null, createdAt };
  }
  if (kind === "session.steer") {
    return { id, role: "user", kind: "text", content: `steer · ${textOf(payload)}`, meta: null, createdAt };
  }
  if (kind === "session.comment" || kind === "note") {
    return { id, role: "system", kind: "text", content: textOf(payload), meta: null, createdAt };
  }
  if (kind === "session.agent" || kind === "draft.ready") {
    const raw = textOf(payload);
    const lean = extractLean(raw) ?? (typeof payload.source === "string" ? payload.source : "");
    if (lean && /import\s+ProofForgeV2/.test(lean)) {
      return { id, role: "agent", kind: "lean", content: lean, meta: null, createdAt };
    }
    return { id, role: "agent", kind: "text", content: raw, meta: null, createdAt };
  }
  if (kind === "session.tool" || kind === "gate.done" || kind === "gate.start") {
    return { id, role: "tool", kind: "text", content: textOf(payload) || kind, meta: null, createdAt };
  }
  if (kind === "session.done" || kind === "deploy.done" || kind === "executor.refused") {
    return { id, role: "system", kind: "text", content: textOf(payload) || kind, meta: null, createdAt };
  }
  if (kind === "executor.online" || kind === "executor.offline") return null;
  return null;
}

export function useDesktopLink(
  roomFromSession?: string,
  seed?: { relay?: string; session?: string },
) {
  const [relayUrl, setRelayUrlState] = useState(DEFAULT_RELAY);
  const [roomId, setRoomIdState] = useState("");
  const [status, setStatus] = useState<LinkStatus>("idle");
  const [lastError, setLastError] = useState<string | null>(null);
  const [snapshot, setSnapshot] = useState<RelaySnapshot | null>(null);
  const [liveEvents, setLiveEvents] = useState<RelayEvent[]>([]);
  const [harness, setHarnessState] = useState("codex");
  const [executor, setExecutor] = useState<ExecutorKind>("user");
  const socketRef = useRef<WebSocket | null>(null);
  const pollRef = useRef<number | null>(null);
  const didAuto = useRef(false);

  useEffect(() => {
    const seededRelay = seed?.relay || loadRelayUrl();
    setRelayUrlState(seededRelay);
    if (seed?.relay) saveRelayUrl(seed.relay);
    const fromRoute =
      roomFromSession && looksLikeDeviceRoom(roomFromSession) ? roomFromSession : "";
    const last = loadLastRoom();
    const room =
      seed?.session ||
      fromRoute ||
      (looksLikeDeviceRoom(last) ? last : "");
    if (room) setRoomIdState(room);
    setHarnessState(loadHarness());
  }, [roomFromSession, seed?.relay, seed?.session]);

  const setRelayUrl = useCallback((url: string) => {
    setRelayUrlState(url);
    saveRelayUrl(url);
  }, []);

  const setRoomId = useCallback((id: string) => {
    setRoomIdState(id);
    saveLastRoom(id);
  }, []);

  const setHarness = useCallback((id: string) => {
    setHarnessState(id);
    saveHarness(id);
  }, []);

  const disconnect = useCallback(() => {
    if (pollRef.current) {
      window.clearInterval(pollRef.current);
      pollRef.current = null;
    }
    socketRef.current?.close();
    socketRef.current = null;
    setStatus("idle");
  }, []);

  const ingestSnapshot = useCallback((next: RelaySnapshot) => {
    setSnapshot(next);
    const ev = eventsFromSnapshot(next);
    if (ev.length) setLiveEvents(ev);
    const def = defaultHarness(next);
    if (def) setHarnessState((cur) => cur || def);
  }, []);

  const connect = useCallback(
    (explicitRoom?: string) => {
      const room = (explicitRoom || roomId).trim();
      const base = relayUrl.trim() || DEFAULT_RELAY;
      if (!room) {
        setLastError("Need a room id (this session, or desktop-…).");
        setStatus("error");
        return;
      }
      saveLastRoom(room);
      saveRelayUrl(base);
      disconnect();
      setLastError(null);
      setStatus("connecting");

      const tick = async () => {
        try {
          const state = await fetchSessionState(base, room);
          ingestSnapshot(state);
        } catch {
          /* WS may still be up */
        }
      };
      void tick();
      pollRef.current = window.setInterval(() => void tick(), 4000);

      try {
        const ws = new WebSocket(wsUrl(base, room));
        socketRef.current = ws;
        ws.onopen = () => {
          setStatus("live");
          setLastError(null);
        };
        ws.onerror = () => {
          setLastError("Relay socket error");
        };
        ws.onclose = () => {
          if (socketRef.current === ws) {
            socketRef.current = null;
            setStatus((s) => (s === "connecting" ? "error" : "idle"));
          }
        };
        ws.onmessage = (ev) => {
          try {
            const msg = JSON.parse(String(ev.data)) as {
              type?: string;
              state?: RelaySnapshot;
              event?: RelayEvent;
              error?: string;
            };
            if (msg.type === "snapshot" && msg.state) {
              ingestSnapshot(unwrapRelayPayload(msg.state));
            }
            if (msg.type === "event" && msg.event?.kind) {
              setLiveEvents((prev) => [...prev, msg.event as RelayEvent]);
            }
            if (msg.type === "error" && msg.error) setLastError(msg.error);
          } catch {
            /* ignore malformed */
          }
        };
      } catch (e) {
        setStatus("error");
        setLastError(e instanceof Error ? e.message : "Relay unreachable");
      }
    },
    [disconnect, ingestSnapshot, relayUrl, roomId],
  );

  useEffect(() => () => disconnect(), [disconnect]);

  useEffect(() => {
    if (didAuto.current) return;
    const room = (seed?.session || roomId).trim();
    if (room && looksLikeDeviceRoom(room)) {
      didAuto.current = true;
      connect(room);
    }
  }, [connect, roomId, seed?.session]);

  const send = useCallback((cmd: ViewerCommand) => {
    const ws = socketRef.current;
    if (!ws || ws.readyState !== WebSocket.OPEN) {
      throw new Error("Relay not connected");
    }
    ws.send(JSON.stringify(cmd));
  }, []);

  const desktop = desktopFrom(snapshot);
  const platform = platformFrom(snapshot);
  const desktopOnline = Boolean(desktop?.online);
  const platformOnline = Boolean(platform?.online);
  const relayOk = status === "live" || status === "connecting";
  const harnesses = harnessesFrom(snapshot);

  const messages = useMemo(() => {
    const out: MessageRow[] = [];
    liveEvents.forEach((e, i) => {
      const row = eventToMessage(e, i);
      if (row) out.push(row);
    });
    return out;
  }, [liveEvents]);

  return {
    relayUrl,
    setRelayUrl,
    roomId,
    setRoomId,
    status,
    lastError,
    snapshot,
    desktop,
    platform,
    desktopOnline,
    platformOnline,
    viewers: viewerCount(snapshot),
    relayOk,
    harness,
    setHarness,
    harnesses,
    executor,
    setExecutor,
    events: liveEvents,
    messages,
    connect,
    disconnect,
    sendPrompt: (nl: string) =>
      send({ type: "cmd.prompt", nl, lane: harness, executor }),
    sendSteer: (nl: string) => send({ type: "cmd.steer", nl }),
    sendComment: (nl: string) => send({ type: "cmd.comment", text: nl }),
    sendCancel: () => send({ type: "cmd.cancel" }),
    sendDeploy: (opts: { networkId: string; module: string; digest?: string }) =>
      send({
        type: "cmd.deploy",
        networkId: opts.networkId,
        module: opts.module,
        executor,
        digest: opts.digest,
      }),
  };
}
