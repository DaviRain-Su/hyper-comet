# ProofShip web (Sessions viewer)

Static Cloudflare Pages front end: **Sessions-shaped** observe + command surface
over [`../relay/`](../relay/). Executors run code and gate; this page never holds
deploy keys.

## Surfaces

- Connection + **executor picker** (UserExecutor desktop/VPS vs Platform Sandbox)
- Transcript tail (`session.user` / `session.agent` / `session.tool` / `session.done`)
- Composer → `cmd.prompt` / `cmd.steer` / `cmd.cancel`
- Deploy → `cmd.deploy` (UserExecutor only; platform refused by relay)
- Interact → viem `eth_call` + `window.ethereum` writes; fill from snapshot
- ProofForge HTTP MCP panel for **external IDE agents** (not the main chat)

## Local

```sh
cd proofship/web
python3 -m http.server 4173
```

Query params:

- `?relay=https://…&session=<id>`
- `?launch=<id>` (alias for session)
- `?viewerToken=…` when relay sets `VIEWER_TOKEN`
- `?executor=platform` to preselect Platform
- `?pfMcp=https://…/mcp`

## Deploy (Pages)

```sh
npx wrangler pages deploy proofship/web --project-name proofship-web
```

## Honest degradation

If the chosen executor is offline, the page stays connected but **read-only**
and says to open desktop ProofShip or choose Platform. Empty rooms show `{}`.

## Auth contract

Viewer: optional `viewerToken` query param. Engine/platform: per-device token on
the executor WebSocket (see relay README). SIWE / share links are Phase 4
follow-on.
