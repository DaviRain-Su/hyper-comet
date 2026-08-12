#!/usr/bin/env node
/**
 * Local smoke: relay health → platform online → prompt → deploy refused → share.
 *
 * Prerequisites: relay already running (`cd proofship/relay && npm run dev`).
 * Uses Node built-in WebSocket (Node 22+).
 *
 *   RELAY_URL=http://127.0.0.1:8787 node proofship/scripts/smoke-relay-platform.mjs
 */
import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const RELAY = (process.env.RELAY_URL ?? "http://127.0.0.1:8787").replace(/\/+$/u, "");
const SESSION = process.env.PROOFSHIP_SESSION_ID ?? `smoke-${Date.now()}`;
const TOKEN = process.env.PROOFSHIP_DEVICE_TOKEN ?? process.env.ENGINE_TOKEN ?? "dev";
const ROOT = join(dirname(fileURLToPath(import.meta.url)), "../..");
const PLATFORM_DIR = join(ROOT, "proofship/platform-sandbox");

function wsBase(httpBase) {
  return httpBase.replace(/^https:/iu, "wss:").replace(/^http:/iu, "ws:");
}

function sleep(ms) {
  return new Promise((r) => setTimeout(r, ms));
}

async function health() {
  const res = await fetch(`${RELAY}/health`);
  if (!res.ok) throw new Error(`health ${res.status}`);
  const body = await res.json();
  if (!body.ok || !body.dualExecutor) throw new Error(`unexpected health: ${JSON.stringify(body)}`);
  console.log("ok health", body.contract);
}

function onceOpen(ws) {
  return new Promise((resolve, reject) => {
    const t = setTimeout(() => reject(new Error("ws open timeout")), 15_000);
    ws.addEventListener("open", () => {
      clearTimeout(t);
      resolve();
    });
    ws.addEventListener("error", () => {
      clearTimeout(t);
      reject(new Error("ws error"));
    });
  });
}

function waitForEvent(ws, predicate, label, ms = 30_000) {
  return new Promise((resolve, reject) => {
    const t = setTimeout(() => reject(new Error(`timeout waiting for ${label}`)), ms);
    const onMsg = (ev) => {
      let msg;
      try {
        msg = JSON.parse(String(ev.data));
      } catch {
        return;
      }
      if (predicate(msg)) {
        clearTimeout(t);
        ws.removeEventListener("message", onMsg);
        resolve(msg);
      }
    };
    ws.addEventListener("message", onMsg);
  });
}

async function main() {
  if (typeof WebSocket === "undefined") {
    throw new Error("Node WebSocket missing — use Node 22+");
  }
  console.log(`smoke relay=${RELAY} session=${SESSION}`);
  await health();

  const platform = spawn("npm", ["start"], {
    cwd: PLATFORM_DIR,
    env: {
      ...process.env,
      PROOFSHIP_RELAY: RELAY,
      PROOFSHIP_DEVICE_TOKEN: TOKEN,
      PROOFSHIP_DEVICE_ID: "smoke-platform",
      PROOFSHIP_SESSION_ID: SESSION,
    },
    stdio: ["ignore", "pipe", "pipe"],
  });
  platform.stdout?.on("data", (c) => process.stdout.write(`[platform] ${c}`));
  platform.stderr?.on("data", (c) => process.stderr.write(`[platform] ${c}`));

  const viewerUrl = `${wsBase(RELAY)}/ws/web/${encodeURIComponent(SESSION)}`;
  const viewer = new WebSocket(viewerUrl);
  try {
    await onceOpen(viewer);
    console.log("ok viewer connected");

    await waitForEvent(
      viewer,
      (msg) =>
        msg.type === "event" &&
        msg.event?.kind === "executor.online" &&
        msg.event?.payload?.role === "platform",
      "executor.online(platform)",
      45_000,
    );
    console.log("ok platform online");

    viewer.send(
      JSON.stringify({
        type: "cmd.prompt",
        nl: "hello from smoke — no lean source",
        executor: "platform",
      }),
    );
    await waitForEvent(
      viewer,
      (msg) => msg.type === "event" && msg.event?.kind === "session.done",
      "session.done",
      45_000,
    );
    console.log("ok prompt → session.done");

    viewer.send(
      JSON.stringify({
        type: "cmd.deploy",
        networkId: "xlayer-testnet",
        module: "Smoke",
        executor: "platform",
      }),
    );
    await waitForEvent(
      viewer,
      (msg) => msg.type === "event" && msg.event?.kind === "executor.refused",
      "executor.refused(deploy)",
      20_000,
    );
    console.log("ok deploy refused without UserExecutor");

    const shareRes = await fetch(`${RELAY}/api/share/${encodeURIComponent(SESSION)}`);
    if (!shareRes.ok) throw new Error(`share ${shareRes.status}`);
    const share = await shareRes.json();
    if (!share.readonly) throw new Error("share missing readonly");
    console.log("ok share snapshot");

    console.log("SMOKE PASS");
  } finally {
    viewer.close();
    platform.kill("SIGTERM");
    await sleep(300);
  }
}

main().catch((err) => {
  console.error("SMOKE FAIL", err);
  process.exit(1);
});
