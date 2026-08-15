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

The app is the zeron desktop (Sessions + ACP agents) plus two settings
modules: **Networks** and **Wallets**. ProofForge is meant to be used as a
Skill / MCP server — not as an embedded Studio. There is no browser Sessions
client; remote control uses the existing zeron edge when you self-host it.

```bash
cargo run -p zeron
```

Every device runs a small engine that stores sessions on that device. A new
installation starts in local-only mode without an account or a network
connection.

## Install and run locally

```bash
cargo run -p zeron
zeron status
```

`zeron daemon install` starts the engine immediately and keeps it running
across reboots. No sign-in or sync configuration is required.

Day-to-day:

```bash
zeron status      # local/synced mode and engine status
zeron update      # update to the latest release
zeron daemon start|stop|restart|status
```

## Optional multi-device sync

Sign in only when you want to open a synced workspace. Authentication changes
the profile selected by the next engine start, so stop the daemon before
changing it:

```bash
zeron daemon stop
zeron login
zeron daemon start
```

You can then start an agent on one synced device and follow or drive it from
another. An always-on machine such as a VPS can keep those agents working
after you close your laptop.

Signing in does not upload, move, or import existing local sessions. Local
sessions and their attachments remain under the local profile and reappear
when you return to local-only mode:

```bash
zeron daemon stop
zeron logout
zeron daemon start
```

`zeron login` and `zeron logout` refuse to modify credentials while an engine
owns the data directory. The desktop app follows the same next-restart
profile boundary.

On macOS: build `zeron` from source and run `zeron daemon install` to install
the launchd service.

## Repository map

```text
apps/zeron            the desktop binary (headed + headless engine daemon)
apps/landing          marketing site
apps/ios              iOS companion
crates/               proto · doc · sync · harness · engine · rpc · kit · ui (gpui)
                      kit = theme / icons / fonts; ui = product screens on top of kit
crates/pf-mcp/        proofship-pf-mcp — ProofForge gate as an MCP server (rmcp, stdio)
edge/                 TypeScript Worker + Durable Objects (sync edge)
skills/               agent skills (ProgramV1, EVM, Cloudflare — embedded into the engine)
docs/                 architecture + competition materials
```

## Honesty boundary

We do **not** claim full formal verification, proven bytecode, or securities
compliance. Deploy keys stay in your environment or wallet — never in the app
or the repository.

Licensed under MIT.
