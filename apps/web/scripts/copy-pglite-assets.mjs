#!/usr/bin/env node
/**
 * Nitro's Vercel bundle inlines `@electric-sql/pglite` but does not emit
 * `pglite.data` / `pglite.wasm` next to the chunk. Without DATABASE_URL the
 * function then crashes on boot (ENOENT pglite.data) and the whole site 500s.
 * Copy those files into the function output after `vite build`.
 */
import { copyFileSync, existsSync, mkdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { createRequire } from "node:module";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const require = createRequire(import.meta.url);

export function pgliteDistDir() {
  // package.json is not in "exports" — resolve the entry, then sit in dist/.
  return dirname(require.resolve("@electric-sql/pglite"));
}

export function vercelLibsDir(base = root) {
  return join(base, ".vercel/output/functions/__server.func/_libs");
}

const FILES = ["pglite.data", "pglite.wasm", "initdb.wasm"];

export function copyPgliteAssets({ base = root, log = console.log } = {}) {
  const destDir = vercelLibsDir(base);
  if (!existsSync(dirname(destDir))) {
    log("[copy-pglite-assets] no Vercel function output — skip");
    return { skipped: true, copied: [] };
  }
  const dist = pgliteDistDir();
  mkdirSync(destDir, { recursive: true });
  const copied = [];
  for (const name of FILES) {
    const from = join(dist, name);
    if (!existsSync(from)) {
      throw new Error(`missing ${from}`);
    }
    copyFileSync(from, join(destDir, name));
    copied.push(name);
  }
  log(`[copy-pglite-assets] copied ${copied.join(", ")} → ${destDir}`);
  return { skipped: false, copied };
}

const invoked = process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1];
if (invoked) {
  copyPgliteAssets();
}
