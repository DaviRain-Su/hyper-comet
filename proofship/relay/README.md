# ProofShip Relay (W1+)

Cloudflare Worker + Durable Object coordinating **Sessions-shaped** rooms between
web viewers and executors.

- **Room key:** `sessionId` (URL alias: `launchId` paths still work)
- **Roles:** `engine` (UserExecutor), `platform` (PlatformExecutor), `viewer`
- **Invariant:** private keys and deploy signing never transit this Worker

## Auth

| Role | How |
|---|---|
| Engine / platform | `?token=&deviceId=` matched against `DEVICE_TOKENS` JSON, or `DEVICE_TOKEN` / `ENGINE_TOKEN` (`*` fallback). Empty config accepts any token (local spike). |
| Viewer | Optional `VIEWER_TOKEN` → require `?viewerToken=` |

## WebSockets

```
GET /ws/engine/:sessionId?token=…&deviceId=…&role=engine|platform
GET /ws/web/:sessionId?viewerToken=…
```

Aliases: `/ws/session/engine/…`, `/ws/session/web/…`.

## Events (engine → relay → viewers)

Sessions-shaped (preferred):

```json
{"type":"event","kind":"session.user","payload":{"text":"…"}}
{"type":"event","kind":"session.agent","payload":{"text":"…"}}
{"type":"event","kind":"session.tool","payload":{"id":"…","call":{}}}
{"type":"event","kind":"session.done","payload":{"status":"…"}}
{"type":"event","kind":"executor.online","payload":{"role":"engine","deviceId":"…"}}
{"type":"event","kind":"deploy.done","payload":{"ok":true,"record":{}}}
```

Legacy projections still accepted: `draft.ready`, `gate.start`, `gate.done`,
`artifact.sealed`, `note`.

Relay assigns `{seq, ts}` and broadcasts `{type:"event", event:{…}}`.

## Viewer commands

Queued with TTL (~15m); drained to the selected online executor. Executors may
`{"type":"cmd.ack","id":"…"}`.

```json
{"type":"cmd.prompt","nl":"…","lane":"codex","executor":"user"|"platform","chatId":"…"}
{"type":"cmd.steer","nl":"…"}
{"type":"cmd.cancel"}
{"type":"cmd.deploy","networkId":"…","module":"…","digest":"…"}
```

`cmd.deploy` always targets the **user** executor. If the only online executor is
platform (or deploy is forced platform), relay emits `executor.refused`.

## HTTP snapshot

```
GET /api/sessions/:id/state
GET /api/launches/:id/state   # alias
```

Returns `{ state, tail, queueDepth }`. Snapshot includes `transcript`,
`executors`, `deployment`, gate/artifact fields.

## Read-only share (Phase 4.4 stub)

```
GET /api/share/:sessionId?token=…
```

Auth: Query parameter `token` matched against `SHARE_TOKEN` when set (the web client also passes `viewerToken` as a fallback). If `SHARE_TOKEN` is unset, falls back to `VIEWER_TOKEN` (`viewerToken` or `token`). When neither is set, local spike requests are accepted.
Response is redacted — gate / artifact / deployment / transcript only; no
command queue and no write WebSocket. Full SIWE + permissioned share links
remain Phase 4.

## Development

```sh
cd proofship/relay
npm install
npm run typecheck
npm run dev
```

## Deploy

```sh
cd proofship/relay
wrangler secret put DEVICE_TOKENS   # '{"laptop":"…"}' preferred
# or: wrangler secret put ENGINE_TOKEN
npm run deploy
```

## Follow-on (not W1)

SIWE / WorkOS accounts, share links, D1, billing quotas — see product-plan
Phase 4 and `proofship/platform-sandbox/`.
