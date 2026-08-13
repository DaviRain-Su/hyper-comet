# ProofShip web

Landing + remote Sessions panel. **Deploy this app on Vercel.**
The Cloudflare Worker (`proofship-relay`) stays the message pipe;
this frontend is not a Cloudflare Pages project.

The agent and ProofForge gate run on the user's computer. This page
attaches a desktop room (`?relay=&session=desktop-{deviceId}`).
That room is bound to one computer and does not change when you
click New session. **Deploy keys never transit the web.**

## Surfaces

- Landing — landscape hero, desktop screenshot, Web Sessions demo, download
- Auth — email / password (Google / X when configured)
- Sessions — Desktop / Platform / Relay lamps, Send / Steer / Comment,
  gate traces, snapshot, event tail, MCP

## Local

```sh
cd apps/web
npm install
npm run dev          # http://127.0.0.1:8080
```

## Deploy on Vercel

1. New Project → import this GitHub repo.
2. **Root Directory:** `apps/web` (not the repo root).
3. Build is already `npm run build` (Vite + Nitro `vercel` preset).
4. Environment variables:

| Name | Required | Notes |
| --- | --- | --- |
| `BETTER_AUTH_URL` | yes in prod | `https://<your-app>.vercel.app` (no trailing slash) |
| `BETTER_AUTH_SECRET` | yes in prod | long random string |
| `DATABASE_URL` | recommended | Vercel Postgres / Neon. Unset = PGLite (wiped on cold start) |

5. After the first deploy, on the machine running the daemon:

```sh
export PROOFSHIP_WEB=https://<your-app>.vercel.app
# then reinstall so the unit keeps the URL
cargo run -p comet -- daemon install
cargo run -p comet -- agent url
```

`comet agent url` will then open **this** Vercel site with
`?relay=…&session=desktop-…`. Relay stays
`https://proofship-relay.davirain-yin.workers.dev`.
