#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/../.." && pwd)"
SHELL_DIR="$ROOT_DIR/amuxshell-web"
DIST_DIR="$SHELL_DIR/dist"
VENDOR_DIR="$DIST_DIR/assets/vendor"
WASM_TARGET_DIR="$ROOT_DIR/amuxterm-web/target/wasm32-unknown-unknown/release"

mkdir -p "$DIST_DIR/assets" "$VENDOR_DIR"

cargo build \
  --manifest-path "$ROOT_DIR/amuxterm-web/Cargo.toml" \
  --release \
  --target wasm32-unknown-unknown \
  --features wasm

wasm-bindgen \
  --target web \
  --out-dir "$VENDOR_DIR" \
  "$WASM_TARGET_DIR/amuxterm_web.wasm"

cp "$SHELL_DIR/src/index.html" "$DIST_DIR/index.html"
cp "$SHELL_DIR/src/app.js" "$DIST_DIR/assets/app.js"
cp "$SHELL_DIR/src/core.js" "$DIST_DIR/assets/core.js"
cp "$SHELL_DIR/src/app.css" "$DIST_DIR/assets/app.css"
