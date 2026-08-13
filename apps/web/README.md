# ProofShip web

Landing + remote Sessions panel. The agent and ProofForge gate run on
your computer. This page is a thin relay viewer.

Not a replacement for the live Cloudflare viewer at
[proofship-web.pages.dev](https://proofship-web.pages.dev) —
that stays in [`proofship/web`](../../proofship/web). This app is the
branded companion: marketing site, login, and a Sessions console that
can attach a desktop room (`?relay=&session=`).

**Deploy keys never transit the web.**

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
