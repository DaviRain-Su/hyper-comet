import assert from "node:assert/strict";
import { existsSync, mkdirSync, rmSync } from "node:fs";
import { join } from "node:path";
import { tmpdir } from "node:os";
import test from "node:test";
import { copyPgliteAssets, pgliteDistDir } from "./copy-pglite-assets.mjs";

test("pglite dist ships the wasm/data files the Vercel function needs", () => {
  const dist = pgliteDistDir();
  assert.equal(existsSync(join(dist, "pglite.data")), true);
  assert.equal(existsSync(join(dist, "pglite.wasm")), true);
});

test("copy is a no-op when the Vercel function output is missing", () => {
  const base = join(tmpdir(), `pf-pglite-${Date.now()}`);
  const result = copyPgliteAssets({ base, log() {} });
  assert.equal(result.skipped, true);
  assert.deepEqual(result.copied, []);
});

test("copy writes pglite.data next to the server _libs chunk", () => {
  const base = join(tmpdir(), `pf-pglite-${Date.now()}-out`);
  mkdirSync(join(base, ".vercel/output/functions/__server.func/_libs"), { recursive: true });
  const result = copyPgliteAssets({ base, log() {} });
  assert.equal(result.skipped, false);
  assert.ok(result.copied.includes("pglite.data"));
  assert.equal(
    existsSync(join(base, ".vercel/output/functions/__server.func/_libs/pglite.data")),
    true,
  );
  rmSync(base, { recursive: true, force: true });
});
