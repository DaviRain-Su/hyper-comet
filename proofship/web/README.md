# ProofShip web shell (Phase 3.1)

Static Cloudflare Pages front end for observing a local Studio launch through
[`../relay/`](../relay/). The desktop engine is still the only writer and gate
executor; this page is a viewer (+ later command enqueue).

## Local

Open `index.html` via any static server:

```sh
cd proofship/web
python3 -m http.server 4173
```

Query params: `?relay=https://…&launch=<id>`.

## Deploy (Pages)

```sh
npx wrangler pages deploy proofship/web --project-name proofship-web
```

Point the UI at a deployed relay Worker from `proofship/relay`.

## Honest degradation

With no engine / empty relay room the snapshot stays `{}` and the status line
says the page is read-only. Do not claim a live product URL until a real
testnet deploy + relay are wired.
