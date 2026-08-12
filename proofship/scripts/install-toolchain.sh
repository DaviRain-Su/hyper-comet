#!/usr/bin/env bash
# Install a ProofForge CLI distribution tarball into proofship/toolchain/.
#
# Usage:
#   proofship/scripts/install-toolchain.sh [dist.tar.gz]
#   PF_DIST_TARBALL=/path/to/dist.tar.gz proofship/scripts/install-toolchain.sh
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
product_root="$(cd "$here/.." && pwd)"
toolchain="$product_root/toolchain"

default_tar="/home/davirain/proof_forge/dist/proof-forge-next-0.1.0-linux-x86_64.tar.gz"
tarball="${1:-${PF_DIST_TARBALL:-$default_tar}}"

die() { echo "install-toolchain: FAIL: $*" >&2; exit 1; }

[[ -f "$tarball" ]] || die "tarball not found: $tarball (pass path as argv[1] or PF_DIST_TARBALL)"

first="$(/usr/bin/python3 - "$tarball" <<'PY'
import sys, tarfile
with tarfile.open(sys.argv[1], 'r:gz') as tf:
    first = next(iter(tf), None)
    print('' if first is None else first.name.split('/', 1)[0])
PY
)"
[[ -n "$first" ]] || die "empty tarball: $tarball"
/usr/bin/python3 - "$tarball" "$first/bin/proof-forge-next" <<'PY' \
  || die "tarball layout unsupported: expected $first/bin/proof-forge-next"
import sys, tarfile
with tarfile.open(sys.argv[1], 'r:gz') as tf:
    want = sys.argv[2]
    raise SystemExit(0 if any(m.name == want for m in tf.getmembers()) else 1)
PY

rm -rf "$toolchain"
mkdir -p "$toolchain"
tar -xzf "$tarball" -C "$toolchain" --strip-components=1
[[ -x "$toolchain/bin/proof-forge-next" ]] || die "installed binary is not executable: $toolchain/bin/proof-forge-next"

echo "install-toolchain: ok -> $toolchain/bin/proof-forge-next" >&2

# The CLI loads its package-owned frontend olean from the executable-sibling
# layout (<bin>/../lib/lean/ProofForgeV2/...), but the engineering tarball
# ships bin/ only. Populate lib/lean from a proof_forge checkout's lake build
# (self-contained: the package has no external Lean dependencies).
oleans="${PF_OLEAN_ROOT:-${PROOF_FORGE_ROOT:-/home/davirain/proof_forge}/.lake/build/lib/lean}"
if [[ -d "$oleans/ProofForgeV2" ]]; then
  mkdir -p "$toolchain/lib"
  rm -rf "$toolchain/lib/lean"
  cp -a "$oleans" "$toolchain/lib/lean"
  [[ -f "$toolchain/lib/lean/ProofForgeV2/Language/ProgramElaborationV1.olean" ]] \
    || die "olean copy incomplete: ProgramElaborationV1.olean missing under $toolchain/lib/lean"
  echo "install-toolchain: ok -> $toolchain/lib/lean (package oleans from $oleans)" >&2
else
  echo "install-toolchain: WARN: no package oleans at $oleans —" >&2
  echo "  'check'/'build' need <bin>/../lib/lean; rerun with PF_OLEAN_ROOT=<proof_forge>/.lake/build/lib/lean" >&2
fi

# Locked chain tools (solc/sbpf/wat2wasm from the checkout's build dir;
# leo/nargo land in the CLI's default cache root) the build targets shell out
# to. Both sources merge into the vendored tool-root.
platform="$(uname -s | tr '[:upper:]' '[:lower:]')-$(uname -m)"
pf_root="${PROOF_FORGE_ROOT:-/home/davirain/proof_forge}"
synced_tools=0
for tools in "${PF_TOOL_ROOT_SRC:-$pf_root/build/tool-root-$platform}" \
  "$HOME/.cache/proof-forge-v2/tool-root/$platform"; do
  if [[ -d "$tools" ]]; then
    mkdir -p "$toolchain/tool-root/$platform"
    cp -a "$tools/." "$toolchain/tool-root/$platform/"
    echo "install-toolchain: ok -> $toolchain/tool-root/$platform (locked tools from $tools)" >&2
    synced_tools=1
  fi
done
if [[ "$synced_tools" -eq 0 ]]; then
  echo "install-toolchain: WARN: no locked tools found —" >&2
  echo "  evm builds need solc; rerun with PF_TOOL_ROOT_SRC=<dir-containing-solc>" >&2
fi

# The CLI's tool-lock rejects any group/world-writable component on a tool's
# absolute path chain, so the directories we create must be 755.
chmod 755 "$toolchain" "$toolchain/lib" "$toolchain/lib/lean" 2>/dev/null || true
[[ -d "$toolchain/tool-root/$platform" ]] && chmod 755 "$toolchain/tool-root" "$toolchain/tool-root/$platform" || true
bad="$(namei -m "$toolchain/bin/proof-forge-next" | awk 'NR>2 && $1 ~ /w/ {print $2}')"
if [[ -n "$bad" ]]; then
  echo "install-toolchain: WARN: group/world-writable path components above the toolchain:" >&2
  echo "$bad" | sed 's/^/  /' >&2
  echo "  the CLI fails closed on these; fix with: chmod 755 <each listed dir>" >&2
fi
