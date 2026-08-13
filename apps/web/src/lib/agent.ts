/**
 * Drafting does not run in this web process.
 * The desktop UserExecutor (comet) owns the agent + real ProofForge gate.
 * This module is kept only so leftover imports fail closed instead of
 * calling a cloud model.
 */

export type DraftResult =
  | { ok: true; source: string; note: string; via: "desktop" }
  | { ok: false; error: string; ask?: string };

export async function draftProgram(): Promise<DraftResult> {
  return {
    ok: false,
    error: "This page does not run an agent.",
    ask: "Open desktop ProofShip. Prompts go to your local machine over the relay. Keys never leave that machine.",
  };
}
