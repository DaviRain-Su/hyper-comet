#!/usr/bin/env bash
# Linux packaging: build the release binary and produce
#   target/package/comet-<version>-linux-<arch>.tar.gz
# containing the binary, the .desktop entry, and the icon, plus an install.sh
# that drops them into ~/.local (XDG) paths.
#
# Usage: scripts/package-linux.sh
# Env:   PROFILE=debug for a fast unoptimized package (CI smoke); default release.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
command -v cargo >/dev/null 2>&1 || PATH="$HOME/.cargo/bin:$PATH"
PROFILE="${PROFILE:-release}"
ARCH="$(uname -m)"
VERSION="$(grep -m1 '^version' "$ROOT/Cargo.toml" | sed 's/.*"\(.*\)".*/\1/')"
OUT_DIR="$ROOT/target/package"
STAGE="$OUT_DIR/comet-$VERSION-linux-$ARCH"
TARBALL="$STAGE.tar.gz"

cd "$ROOT"
if [[ "$PROFILE" == "release" ]]; then
  cargo build --release -p comet -p proofship-pf-mcp
  BIN="$ROOT/target/release/comet"
  PF_MCP_BIN="$ROOT/target/release/proofship-pf-mcp"
else
  cargo build -p comet -p proofship-pf-mcp
  BIN="$ROOT/target/debug/comet"
  PF_MCP_BIN="$ROOT/target/debug/proofship-pf-mcp"
fi

rm -rf "$STAGE" "$TARBALL"
mkdir -p "$STAGE"
install -m 755 "$BIN" "$STAGE/comet"
# ProofForge MCP gate: the engine resolves it as a sibling of the comet binary.
install -m 755 "$PF_MCP_BIN" "$STAGE/proofship-pf-mcp"
install -m 644 "$ROOT/dist/comet.desktop" "$STAGE/comet.desktop"
install -m 644 "$ROOT/dist/comet.png" "$STAGE/comet.png"

cat >"$STAGE/install.sh" <<'INSTALL'
#!/usr/bin/env bash
# Install Comet into ~/.local (no root needed).
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
install -Dm755 "$HERE/comet" "$HOME/.local/bin/comet"
install -Dm755 "$HERE/proofship-pf-mcp" "$HOME/.local/bin/proofship-pf-mcp"
install -Dm644 "$HERE/comet.desktop" "$HOME/.local/share/applications/comet.desktop"
install -Dm644 "$HERE/comet.png" "$HOME/.local/share/icons/hicolor/1024x1024/apps/comet.png"
command -v update-desktop-database >/dev/null 2>&1 \
  && update-desktop-database "$HOME/.local/share/applications" || true
echo "Installed. Make sure ~/.local/bin is on your PATH."
INSTALL
chmod 755 "$STAGE/install.sh"

tar -czf "$TARBALL" -C "$OUT_DIR" "$(basename "$STAGE")"
rm -rf "$STAGE"
echo "packaged: $TARBALL"
tar -tzf "$TARBALL"
