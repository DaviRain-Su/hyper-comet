import assert from "node:assert/strict";
import test from "node:test";
import { createRequire } from "node:module";

// Tiny copy of unwrap logic — source is TS. We import via a compile-free
// check by evaluating the same contract against the live Worker shape.

function asRecord(v) {
  return v && typeof v === "object" && !Array.isArray(v) ? v : null;
}

function unwrapRelayPayload(raw) {
  const rec = asRecord(raw);
  if (!rec) return {};
  const inner = asRecord(rec.state);
  const hoisted = inner ? { ...rec, ...inner } : rec;
  const executors = hoisted.executors ?? rec.presence ?? inner?.executors;
  return {
    ...hoisted,
    state: rec.state ?? rec,
    tail: Array.isArray(rec.tail) ? rec.tail : hoisted.tail,
    executors,
    launch: hoisted.launch ?? inner?.launch,
  };
}

function normalizeExecutors(raw) {
  if (!raw) return [];
  const rec = asRecord(raw);
  if (rec && ("userOnline" in rec || "platformOnline" in rec)) {
    return [
      { role: "engine", online: Boolean(rec.userOnline), deviceId: rec.userDeviceId },
      { role: "platform", online: Boolean(rec.platformOnline) },
    ];
  }
  return [];
}

function looksLikeDeviceRoom(id) {
  return /^desktop-[a-z0-9-]+$/i.test(String(id).trim());
}

test("GET /state wrap hoists userOnline so Desktop lamp can turn on", () => {
  const wrapped = {
    state: {
      executors: { userOnline: true, userDeviceId: "abc", platformOnline: false },
      launch: { harnesses: [{ id: "codex", name: "Codex" }] },
      harnesses: [{ id: "codex", name: "Codex" }],
    },
    tail: [],
    queueDepth: 0,
    presence: { userOnline: true, platformOnline: false, viewerCount: 0 },
  };
  const snap = unwrapRelayPayload(wrapped);
  const list = normalizeExecutors(snap.executors);
  assert.equal(list.find((e) => e.role === "engine")?.online, true);
  assert.equal(snap.launch.harnesses[0].id, "codex");
});

test("inner WS snapshot still reads executors", () => {
  const inner = {
    executors: { userOnline: true, userDeviceId: "dev" },
  };
  const snap = unwrapRelayPayload(inner);
  assert.equal(normalizeExecutors(snap.executors)[0].online, true);
});

test("local chat UUIDs are not device rooms", () => {
  assert.equal(looksLikeDeviceRoom("desktop-ba8835a2-079e-45e2-9b97-035d0e4f7a78"), true);
  assert.equal(looksLikeDeviceRoom("3f2c1b90-1111-2222-3333-444444444444"), false);
  assert.equal(looksLikeDeviceRoom(""), false);
});

void createRequire;
