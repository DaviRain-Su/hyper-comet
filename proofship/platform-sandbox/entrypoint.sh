#!/bin/sh
# PlatformExecutor Sandbox entrypoint.
# Wire PROOFSHIP_RELAY + device token; refuse deploy; run gate when CLI present.
set -eu

cd "$(dirname "$0")"

if [ -f dist/index.js ]; then
  exec node dist/index.js
fi

if command -v npx >/dev/null 2>&1; then
  exec npx --yes tsx src/index.ts
fi

echo "platform-sandbox: build first (npm run build) or install tsx" >&2
exit 1
