# ProofShip Launch Studio — native GUI implementation spec

Status: Track C landed through C3. C1a (engine gate + store + RPC), C1b (Studio
panel), C2 (ACP harness drafting + lane picker), and C3 (bounded gate-repair loop) landed. Source of truth for
product behavior: `proofship/README.md` (vision) + `proofship/bridge/server.mjs`
(the reference local bridge the engine replaces).

**Product framing (2026-08-12):** ProofShip is a general web3 product
dev/deploy app — agents draft any ProgramV1 contract, the ProofForge machine
gate decides what ships (first chain: X Layer). The RWA hackathon vertical was
removed from this repo (it remains in proof_forge for reference); the Studio is
vertical-agnostic.

## Product loop

```
NL input ──▶ draft via agent lane (harness ACP lanes)
         ──▶ gate: proof-forge-next check → build --target evm → inspect
         ──▶ pass: artifact summary (+ later: deploy lane)  fail: PF-* diagnostics → repair
```

## Engine: `comet-engine::studio` (landed, C1a + C2)

- **`StudioGate`** (`studio/gate.rs`) — validates (module regex, `import
  ProofForgeV2` prefix, ≤64KiB), stages into the studio inbox
  (`<data_dir>/studio/inbox`, overridable via `StudioPaths.inbox_root`),
  streams `Started/StageDone/Done` per stage, 240s timeout, capped output
  tails, artifact list + `outputSetDigest` parse. CLI/env resolution mirrors
  `proofship/scripts/gate.sh` (`PF_CLI` → PATH → `PROOF_FORGE_ROOT` → vendored
  toolchain; `ELAN_TOOLCHAIN` / `PROOF_FORGE_TOOL_ROOT` pins).
- **`StudioStore`** (`studio/store.rs`) — `<data_dir>/studio/launches.json`,
  camelCase wire shapes, cap 20, atomic writes. Launch/draft shapes are
  vertical-agnostic (`fields` is an opaque key→value summary table).
- **`DraftRunner`** (`studio/draft.rs`) — creates `<inbox_root>/agent/<id>/`,
  drives the selected installed ACP harness with the generic ProgramV1 authoring
  prompt, includes the resolved `proof-forge-next` path plus exact self-check
  commands, collects exactly one `.lean` file, validates it with the same source
  contract as the gate, and streams `Started/Note/Done` draft events.
- **`StudioLaunchRunner`** (`studio/launch_run.rs`) — runs draft → gate and,
  on a failed gate, re-prompts the same lane with the original NL, failed source,
  and the failing PF-* diagnostics capped to ~4KiB. It re-gates each revised
  source, stops on pass, or exhausts after 4 rounds / 30 minutes total.
- **RPC** — `StudioStatus`, `StudioDraft` (stream), `StudioGate` (stream),
  `StudioLaunchRun` (stream), `StudioLaunches`, `StudioPutLaunches`
  (device-local forwarding class).

## UI: `comet-ui::studio` (landed, C1b + C2)

- **Entry**: rail toggle Sessions ↔ Studio; one Studio thread per workspace in
  v1, persisted via the launch store RPCs.
- **Thread** (virtualized, existing transcript machinery):
  - user NL message row;
  - **draft card**: module name, fields summary table (opaque map), source in
    the existing markdown code renderer, lane note ("drafted by codex");
  - **gate card**: 3-stage pipeline (check → build → inspect) with live
    spinner → pass/fail per stage; failing stage expands its `PF-*`
    diagnostics; pass shows the artifact manifest from `inspect`.
  - launch-run rounds show compact `R1`/`R2` badges on draft and gate cards; if
    all repairs fail, the thread ends with `repair exhausted after 4 rounds —
    last diagnostics above`.
- **Composer**: multiline NL input + lane picker (harness picker component +
  registry install probes). NL submit calls `StudioLaunchRun`; manual "Gate
  source…" and "Load sample" remain single-source utilities.
- **Sample source** for demos/tests without an agent CLI:
  `crates/engine/tests/fixtures/rwa_share_registry.lean` (gate regression
  fixture) can be loaded as a demo draft.

## Slices

1. ~~C1a engine gate + store + RPC~~ ✅ (e2e: `studio_gate_real_toolchain_passes`,
   digest-identical to `gate.sh`).
2. ~~C1b gpui Studio panel~~ ✅ (thread/draft card/gate card/composer) on the
   C1a RPCs, including manual paste/sample fixture.
3. ~~C2 `DraftRunner`~~ ✅ harness-lane drafting in a scratch workdir via ACP
   harnesses, lane picker wiring, draft note, and automatic draft → gate.
4. ~~C3 gate-repair loop~~ ✅ bounded draft → gate → repair ≤4 orchestration,
   round-aware UI rows, and final exhaustion note. Deploy lane (X Layer) and web
   app (`proofship/relay/` seam) are later phases; keys never enter the app.

## Non-goals (v1)

- No deploy endpoint, no private keys.
- No relay/web viewer (web app phase).
- Studio **Preview** (right pane): ABI → local HTML dapp on `127.0.0.1`; Start
  opens the **system browser**. In-pane ABI mirror for no-arg views. Embedded
  WebView belongs in the web app later (gpui cannot host a reliable child
  WebView on Linux/Wayland).
- No per-vertical templates in the engine (removed with the RWA vertical; a
  data-driven template system may return as its own feature).
