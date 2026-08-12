import { join } from "node:path";
import { describe, expect, it } from "vitest";
import {
  PLATFORM_DEPLOY_REFUSED_REASON,
  cliCandidatePaths,
  refuseDeploy,
  shouldRefuseDeploy,
} from "./gate.js";

describe("cmd.deploy refusal", () => {
  it("always refuses cmd.deploy with platform_executor_cannot_hold_deploy_keys", () => {
    expect(shouldRefuseDeploy("cmd.deploy")).toBe(true);
    expect(shouldRefuseDeploy("cmd.prompt")).toBe(false);

    const payload = refuseDeploy({
      networkId: "xlayer-testnet",
      module: "Demo",
    });
    expect(payload.reason).toBe("platform_executor_cannot_hold_deploy_keys");
    expect(payload.reason).toBe(PLATFORM_DEPLOY_REFUSED_REASON);
    expect(payload.networkId).toBe("xlayer-testnet");
    expect(payload.module).toBe("Demo");
  });

  it("uses the same reason for every deploy payload", () => {
    for (const cmd of [
      { networkId: "a", module: "M" },
      { networkId: "b", module: "Other" },
    ]) {
      expect(refuseDeploy(cmd).reason).toBe(PLATFORM_DEPLOY_REFUSED_REASON);
      expect(shouldRefuseDeploy("cmd.deploy")).toBe(true);
    }
  });
});

describe("cliCandidatePaths", () => {
  it("orders explicit CLI, PATH, lake root, then vendored walk-up", () => {
    const cwd = "/home/user/proofship";
    const paths = cliCandidatePaths({
      proofForgeCli: "/opt/proof-forge-next",
      pathEnv: "/usr/bin:/usr/local/bin",
      proofForgeRoot: "/forge",
      cwd,
      maxWalkUp: 2,
    });
    expect(paths[0]).toBe("/opt/proof-forge-next");
    expect(paths[1]).toBe(join("/usr/bin", "proof-forge-next"));
    expect(paths[2]).toBe(join("/usr/local/bin", "proof-forge-next"));
    expect(paths[3]).toBe(join("/forge", ".lake/build/bin/proof-forge-next"));
    expect(paths[4]).toBe(join(cwd, "proofship/toolchain/bin/proof-forge-next"));
    expect(paths[5]).toBe(
      join(join(cwd, ".."), "proofship/toolchain/bin/proof-forge-next"),
    );
  });

  it("falls back to PF_CLI when PROOF_FORGE_CLI is unset", () => {
    const paths = cliCandidatePaths({
      pfCli: "/from/pf-cli",
      cwd: "/tmp",
      maxWalkUp: 0,
    });
    expect(paths[0]).toBe("/from/pf-cli");
  });
});
