#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/../.." && pwd)"

cat <<'EOF'
AMUX web shell manual verification

Checklist: docs/web-shell-manual-verification.md
Server:      http://127.0.0.1:8080/app
Terminal:    enabled for this run
EOF

AMUXD_TERMINAL_RENDERER_V1=1 cargo run --manifest-path "$ROOT_DIR/amuxd/Cargo.toml"
