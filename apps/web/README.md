# ProofShip Sessions companion

TanStack Start landing + Sessions demo. Official ProofShip brand
(dark `#06040a`, purple `#8b5cf6`, Geist, statue ASCII). Bilingual EN / 中文.

This is **not** the live relay viewer. That stays at
[`proofship/web`](../../proofship/web) and deploys to
[proofship-web.pages.dev](https://proofship-web.pages.dev).
This app is the polished companion: marketing landing, local sessions,
agent draft, and a fail-closed gate simulator. **Deploy keys never
transit the web.**

## Surfaces

- Landing — official sparse hero, screenshot, four features, ASCII close
- Auth — email/password (Google / X when configured)
- Sessions — template picker, transcript, Lean file cards, presence lamps
- Gate — static `PF-*` analyzer (not the Lean compiler). Templates such
  as RWA Share Registry are authored to pass; free-form drafts may fail closed
- Agent — optional `grok-4.5` draft via ProofForge skill; template fallback

## Honest boundary

| This app does | This app does not |
| --- | --- |
| Draft Lean / show diagnostics | Run the real ProofForge Lean toolchain |
| Simulate `check` fail-closed | Produce deployable artifacts |
| Keep keys off the wire | Hold or forward `PF_XLAYER_KEY` |

For a real gate + deploy, use desktop ProofShip (`cargo run -p comet`)
or the relay viewer with a connected UserExecutor.

## Local

```sh
cd apps/web
npm install
npm run dev          # http://127.0.0.1:8080
```

Optional: set `DATABASE_URL` to a Postgres / Neon URL. Unset, it uses
embedded PGLite.

```sh
npm run typecheck
npm run build
```

## Layout

```text
src/components/landing    official-brand landing
src/components/sessions   workspace, transcript, gate rail
src/lib/gate.ts           PF-* static analyzer
src/lib/templates.ts      RWA / escrow / token starters
src/lib/agent.ts          draft via grok-4.5 or template
migrations/               auth + sessions
```
