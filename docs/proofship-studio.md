# ProofShip · gate / deploy services (formerly “Launch Studio”)

**Product UI (2026-08-12):** There is **no Studio chat mode**. The app is
Cursor-shaped: **Sessions** (normal ACP conversation) + ProofForge **skill** +
**MCP**. Users do not switch into a second chat room to ship contracts.

Engine code under `comet-engine::studio` remains the **service layer** for
gate / deploy / preview / networks — not a product conversation surface.
Historical Track C UI (`comet-ui::studio`) is kept in-tree but **not linked**
from the sidebar; deep links / history that pointed at Studio redirect to
Sessions.

## Product loop (Sessions)

```
NL in Sessions ──▶ ACP agent (skill + ProofForge MCP tools)
               ──▶ agent writes ProgramV1 / calls pf_check|pf_build|pf_artifacts
               ──▶ same transcript shows tools + source
               ──▶ deploy / preview: engine RPCs (side panels later; keys stay local)
```

Web uses the **same loop** via `proofship/relay/`: viewers observe Sessions-shaped
events and enqueue `cmd.prompt` / `cmd.deploy`; the UserExecutor (desktop/VPS)
runs enrich + gate; Platform Sandbox may gate but never holds deploy keys.

Enrichment lives in `studio/session_enrich.rs` and runs inside `drive_run`
(after the doc stores the raw user prompt): prepend
`.agents/skills/proofforge-program-v1/SKILL.md` and attach
`resolve_studio_mcp_servers` to `RunRequest.mcp_servers`.

## Engine: `comet-engine::studio` (services)

- **`StudioGate`** (`studio/gate.rs`) — validates (module regex, `import
  ProofForgeV2` prefix, ≤64KiB), stages into the studio inbox, runs
  `proof-forge-next check → build --target evm → inspect`.
- **`session_enrich` / `mcp`** — Sessions skill + stdio MCP wiring
  (`proofship/mcp/`; full PF MCP when `PROOF_FORGE_ROOT` is set).
- **`DraftRunner` / `StudioLaunchRunner`** — legacy orchestration (draft → gate
  → repair). Not the primary UX; Sessions + MCP replaces that chat path.
- **Deploy / Preview / Networks / Wallets** — RPC + stores for shipping and
  interacting; product entry is Settings / future side panels, not a chat tab.
- **RPC** — `StudioStatus`, `StudioGate`, `StudioDeploy`, preview/interact, etc.

## Deprecated UI

- Sidebar **Studio** toggle: removed.
- `Route::Studio` / `COMET_OPEN_ROUTE=studio`: redirects to Sessions.
- `StudioView` panel code: retained, unlinked.

## Honesty

Engineering-grade machine gate. Not full formal verification / bytecode-proven.
Keys never enter the app or relay.
