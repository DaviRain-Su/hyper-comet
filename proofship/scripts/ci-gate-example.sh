#!/usr/bin/env bash
# ProofShip — CI example: run gate.sh and assert gate-report.json is sealed.
#
# Usage (from repo root):
#   proofship/scripts/ci-gate-example.sh
#   proofship/scripts/ci-gate-example.sh path/to/file.lean ModuleName
#
# Exit 0 when the gate passes and gate-report.json has ok+certified.
# Suitable as a GitHub Actions step after install-toolchain.sh.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$root"

src="${1:-crates/engine/tests/fixtures/rwa_share_registry.lean}"
module="${2:-RwaShareRegistry}"

proofship/scripts/gate.sh "$src" "$module"

out_rel="studio-inbox/out-$(echo "$module" | tr '[:upper:]' '[:lower:]')"
proj="${PROOFSHIP_PROJECT_ROOT:-$root/proofship/inbox}"
report="$proj/$out_rel/gate-report.json"

[[ -f "$report" ]] || { echo "ci-gate: missing report at $report" >&2; exit 1; }

python3 - "$report" <<'PY'
import json, sys
path = sys.argv[1]
with open(path, encoding="utf-8") as f:
    report = json.load(f)
assert report.get("schemaVersion") == 1, report
assert report.get("ok") is True, report
assert report.get("certified") is True, report
digest = report.get("outputSetDigest")
print(f"ci-gate: PASS certified module={report.get('module')} digest={digest}")
# Markdown badge snippet for README / CI summary:
status = "passing"
color = "3fb950"
print(f"ci-gate: badge ![gate](https://img.shields.io/badge/gate-{status}-{color})")
PY
