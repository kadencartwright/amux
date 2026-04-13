#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/../.." && pwd)"
SHELL_DIR="$ROOT_DIR/amuxshell-web"
DIST_DIR="$SHELL_DIR/dist"
VENDOR_DIR="$DIST_DIR/assets/vendor"

mkdir -p "$DIST_DIR/assets" "$VENDOR_DIR"
rm -f "$VENDOR_DIR"/*

if [ ! -d "$SHELL_DIR/node_modules/ghostty-web" ]; then
  npm install --prefix "$SHELL_DIR"
fi

cp "$SHELL_DIR/src/index.html" "$DIST_DIR/index.html"
cp "$SHELL_DIR/src/app.js" "$DIST_DIR/assets/app.js"
cp "$SHELL_DIR/src/core.js" "$DIST_DIR/assets/core.js"
cp "$SHELL_DIR/src/app.css" "$DIST_DIR/assets/app.css"
cp "$SHELL_DIR/node_modules/ghostty-web/dist/ghostty-web.js" "$VENDOR_DIR/ghostty-web.js"
cp "$SHELL_DIR/node_modules/ghostty-web/ghostty-vt.wasm" "$VENDOR_DIR/ghostty-vt.wasm"
