# ProofShip

**AI drafts the contract. The gate decides if it ships.**

![Zeron driving a Claude Code session with a live branch diff sidebar](apps/landing/public/assets/app-screenshot.jpg)

ProofShip is a local-first desktop app for building and deploying Web3 products:
you describe a contract in natural language, a coding agent of your choice
drafts it as a [ProofForge](https://github.com/DaviRain-Su/proof_forge)
ProgramV1 source, and a
machine gate — `check → build → inspect` — decides whether anything ships.
No gate pass, no artifacts, no deploy. First target chain: **X Layer**.

> AI 起草合约,门禁决定它能不能上链。ProofShip 是一个本地优先的桌面 app:
> 自然语言描述需求,你选择的 code agent 起草 ProofForge ProgramV1 源,
> 机器门禁(check→build→inspect)决定产出与否——不过门禁,没有制品,没有部署。
> 首发目标链:X Layer。

## Why (AI × Web3)

AI can draft a smart contract in a minute; nothing about that says the contract
is allowed to ship. ProofShip's answer is a **machine-checked pre-deploy gate**
between the draft and the chain:

- **Agent drafting, your lane** — one ACP (Agent Client Protocol) layer drives
  whichever agent CLI you already use: Claude Code, Codex, Grok Build, Hermes,
  Pi, Cursor, OpenCode (`crates/harness`). In **Sessions**, the agent is automatically
  injected with the ProofForge skill and stdio MCP (`pf_check` / `pf_build` / `pf_artifacts`).
- **The gate is the authority** — semantic checks with `PF-*` diagnostics that
  feed back into the agent as a bounded repair loop, then EVM build and an
  exact-disk-closure inspect with content digests. Failing drafts produce
  **zero artifacts** (fail closed).
- **One-command deploy** — gated artifacts deploy to **X Layer testnet**
  (chainId 1952) via `proofship/scripts/deploy-xlayer-testnet.sh`; keys live
  only in env vars, never in the app or the repo.
- **Local-first, multi-device-ready** — desktop works offline. The engine
  **defaults** to the hosted ProofShip relay so web Sessions at
  [proofship-web.pages.dev](https://proofship-web.pages.dev) can drive this
  machine (`PROOFSHIP_RELAY=off` to disable). Sync edge (`edge/`) still needs
  your own Worker if you want comet-style CRDT rooms.

## Quick start

```bash
# 1. Toolchain (vendored proof-forge-next + olean closure + locked chain tools)
proofship/scripts/install-toolchain.sh

# 2. Run the machine gate on the bundled regression sample
proofship/scripts/gate.sh crates/engine/tests/fixtures/rwa_share_registry.lean RwaShareRegistry

# 3. Deploy a gate-passing contract to X Layer testnet (your own funded key;
#    never written to any file — see the script's discipline header)
export PF_XLAYER_KEY=<hex>
PF_XLAYER_CONFIRM=yes PF_XLAYER_PRIVATE_KEY_ENV=PF_XLAYER_KEY \
  proofship/scripts/deploy-xlayer-testnet.sh \
  crates/engine/tests/fixtures/rwa_share_registry.lean RwaShareRegistry \
  'constructor(uint64,uint64,uint64)' 1000000 50000 100000
```

The desktop app (Rust + gpui): `cargo run -p comet` — sidebar **Sessions** runs NL → agent draft with ProofForge skill + MCP (`pf_check` / `pf_build` / `pf_artifacts`) → machine gate; engine services power gate/deploy/preview; see `docs/proofship-studio.md`.

## Repository map

```text
apps/comet            the desktop binary (headed + headless engine daemon)
apps/web              Sessions companion (TanStack landing + local gate demo)
apps/landing          marketing site (proofship.pages.dev)
apps/ios              iOS companion
crates/               proto · doc · sync · harness · engine · rpc · ui (gpui)
proofship/            platform pieces: toolchain installer, gate + deploy
                      scripts, local bridge reference, Cloudflare relay
proofship/web         live Sessions viewer (proofship-web.pages.dev)
edge/                 TypeScript Worker + Durable Objects (sync edge)
docs/                 architecture, studio spec, competition materials
```

## Honesty boundary

The gate is **engineering-grade** machine verification (semantic checks +
same-file theorem certification). We do **not** claim full formal verification,
proven bytecode, or securities compliance. Deploy keys never touch the app or
the repository.

## Status

Built for the **OKX Build X Series — AI Season** hackathon
(submission materials: `docs/competition/`). Licensed under MIT.
