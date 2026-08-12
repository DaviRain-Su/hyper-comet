/**
 * Pure PlatformExecutor policy: deploy refusal + CLI resolution order.
 * Keys never live on the platform; deploy is always refused here.
 */
import { join } from "node:path";

export const PLATFORM_DEPLOY_REFUSED_REASON =
  "platform_executor_cannot_hold_deploy_keys" as const;

export const PLATFORM_DEPLOY_REFUSED_HINT =
  "Connect a user desktop/VPS executor and deploy there (wallet or DevEnvKey). Keys never live on platform.";

export interface DeployRefusal {
  reason: typeof PLATFORM_DEPLOY_REFUSED_REASON;
  hint: string;
  networkId: string;
  module: string;
}

/** `cmd.deploy` is always refused on PlatformExecutor. */
export function shouldRefuseDeploy(type: string): boolean {
  return type === "cmd.deploy";
}

export function refuseDeploy(cmd: { networkId: string; module: string }): DeployRefusal {
  return {
    reason: PLATFORM_DEPLOY_REFUSED_REASON,
    hint: PLATFORM_DEPLOY_REFUSED_HINT,
    networkId: cmd.networkId,
    module: cmd.module,
  };
}

export interface CliResolveInput {
  proofForgeCli?: string;
  pfCli?: string;
  pathEnv?: string;
  proofForgeRoot?: string;
  cwd: string;
  maxWalkUp?: number;
}

/**
 * Candidate paths for `proof-forge-next`, in resolution order:
 * 1. PROOF_FORGE_CLI (else PF_CLI)
 * 2. each PATH entry + `/proof-forge-next`
 * 3. PROOF_FORGE_ROOT `.lake/build/bin/proof-forge-next`
 * 4. walk up from cwd for vendored `proofship/toolchain/bin/proof-forge-next`
 */
export function cliCandidatePaths(input: CliResolveInput): string[] {
  const out: string[] = [];
  const seen = new Set<string>();
  const push = (p: string): void => {
    if (!p || seen.has(p)) return;
    seen.add(p);
    out.push(p);
  };

  const explicit = input.proofForgeCli || input.pfCli;
  if (explicit) push(explicit);

  for (const dir of (input.pathEnv ?? "").split(":")) {
    if (!dir) continue;
    push(join(dir, "proof-forge-next"));
  }

  if (input.proofForgeRoot) {
    push(join(input.proofForgeRoot, ".lake/build/bin/proof-forge-next"));
  }

  let dir = input.cwd;
  const max = input.maxWalkUp ?? 6;
  for (let i = 0; i < max; i++) {
    push(join(dir, "proofship/toolchain/bin/proof-forge-next"));
    const parent = join(dir, "..");
    if (parent === dir) break;
    dir = parent;
  }
  return out;
}
