# ProofShip

**AI drafts the contract. The gate decides if it ships.**

![Zeron driving a Claude Code session with a live branch diff sidebar](apps/landing/public/assets/app-screenshot.jpg)

ProofShip is a local-first desktop app for building and deploying Web3 products:
you describe a contract in natural language, a coding agent of your choice
drafts it as a [ProofForge](https://github.com/DaviRain-Su/proof_forge)
ProgramV1 source, and a machine gate decides whether anything ships.
First target chain: **X Layer**.

> AI 起草合约,门禁决定它能不能上链。ProofShip 是一个本地优先的桌面 app。

## What this is

The app is the comet desktop (Sessions + ACP agents) plus two settings
modules: **Networks** and **Wallets**. ProofForge is meant to be used as a
Skill / MCP server — not as an embedded Studio. There is no browser Sessions
client; remote control uses the existing comet edge when you self-host it.

```bash
cargo run -p comet
```

## Repository map

```text
apps/comet            the desktop binary (headed + headless engine daemon)
apps/landing          marketing site
apps/ios              iOS companion
crates/               proto · doc · sync · harness · engine · rpc · ui (gpui)
edge/                 TypeScript Worker + Durable Objects (sync edge)
.agents/skills/       ProofForge skill drafts (MCP wiring comes later)
docs/                 architecture + competition materials
```

## Honesty boundary

We do **not** claim full formal verification, proven bytecode, or securities
compliance. Deploy keys stay in your environment or wallet — never in the app
or the repository.

Licensed under MIT.
