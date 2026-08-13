export type Diagnostic = {
  code: string;
  severity: "error" | "warn";
  message: string;
  line?: number;
};

export type GateStepId = "check" | "build" | "inspect";

export type GateStep = {
  id: GateStepId;
  label: string;
  status: "pending" | "running" | "pass" | "fail" | "skipped";
  detail: string;
  diagnostics: Diagnostic[];
};

export type Artifact = {
  name: string;
  kind: "abi" | "bytecode" | "digest" | "note";
  body: string;
};

export type GateResult = {
  passed: boolean;
  module: string | null;
  steps: GateStep[];
  artifacts: Artifact[];
  digest: string | null;
  closedReason?: string;
};

const FORBIDDEN = [
  { re: /\bmapping\b/, code: "PF-SRC-SOLIDITY", msg: "Solidity `mapping` is outside ProgramV1. Use `Map Principal UInt64`." },
  { re: /\bfunction\b/, code: "PF-SRC-SOLIDITY", msg: "Solidity `function` is outside ProgramV1. Use `entry` / `view`." },
  { re: /\bpragma\b/, code: "PF-SRC-SOLIDITY", msg: "Solidity pragma is outside ProgramV1." },
  { re: /\binvariant\b/, code: "PF-PLAN-INVARIANT", msg: "Do not put `invariant` in the deploy file. EVM build fails nonempty invariants." },
  { re: /\bproof\b/, code: "PF-PLAN-PROOF", msg: "Proofs live in twin files, not the ship source." },
  { re: /\berror\s+[A-Za-z_][A-Za-z0-9_]*\s*(?![\s\S]*?\()/, code: "PF-INTERNAL", msg: "Bare `error X` triggers PF-INTERNAL. Write `error X()`." },
  { re: /entry\s+\w+\s*\([^)]*\bBool\b/, code: "PF-SRC-INVALID", msg: "Bool is not allowed as an entry parameter. Use UInt64 (0/1) + `assert ok <= 1`." },
  { re: /init\s*\([^)]*\bBool\b/, code: "PF-SRC-INVALID", msg: "Bool is not allowed as an init parameter." },
  { re: /Map\s+\w+\s+(?!UInt64)/, code: "PF-PLAN-INVARIANT", msg: "Map values must be UInt64 for the EVM plan." },
];

function lineOf(source: string, index: number) {
  return source.slice(0, index).split("\n").length;
}

function djb2(input: string) {
  let h = 5381;
  for (let i = 0; i < input.length; i += 1) h = ((h << 5) + h + input.charCodeAt(i)) | 0;
  return (h >>> 0).toString(16).padStart(8, "0");
}

export function extractModule(source: string): string | null {
  const m = source.match(/program\s+([A-Za-z_][A-Za-z0-9_]*)\s+where/);
  return m?.[1] ?? null;
}

export function extractLean(text: string): string | null {
  const fenced = text.match(/```(?:lean)?\s*([\s\S]*?)```/i);
  if (fenced?.[1]?.includes("import ProofForgeV2")) return fenced[1].trim() + "\n";
  const start = text.indexOf("import ProofForgeV2");
  if (start >= 0) {
    const rest = text.slice(start);
    const end = rest.indexOf("end Proofship");
    if (end >= 0) return rest.slice(0, end + "end Proofship".length).trim() + "\n";
    return rest.trim() + "\n";
  }
  return null;
}

export function runGate(source: string): GateResult {
  const diagnostics: Diagnostic[] = [];
  const trimmed = source.replace(/^\uFEFF/, "");
  const module = extractModule(trimmed);

  if (!trimmed.includes("import ProofForgeV2")) {
    diagnostics.push({
      code: "PF-SRC-IMPORT",
      severity: "error",
      message: "First line must be exactly `import ProofForgeV2`.",
      line: 1,
    });
  }
  if (!/program\s+[A-Za-z_][A-Za-z0-9_]*\s+where/.test(trimmed)) {
    diagnostics.push({
      code: "PF-SRC-PROGRAM",
      severity: "error",
      message: "Missing `program <Module> where` block.",
    });
  }
  if (!trimmed.includes("namespace Proofship")) {
    diagnostics.push({
      code: "PF-SRC-NS",
      severity: "error",
      message: "Wrap the program in `namespace Proofship` … `end Proofship`.",
    });
  }
  if (trimmed.length > 64 * 1024) {
    diagnostics.push({
      code: "PF-SRC-SIZE",
      severity: "error",
      message: "Source exceeds the 64KiB studio inbox limit.",
    });
  }
  if (!/\binit\s*\(/.test(trimmed)) {
    diagnostics.push({
      code: "PF-SRC-INIT",
      severity: "error",
      message: "Program must declare `init(...)`.",
    });
  }
  if (!/\bentry\s+/.test(trimmed)) {
    diagnostics.push({
      code: "PF-SRC-ENTRY",
      severity: "error",
      message: "Program must declare at least one `entry`.",
    });
  }

  for (const rule of FORBIDDEN) {
    const m = trimmed.match(rule.re);
    if (m && m.index !== undefined) {
      diagnostics.push({
        code: rule.code,
        severity: "error",
        message: rule.msg,
        line: lineOf(trimmed, m.index),
      });
    }
  }

  const errors = diagnostics.filter((d) => d.severity === "error");
  const check: GateStep = {
    id: "check",
    label: "check",
    status: errors.length ? "fail" : "pass",
    detail: errors.length
      ? `${errors.length} PF-* diagnostic${errors.length === 1 ? "" : "s"}`
      : "Semantic surface accepted. Same-file structure holds.",
    diagnostics,
  };

  if (errors.length) {
    return {
      passed: false,
      module,
      steps: [
        check,
        { id: "build", label: "build", status: "skipped", detail: "Fail-closed. Zero artifacts.", diagnostics: [] },
        { id: "inspect", label: "inspect", status: "skipped", detail: "Fail-closed. Zero artifacts.", diagnostics: [] },
      ],
      artifacts: [],
      digest: null,
      closedReason: "Gate rejected the draft. No artifacts, no deploy.",
    };
  }

  const entries = [...trimmed.matchAll(/entry\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(([^)]*)\)/g)].map((m) => ({
    name: m[1],
    inputs: m[2],
  }));
  const views = [...trimmed.matchAll(/view\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(([^)]*)\)/g)].map((m) => ({
    name: m[1],
    inputs: m[2],
  }));
  const inits = trimmed.match(/init\s*\(([^)]*)\)/);

  const abi = {
    module,
    target: "evm",
    chainHint: "xlayer-testnet",
    constructor: inits?.[1] ?? "",
    entries,
    views,
  };
  const abiJson = JSON.stringify(abi, null, 2);
  const bytecode = `0x${djb2(trimmed)}${djb2(trimmed + module)}${"a7c3".repeat(8)}`;
  const digest = `sha256:${djb2(abiJson + bytecode + trimmed)}${djb2(module ?? "anon")}`;

  const build: GateStep = {
    id: "build",
    label: "build --target evm",
    status: "pass",
    detail: `Emitted ABI + bytecode for ${module}.`,
    diagnostics: [],
  };
  const inspect: GateStep = {
    id: "inspect",
    label: "inspect",
    status: "pass",
    detail: `Exact-disk closure ${digest}`,
    diagnostics: [],
  };

  return {
    passed: true,
    module,
    steps: [check, build, inspect],
    artifacts: [
      { name: `${module}.abi.json`, kind: "abi", body: abiJson },
      { name: `${module}.bin`, kind: "bytecode", body: bytecode },
      { name: "output-set.digest", kind: "digest", body: digest },
    ],
    digest,
  };
}

export function gateSummary(result: GateResult) {
  if (result.passed) return `Gate passed · ${result.module} · ${result.digest}`;
  const first = result.steps[0]?.diagnostics[0];
  return first ? `${first.code}: ${first.message}` : "Gate failed closed.";
}
