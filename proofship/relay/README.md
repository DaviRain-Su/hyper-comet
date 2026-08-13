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
| Viewer | SIWE session (`Authorization: Bearer` or `?sessionToken=`) **or** optional `VIEWER_TOKEN` (`?viewerToken=`). Local spike with neither still accepts. |

### SIWE (Phase 4.2)

Wallet address is the account. The signature is a login, **not** a deploy key.

```
GET  /api/auth/siwe/nonce?address=0x…&chainId=1952
POST /api/auth/siwe/verify          { message, signature } → { token, address, expiresAt }
GET  /api/auth/me                   Authorization: Bearer <token>
POST /api/auth/logout
POST /api/sessions/:id/share        { role?: readonly|comment|command }
GET  /api/orgs
POST /api/orgs                      { name }
POST /api/orgs/:id/select
GET  /api/orgs/:id/members
POST /api/orgs/:id/members          { address, role?: admin|member }
DELETE /api/orgs/:id/members?address=
POST /api/sessions/:id/claim        bind room to the active org
POST /api/sessions/:id/comments     { text }  (comment or command role)

Share roles: `readonly` observe · `comment` transcript notes (no executor) ·
`command` prompt/steer/cancel/deploy. Org members of a claimed room get command.
```

Sessions persist in D1 when `DB` is bound (`schema.sql`); otherwise an in-memory
store (local spike / tests). Tokens are stored hashed. CORS is enabled on `/api/*`.

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

## Read-only share

```
GET /api/share/:sessionId?token=…
```

Auth, in order: minted SIWE share token for that session → owner session
token → `SHARE_TOKEN` / `VIEWER_TOKEN` stub → local spike if none set.
Response is redacted — gate / artifact / deployment / transcript only; no
command queue and no write WebSocket. Comment / command share roles are still
follow-on.

## Development

```sh
cd proofship/relay
npm install
npm run typecheck
npm run dev
```

## Deploy

Live (2026-08-13):

- Worker: `https://proofship-relay.davirain-yin.workers.dev`
- D1: `proofship-accounts` bound as `DB`
- SIWE domain: `proofship-web.pages.dev`

```sh
cd proofship/relay
# wrangler secret put DEVICE_TOKENS   # '{"laptop":"…"}' for executors
npx wrangler d1 execute proofship-accounts --remote --file=schema.sql
npx wrangler deploy
```

## Follow-on

WorkOS email/OAuth (already on `edge/`), D1 production bind, comment/command
share roles, billing quotas — see product-plan Phase 4.
