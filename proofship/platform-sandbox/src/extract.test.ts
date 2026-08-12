import { describe, expect, it } from "vitest";
import { MAX_SOURCE, extractSource, validModule } from "./extract.js";

const DEMO = `import ProofForgeV2

program Demo where
  state x : UInt64
`;

describe("validModule", () => {
  it("accepts Lean-style identifiers up to 64 chars", () => {
    expect(validModule("Demo")).toBe(true);
    expect(validModule("TimeLockPayout")).toBe(true);
    expect(validModule("A" + "x".repeat(63))).toBe(true);
  });

  it("rejects empty, leading digit, hyphen, and oversized names", () => {
    expect(validModule("")).toBe(false);
    expect(validModule("1Bad")).toBe(false);
    expect(validModule("Foo-Bar")).toBe(false);
    expect(validModule("A" + "x".repeat(64))).toBe(false);
  });
});

describe("extractSource", () => {
  it("extracts a lean fence with import ProofForgeV2 and program Name", () => {
    const nl = ["Draft this ProgramV1:", "```lean", DEMO.trimEnd(), "```"].join("\n");
    expect(extractSource(nl)).toEqual({ module: "Demo", source: DEMO.trim() });
  });

  it("extracts a lean4 fence the same way", () => {
    const nl = ["```lean4", DEMO.trimEnd(), "```"].join("\n");
    expect(extractSource(nl)?.module).toBe("Demo");
  });

  it("extracts bare source when the prompt is source-dominant", () => {
    const nl = `Please gate this.\n\n${DEMO}`;
    expect(extractSource(nl)).toEqual({ module: "Demo", source: DEMO.trim() });
  });

  it("rejects a fence missing import ProofForgeV2", () => {
    const nl = ["```lean", "program Demo where", "  state x : UInt64", "```"].join("\n");
    expect(extractSource(nl)).toBeNull();
  });

  it("rejects oversized source", () => {
    const pad = "-- " + "x".repeat(MAX_SOURCE);
    const nl = ["```lean", "import ProofForgeV2", "program Demo where", pad, "```"].join(
      "\n",
    );
    expect(extractSource(nl)).toBeNull();
  });

  it("rejects a bad module name (too long)", () => {
    const name = "A" + "x".repeat(64);
    const nl = ["```lean", "import ProofForgeV2", `program ${name} where`, "```"].join(
      "\n",
    );
    expect(extractSource(nl)).toBeNull();
  });

  it("rejects a hyphenated program name with no valid hint", () => {
    const nl = ["```lean", "import ProofForgeV2", "program Foo-Bar where", "```"].join(
      "\n",
    );
    expect(extractSource(nl)).toBeNull();
  });
});
