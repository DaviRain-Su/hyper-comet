#!/usr/bin/env bash
# ProofShip — one-command X Layer TESTNET deploy for any gate-passing ProgramV1
# contract (vertical-agnostic).
#
#   1. re-runs the full product gate (check → build → inspect exact closure)
#   2. abi-encodes the constructor when a signature is given
#   3. cast create to X Layer testnet (chainId 1952, OKB gas)
#   4. prints the explorer link
#
# Discipline:
#   - opt-in: PF_XLAYER_CONFIRM=yes required
#   - key lives ONLY in an env var you name via PF_XLAYER_PRIVATE_KEY_ENV;
#     never in files, never on this script's argv, never on any MCP surface
#
# Usage:
#   PF_XLAYER_CONFIRM=yes PF_XLAYER_PRIVATE_KEY_ENV=PF_XLAYER_KEY \
#     proofship/scripts/deploy-xlayer-testnet.sh <source.lean> <ModuleName> <ctor-sig|-> [ctor-args...]
#
# Example (share registry ctor):
#   ... deploy-xlayer-testnet.sh contract.lean RwaShareRegistry \
#       'constructor(uint64,uint64,uint64)' 1000000 50000 100000
# No-constructor contract: pass '-' as the signature.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd "$here/../.." && pwd)"
proj="${PROOFSHIP_PROJECT_ROOT:-$root/proofship/inbox}"

die() { echo "deploy-xlayer: REFUSED/FAIL: $*" >&2; exit 2; }

[[ "${PF_XLAYER_CONFIRM:-}" == "yes" ]] \
  || die "set PF_XLAYER_CONFIRM=yes (testnet deploy is opt-in)"
KEY_ENV_NAME="${PF_XLAYER_PRIVATE_KEY_ENV:-}"
[[ -n "$KEY_ENV_NAME" ]] || die "set PF_XLAYER_PRIVATE_KEY_ENV to the env var NAME holding the key"
[[ -n "${!KEY_ENV_NAME:-}" ]] || die "env '$KEY_ENV_NAME' is empty"

src="${1:?usage: deploy-xlayer-testnet.sh <source.lean> <ModuleName> <ctor-sig|-> [ctor-args...]}"
module="${2:?missing ModuleName}"
ctor_sig="${3:?missing ctor signature ('-' when the contract has no constructor)}"
if [[ $# -ge 3 ]]; then shift 3; else shift $#; fi

cast_bin="$(command -v cast 2>/dev/null || true)"
[[ -n "$cast_bin" && -x "$cast_bin" ]] || cast_bin="$HOME/.foundry/bin/cast"
[[ -x "$cast_bin" ]] || die "cast not found (PATH or ~/.foundry/bin)"

CHAIN_ID=1952
RPC="${PF_XLAYER_RPC:-https://testrpc.xlayer.tech/terigon}"

echo "== 1/3 gate (check + build + inspect) ==" >&2
"$here/gate.sh" "$src" "$module"
out="$proj/studio-inbox/out-$(echo "$module" | tr '[:upper:]' '[:lower:]')"
bin_file="$out/$module.bin"
[[ -s "$bin_file" ]] || die "bin missing under $out"

echo "== 2/3 deploy to X Layer testnet (chainId $CHAIN_ID) ==" >&2
encoded=""
if [[ "$ctor_sig" != "-" ]]; then
  encoded="$("$cast_bin" abi-encode "$ctor_sig" "$@")"
fi
bytecode="$(tr -d '\n\r ' < "$bin_file")${encoded#0x}"
json="$("$cast_bin" send --json --rpc-url "$RPC" --private-key "${!KEY_ENV_NAME}" \
  --create "0x${bytecode}")"
addr="$(/usr/bin/python3 -I -S -c 'import json,sys; print(json.load(sys.stdin).get("contractAddress",""))' <<<"$json")"
[[ -n "$addr" && "$addr" != "null" ]] || die "deploy returned no contractAddress: $json"

echo "== 3/3 done ==" >&2
echo "contract=$addr"
echo "network=xlayer-testnet chainId=$CHAIN_ID rpc=$RPC"
echo "explorer=https://www.okx.com/web3/explorer/xlayer-test (paste address: $addr)"
if [[ "$ctor_sig" != "-" ]]; then
  echo "ctor: $ctor_sig args=($*)"
fi
