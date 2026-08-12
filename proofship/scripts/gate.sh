#!/usr/bin/env bash
# ProofShip — AI gate loop helper (vertical-agnostic).
# One command an agent (or the Studio backend) calls after writing a candidate
# ProgramV1 source file: check → build --target evm → inspect exact closure.
#
# Usage:
#   proofship/scripts/gate.sh <path-to-file.lean> [ModuleName]
# Example:
#   proofship/scripts/gate.sh crates/engine/tests/fixtures/rwa_share_registry.lean RwaShareRegistry
#
# The source is staged into ${PROOFSHIP_PROJECT_ROOT:-<repo>/proofship/inbox}
# and the gate runs there, so any directory can serve as the project root.
#
# Exit 0 = gate passed (safe to deploy lane). Non-zero = gate rejected; stdout
# shows the PF-* diagnostics to feed back into the agent repair loop.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$root"
proj="${PROOFSHIP_PROJECT_ROOT:-$root/proofship/inbox}"

src_arg="${1:-}"
[[ -n "$src_arg" && -f "$src_arg" ]] || { echo "gate: no such source file: ${src_arg:-<none>}" >&2; exit 64; }
module="${2:-$(basename "$src_arg" .lean)}"

resolve_pf_cli() {
  if [[ -n "${PF_CLI:-}" ]]; then
    [[ -x "$PF_CLI" ]] && { printf '%s\n' "$PF_CLI"; return 0; }
    return 1
  fi
  local path_cli
  path_cli="$(command -v proof-forge-next 2>/dev/null || true)"
  if [[ -n "$path_cli" && -x "$path_cli" ]]; then
    printf '%s\n' "$path_cli"
    return 0
  fi
  if [[ -n "${PROOF_FORGE_ROOT:-}" && -x "$PROOF_FORGE_ROOT/.lake/build/bin/proof-forge-next" ]]; then
    printf '%s\n' "$PROOF_FORGE_ROOT/.lake/build/bin/proof-forge-next"
    return 0
  fi
  if [[ -x "$root/proofship/toolchain/bin/proof-forge-next" ]]; then
    printf '%s\n' "$root/proofship/toolchain/bin/proof-forge-next"
    return 0
  fi
  return 1
}
cli="$(resolve_pf_cli)" || { printf 'gate: product CLI missing. Resolve it by one of:
  1) PF_CLI=/absolute/path/to/proof-forge-next
  2) put proof-forge-next on PATH
  3) PROOF_FORGE_ROOT=/path/to/proof_forge (uses .lake/build/bin/proof-forge-next)
  4) install vendored toolchain: proofship/scripts/install-toolchain.sh [dist.tar.gz]
' >&2; exit 70; }

# Pin the runtime Lean toolchain to the CLI's build pin: the CLI resolves the
# Lean sysroot + its package oleans through elan, and a newer active default
# fails at runtime with "Init.olean: incompatible header".
if [[ -z "${ELAN_TOOLCHAIN:-}" ]]; then
  for pin in "$root/proofship/toolchain/lean-toolchain" "${PROOF_FORGE_ROOT:-}/lean-toolchain"; do
    if [[ -f "$pin" ]]; then
      ELAN_TOOLCHAIN="$(tr -d '[:space:]' <"$pin")"
      export ELAN_TOOLCHAIN
      break
    fi
  done
fi

# Point the CLI at the vendored locked tool-root (solc/sbpf/wat2wasm) unless
# the operator already provides one.
if [[ -z "${PROOF_FORGE_TOOL_ROOT:-}" ]]; then
  platform="$(uname -s | tr '[:upper:]' '[:lower:]')-$(uname -m)"
  tool_root="$root/proofship/toolchain/tool-root/$platform"
  [[ -d "$tool_root" ]] && export PROOF_FORGE_TOOL_ROOT="$tool_root"
fi

mkdir -p "$proj/studio-inbox"
cp "$src_arg" "$proj/studio-inbox/$module.lean"
rel_source="studio-inbox/$module.lean"

echo "== gate: check ==" >&2
"$cli" check "$rel_source" --module "$module" --root "$proj"

echo "== gate: build (evm) ==" >&2
out="studio-inbox/out-$(echo "$module" | tr '[:upper:]' '[:lower:]')"
rm -rf "$proj/$out"  # product build fails closed on pre-existing output dir
"$cli" build "$rel_source" --module "$module" --root "$proj" --target evm -o "$out"

echo "== gate: inspect (exact disk closure) ==" >&2
inspect_out="$("$cli" inspect --output-dir "$proj/$out")"
printf '%s\n' "$inspect_out"
digest="$(printf '%s\n' "$inspect_out" | sed -n 's/.*outputSetDigest[^0-9a-fA-F]*\([0-9a-fA-F]\{64\}\).*/\1/p' | head -n1)"
generated_at="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
report="$proj/$out/gate-report.json"
{
  printf '{\n'
  printf '  "schemaVersion": 1,\n'
  printf '  "ok": true,\n'
  printf '  "module": "%s",\n' "$module"
  printf '  "target": "evm",\n'
  if [[ -n "$digest" ]]; then
    printf '  "outputSetDigest": "%s",\n' "$digest"
  else
    printf '  "outputSetDigest": null,\n'
  fi
  printf '  "artifacts": [],\n'
  printf '  "certified": true,\n'
  printf '  "honesty": "Engineering-grade machine gate (check/build/inspect + same-file theorem certification). Not full formal verification or bytecode-proven.",\n'
  printf '  "generatedAt": "%s"\n' "$generated_at"
  printf '}\n'
} >"$report"

echo "gate: PASS $module → $proj/$out (report: $report)" >&2
