# Cloudflare Computer spike (not production)

`@cloudflare/computer` is a **preview** agent workspace (isolate + optional
container). ProofShip’s heavy ProofForge toolchain (olean, EVM build, inspect)
generally needs a **container** backend; isolate-only is fine for orchestration
and file staging, not for replacing gate.

## Spike goals

1. Use Computer as a thin orchestrator: accept NL, stage ProgramV1 files, call
   out to a Sandbox/container job for `pf_check` / `pf_build` / `pf_artifacts`.
2. Mirror the same relay events (`session.*`, `gate.*`) so the web Sessions UI
   does not need a second protocol.
3. Confirm billing/CPU limits vs Sandbox for gate-heavy workloads.

## Non-goals

- Not the production default PlatformExecutor (Sandbox is).
- Not a place to store user deploy keys.
- Does not unblock W5 — Ship Sandbox gate path first; keep this doc as a
  follow-on experiment.

## Suggested experiment outline

1. Minimal Worker that creates a Computer session per `sessionId`.
2. Forward viewer `cmd.prompt` → Computer; Computer shells into Sandbox for gate.
3. Compare wall time / failure modes against a pure Sandbox executor.
4. Write findings back into this file; only then consider product exposure.
