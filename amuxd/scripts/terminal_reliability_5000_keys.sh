#!/usr/bin/env bash
set -euo pipefail

ADDR="${AMUXD_ADDR:-127.0.0.1:8080}"
BASE_URL="http://${ADDR}"
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DATA_DIR="${AMUXD_DATA_DIR:-/tmp/amuxd-terminal-reliability}"
LOG_FILE="${DATA_DIR}/amuxd-terminal-reliability.log"
KEYLOG_FILE="${DATA_DIR}/keylog.txt"
EXPECTED_KEYS=5000
MIN_DELIVERED=4995
LINES=100
TEXT_KEYS_PER_LINE=49
DAEMON_PID=""
SESSION_ID=""

need_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "error: missing required command '$1'" >&2
    exit 1
  fi
}

cleanup() {
  if [[ -n "${SESSION_ID}" ]]; then
    curl -s -X DELETE "${BASE_URL}/sessions/${SESSION_ID}" >/dev/null 2>&1 || true
  fi
  if [[ -n "${DAEMON_PID}" ]] && kill -0 "${DAEMON_PID}" >/dev/null 2>&1; then
    kill "${DAEMON_PID}" >/dev/null 2>&1 || true
    wait "${DAEMON_PID}" 2>/dev/null || true
  fi
}

start_daemon() {
  mkdir -p "${DATA_DIR}"
  : >"${LOG_FILE}"

  AMUXD_ADDR="${ADDR}" \
  AMUXD_DATA_DIR="${DATA_DIR}" \
  AMUXD_TERMINAL_RENDERER_V1=1 \
  "${ROOT_DIR}/target/debug/amuxd" >"${LOG_FILE}" 2>&1 &
  DAEMON_PID="$!"

  for _ in {1..50}; do
    if curl -fsS "${BASE_URL}/health" >/dev/null 2>&1; then
      return 0
    fi
    sleep 0.2
  done

  echo "error: daemon failed to become healthy" >&2
  cat "${LOG_FILE}" >&2
  exit 1
}

post_input() {
  local payload="$1"
  curl -fsS -X POST \
    "${BASE_URL}/sessions/${SESSION_ID}/terminal/input" \
    -H 'content-type: application/json' \
    -d "${payload}" >/dev/null
}

echo "==> checking required tools"
need_cmd cargo
need_cmd curl
need_cmd jq
need_cmd tmux

echo "==> building daemon"
cargo build --manifest-path "${ROOT_DIR}/Cargo.toml" >/dev/null

trap cleanup EXIT

echo "==> starting daemon"
start_daemon

CREATE_JSON="$(curl -fsS -X POST "${BASE_URL}/sessions" -H 'content-type: application/json' -d '{"name":"reliability-check"}')"
SESSION_ID="$(jq -r '.id' <<<"${CREATE_JSON}")"

echo "==> opening capture file in terminal session"
SETUP_PAYLOAD="$(jq -nc --arg cmd "cat > ${KEYLOG_FILE}" '{events:[
  {type:"text", text:$cmd},
  {type:"key", key:{kind:"named", key:"enter"}, ctrl:false, alt:false, shift:false}
]}')"
post_input "${SETUP_PAYLOAD}"
sleep 0.2

echo "==> sending ${EXPECTED_KEYS} scripted key events as ${LINES} short lines"
for _ in $(seq 1 "${LINES}"); do
  KEY_PAYLOAD="$(jq -nc --argjson count "${TEXT_KEYS_PER_LINE}" '{events:(
    [range(0; $count) | {type:"text", text:"x"}] +
    [{type:"key", key:{kind:"named", key:"enter"}, ctrl:false, alt:false, shift:false}]
  )}')"
  post_input "${KEY_PAYLOAD}"
  sleep 0.02
done

EOF_PAYLOAD='{"events":[{"type":"key","key":{"kind":"character","text":"d"},"ctrl":true,"alt":false,"shift":false}]}'
post_input "${EOF_PAYLOAD}"
sleep 0.2

RESULT_PAYLOAD="$(jq -nc --arg cmd "wc -c < ${KEYLOG_FILE}; printf \"__AMUX_RELIABILITY_DONE__\\n\"" '{events:[
  {type:"text", text:$cmd},
  {type:"key", key:{kind:"named", key:"enter"}, ctrl:false, alt:false, shift:false}
]}')"
post_input "${RESULT_PAYLOAD}"

echo "==> waiting for terminal output marker"
for _ in {1..100}; do
  SURFACE_JSON="$(curl -fsS "${BASE_URL}/sessions/${SESSION_ID}/terminal")"
  if jq -e '.snapshot.plain_text | contains("__AMUX_RELIABILITY_DONE__")' <<<"${SURFACE_JSON}" >/dev/null; then
    break
  fi
  sleep 0.1
done

DELIVERED="$(wc -c < "${KEYLOG_FILE}" | tr -d '[:space:]')"
PERCENT="$(awk -v delivered="${DELIVERED}" -v expected="${EXPECTED_KEYS}" 'BEGIN { printf "%.3f", (delivered / expected) * 100 }')"

echo "delivered=${DELIVERED}/${EXPECTED_KEYS} (${PERCENT}%)"
echo "pass threshold: ${MIN_DELIVERED}/${EXPECTED_KEYS} (99.9%)"

if (( DELIVERED < MIN_DELIVERED )); then
  echo "reliability check failed" >&2
  exit 1
fi

echo "terminal reliability check passed"
