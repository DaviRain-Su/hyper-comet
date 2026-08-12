# Platform executor (Cloudflare Sandbox)

**PlatformExecutor** — Node/TypeScript client that speaks the same relay
contract as a UserExecutor (`role=platform` on `/ws/engine/:sessionId`).

Keys never live in the Sandbox image or Worker secrets for deploy signing.
Users deploy from a UserExecutor (desktop / VPS) or browser wallet.

## What it may run

| Job | Allowed |
|---|---|
| `gate` | Yes — `proof-forge-next check → build --target evm → inspect` when CLI + ProgramV1 source are present |
| `agent_draft` | Later — hosted ACP / HTTP MCP loop |
| `deploy_with_key` | **Never** — relay refuses `cmd.deploy` to platform; client also refuses |

## Local run against relay

```sh
# Terminal A — relay (local spike accepts any device token)
cd proofship/relay && npm install && npm run dev

# Terminal B — platform executor
cd proofship/platform-sandbox
npm install
PROOFSHIP_RELAY=http://127.0.0.1:8787 \
PROOFSHIP_DEVICE_TOKEN=dev \
PROOFSHIP_DEVICE_ID=platform-1 \
PROOFSHIP_SESSION_ID=demo \
npm start
```

Optional: bake or point at the product CLI so gate jobs actually run:

```sh
export PROOF_FORGE_CLI=/path/to/proof-forge-next   # or put proof-forge-next on PATH
# Prompt the web viewer with executor=platform and a ```lean fence containing
# `import ProofForgeV2` + `program ModuleName where …`
```

Without CLI or Lean source, the executor publishes an honest `session.agent`
note and `session.done` with `ok: false` / hint to provide ProgramV1 or use
UserExecutor.

### Scripts

| Script | What |
|---|---|
| `npm start` | `tsx src/index.ts` |
| `npm test` | vitest — extract / module / deploy-refusal / CLI order (no relay) |
| `npm run typecheck` | `tsc --noEmit` |
| `npm run build` | emit `dist/` |
| `npm run start:dist` | `node dist/index.js` |

### Env

| Var | Default |
|---|---|
| `PROOFSHIP_RELAY` | *(required)* Worker base URL |
| `PROOFSHIP_DEVICE_TOKEN` / `DEVICE_TOKEN` / `ENGINE_TOKEN` | empty (local spike OK) |
| `PROOFSHIP_DEVICE_ID` | `platform-1` |
| `PROOFSHIP_SESSION_ID` / `PROOFSHIP_LAUNCH_ID` / `LAUNCH_ID` | `default` |
| `PROOF_FORGE_CLI` / `PF_CLI` | resolve PATH / vendored toolchain |

## Contract behavior

1. Connect WS as `role=platform`; reconnect with exponential backoff.
2. `cmd.prompt` → `session.user`; if Lean source extractable + CLI →
   `gate.start` / `gate.done` / `artifact.sealed` (ABI + digest when present) /
   `session.done`.
3. `cmd.deploy` → `executor.refused` (`platform_executor_cannot_hold_deploy_keys`).
4. `cmd.cancel` / `cmd.steer` → polite note + `session.done`.
5. Always `cmd.ack` when the command carries `id`.

## Smoke

With relay running:

```sh
RELAY_URL=http://127.0.0.1:8787 node ../scripts/smoke-relay-platform.mjs
```

## Container sketch

```dockerfile
FROM node:22-bookworm-slim
WORKDIR /app
COPY package.json package-lock.json* ./
RUN npm install --omit=dev=false
COPY tsconfig.json tsconfig.build.json ./
COPY src ./src
RUN npm run build
# Optionally COPY proof-forge-next + olean cache into the image.
COPY entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh
ENTRYPOINT ["/entrypoint.sh"]
```

`entrypoint.sh` runs `node dist/index.js` (or `npx tsx src/index.ts` in
dev images). Pass `PROOFSHIP_RELAY` + device token at runtime.

## Routing

Relay `SessionRoom` prefers `preferredExecutor` from the last prompt. Web UI
executor radio sends `executor: "platform" | "user"` on `cmd.prompt`. Deploy
always resolves to `user`.

## Quotas (Phase 4+)

Bill on active CPU seconds and gate invocations after accounts land.

## Related

- Relay contract: [`../relay/README.md`](../relay/README.md)
- Computer spike (not production default): [`COMPUTER_SPIKE.md`](./COMPUTER_SPIKE.md)
