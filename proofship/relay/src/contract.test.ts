import { describe, expect, it } from "vitest";
import {
  PLATFORM_DEPLOY_REFUSAL,
  authorizeEngine,
  eventStatePatch,
  parseViewerCommand,
  resolveExecutor,
  shouldRefusePlatformDeploy,
  type SessionState,
} from "./contract";

describe("parseViewerCommand", () => {
  it("parses cmd.prompt with optional fields", () => {
    expect(
      parseViewerCommand({
        type: "cmd.prompt",
        nl: "ship it",
        lane: "main",
        chatId: "c1",
        executor: "platform",
      }),
    ).toEqual({
      type: "cmd.prompt",
      nl: "ship it",
      lane: "main",
      chatId: "c1",
      executor: "platform",
    });
  });

  it("rejects prompt without nl", () => {
    expect(parseViewerCommand({ type: "cmd.prompt" })).toBeNull();
  });

  it("parses cmd.steer", () => {
    expect(parseViewerCommand({ type: "cmd.steer", nl: "nudge", chatId: "c2" })).toEqual({
      type: "cmd.steer",
      nl: "nudge",
      chatId: "c2",
    });
  });

  it("rejects steer without nl", () => {
    expect(parseViewerCommand({ type: "cmd.steer", chatId: "c2" })).toBeNull();
  });

  it("parses cmd.cancel", () => {
    expect(parseViewerCommand({ type: "cmd.cancel", chatId: "c3" })).toEqual({
      type: "cmd.cancel",
      chatId: "c3",
    });
    expect(parseViewerCommand({ type: "cmd.cancel" })).toEqual({ type: "cmd.cancel" });
  });

  it("parses cmd.deploy", () => {
    expect(
      parseViewerCommand({
        type: "cmd.deploy",
        networkId: "net-1",
        module: "mod.wasm",
        digest: "sha256:abc",
        chatId: "c4",
        executor: "platform",
      }),
    ).toEqual({
      type: "cmd.deploy",
      networkId: "net-1",
      module: "mod.wasm",
      digest: "sha256:abc",
      chatId: "c4",
      executor: "platform",
    });
  });

  it("rejects deploy missing networkId or module", () => {
    expect(parseViewerCommand({ type: "cmd.deploy", networkId: "n" })).toBeNull();
    expect(parseViewerCommand({ type: "cmd.deploy", module: "m" })).toBeNull();
  });

  it("rejects unknown types", () => {
    expect(parseViewerCommand({ type: "cmd.unknown" })).toBeNull();
    expect(parseViewerCommand(null)).toBeNull();
    expect(parseViewerCommand("x")).toBeNull();
  });
});

describe("resolveExecutor", () => {
  it("always routes deploy to user even if executor=platform", () => {
    const cmd = parseViewerCommand({
      type: "cmd.deploy",
      networkId: "n",
      module: "m",
      executor: "platform",
    })!;
    expect(resolveExecutor(cmd, "platform")).toBe("user");
    expect(resolveExecutor(cmd)).toBe("user");
  });

  it("honors explicit prompt executor", () => {
    expect(
      resolveExecutor({ type: "cmd.prompt", nl: "hi", executor: "platform" }, "user"),
    ).toBe("platform");
  });

  it("falls back to preferredExecutor then user", () => {
    expect(resolveExecutor({ type: "cmd.prompt", nl: "hi" }, "platform")).toBe("platform");
    expect(resolveExecutor({ type: "cmd.steer", nl: "hi" }, "platform")).toBe("platform");
    expect(resolveExecutor({ type: "cmd.cancel" })).toBe("user");
    expect(resolveExecutor({ type: "cmd.cancel" }, "user")).toBe("user");
  });
});

describe("shouldRefusePlatformDeploy", () => {
  it("documents defensive refuse when target is platform", () => {
    const deploy = {
      type: "cmd.deploy" as const,
      networkId: "n",
      module: "m",
      executor: "platform" as const,
    };
    // Normal path: resolveExecutor → user, so refuse is false.
    const target = resolveExecutor(deploy, "platform");
    expect(target).toBe("user");
    expect(shouldRefusePlatformDeploy(deploy, target)).toBe(false);

    // Safety belt if routing ever returned platform:
    expect(shouldRefusePlatformDeploy(deploy, "platform")).toBe(true);
    expect(PLATFORM_DEPLOY_REFUSAL.reason).toBe("platform_executor_cannot_hold_deploy_keys");
  });

  it("does not refuse non-deploy commands", () => {
    expect(shouldRefusePlatformDeploy({ type: "cmd.prompt", nl: "x" }, "platform")).toBe(false);
  });
});

describe("authorizeEngine", () => {
  it("accepts any token when no tokens configured", () => {
    const url = new URL("https://relay.example/ws/engine/s1?token=anything&deviceId=dev-a");
    expect(authorizeEngine({}, url)).toEqual({ ok: true, deviceId: "dev-a" });
  });

  it("matches per-device DEVICE_TOKENS", () => {
    const env = { DEVICE_TOKENS: JSON.stringify({ "dev-a": "secret-a", "dev-b": "secret-b" }) };
    expect(
      authorizeEngine(env, new URL("https://x/?token=secret-a&deviceId=dev-a")),
    ).toEqual({ ok: true, deviceId: "dev-a" });
    expect(authorizeEngine(env, new URL("https://x/?token=wrong&deviceId=dev-a"))).toEqual({
      ok: false,
    });
  });

  it("falls back to shared DEVICE_TOKEN / ENGINE_TOKEN via *", () => {
    expect(
      authorizeEngine(
        { DEVICE_TOKEN: "shared" },
        new URL("https://x/?token=shared&deviceId=any"),
      ),
    ).toEqual({ ok: true, deviceId: "any" });
    expect(
      authorizeEngine({ ENGINE_TOKEN: "eng" }, new URL("https://x/?token=eng")),
    ).toEqual({ ok: true, deviceId: "default" });
    expect(
      authorizeEngine({ ENGINE_TOKEN: "eng" }, new URL("https://x/?token=nope")),
    ).toEqual({ ok: false });
  });
});

describe("eventStatePatch", () => {
  it("patches launch, draft, gate, artifact, deployment", () => {
    let state: SessionState = {};
    state = eventStatePatch(state, {
      seq: 1,
      ts: "t1",
      kind: "session.open",
      payload: { id: "s1" },
    });
    expect(state.launch).toEqual({ id: "s1" });

    state = eventStatePatch(state, {
      seq: 2,
      ts: "t2",
      kind: "draft.ready",
      payload: { text: "d" },
    });
    expect(state.draft).toEqual({ text: "d" });

    state = eventStatePatch(state, { seq: 3, ts: "t3", kind: "gate.start", payload: {} });
    expect(state.gate).toBe("running");

    state = eventStatePatch(state, {
      seq: 4,
      ts: "t4",
      kind: "gate.done",
      payload: { ok: true },
    });
    expect(state.gate).toEqual({ ok: true });

    state = eventStatePatch(state, {
      seq: 5,
      ts: "t5",
      kind: "artifact.sealed",
      payload: { digest: "x" },
    });
    expect(state.artifact).toEqual({ digest: "x" });

    state = eventStatePatch(state, {
      seq: 6,
      ts: "t6",
      kind: "deploy.done",
      payload: { tx: "1" },
    });
    expect(state.deployment).toEqual({ tx: "1" });
  });

  it("tracks executor online/offline and transcript/notes", () => {
    let state: SessionState = {};
    state = eventStatePatch(state, {
      seq: 1,
      ts: "t1",
      kind: "executor.online",
      payload: { role: "engine", deviceId: "d1" },
    });
    expect(state.executors).toEqual({ userOnline: true, userDeviceId: "d1" });

    state = eventStatePatch(state, {
      seq: 2,
      ts: "t2",
      kind: "executor.online",
      payload: { role: "platform" },
    });
    expect(state.executors?.platformOnline).toBe(true);

    state = eventStatePatch(state, {
      seq: 3,
      ts: "t3",
      kind: "session.user",
      payload: { text: "hi" },
    });
    expect(state.transcript).toHaveLength(1);

    state = eventStatePatch(state, {
      seq: 4,
      ts: "t4",
      kind: "note",
      payload: { text: "n" },
    });
    expect(state.notes).toEqual([{ text: "n" }]);

    state = eventStatePatch(state, {
      seq: 5,
      ts: "t5",
      kind: "executor.offline",
      payload: { role: "platform" },
    });
    expect(state.executors?.platformOnline).toBe(false);
  });
});
