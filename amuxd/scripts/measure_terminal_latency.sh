#!/usr/bin/env bash
set -euo pipefail

MODE="${1:-local}"
ADDR="${AMUXD_ADDR:-127.0.0.1:8080}"
BASE_URL="http://${ADDR}"
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DATA_DIR="${AMUXD_DATA_DIR:-/tmp/amuxd-terminal-latency}"
LOG_FILE="${DATA_DIR}/amuxd-terminal-latency.log"
ITERATIONS="${ITERATIONS:-25}"
DAEMON_PID=""
SESSION_ID=""

case "${MODE}" in
  local|lan)
    BUDGET_MS=160
    ;;
  remote)
    BUDGET_MS=280
    ;;
  *)
    echo "usage: $0 [local|lan|remote]" >&2
    exit 1
    ;;
esac

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

post_marker_command() {
  local marker="$1"
  local payload
  payload="$(jq -nc --arg text "${marker}" '{events:[
    {type:"text", text:$text}
  ]}')"
  curl -fsS -X POST \
    "${BASE_URL}/sessions/${SESSION_ID}/terminal/input" \
    -H 'content-type: application/json' \
    -d "${payload}" >/dev/null
}

clear_prompt_line() {
  local payload='{"events":[{"type":"key","key":{"kind":"character","text":"u"},"ctrl":true,"alt":false,"shift":false}]}'
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
need_cmd awk
need_cmd date

echo "==> building daemon"
cargo build --manifest-path "${ROOT_DIR}/Cargo.toml" >/dev/null

trap cleanup EXIT

echo "==> starting daemon"
start_daemon

CREATE_JSON="$(curl -fsS -X POST "${BASE_URL}/sessions" -H 'content-type: application/json' -d '{"name":"latency-check"}')"
SESSION_ID="$(jq -r '.id' <<<"${CREATE_JSON}")"

LATENCIES_FILE="${DATA_DIR}/latencies.txt"
: >"${LATENCIES_FILE}"

for _ in {1..3}; do
  post_marker_command "@@"
  sleep 0.05
  clear_prompt_line
done

echo "==> measuring ${ITERATIONS} keypress-to-echo iterations (${MODE}, budget ${BUDGET_MS} ms)"
for i in $(seq 1 "${ITERATIONS}"); do
  MARKER="@${i}@"
  START_MS="$(date +%s%3N)"
  post_marker_command "${MARKER}"

  for _ in {1..200}; do
    SURFACE_JSON="$(curl -fsS "${BASE_URL}/sessions/${SESSION_ID}/terminal")"
    if jq -e --arg marker "${MARKER}" '.snapshot.plain_text | contains($marker)' <<<"${SURFACE_JSON}" >/dev/null; then
      END_MS="$(date +%s%3N)"
      echo "$((END_MS - START_MS))" >>"${LATENCIES_FILE}"
      clear_prompt_line
      break
    fi
    sleep 0.01
  done
  sleep 0.02
done

P95_MS="$(sort -n "${LATENCIES_FILE}" | awk -v n="${ITERATIONS}" 'NR == int((n * 0.95) + 0.5) { print; exit }')"
MEAN_MS="$(awk '{sum += $1} END { if (NR == 0) print 0; else printf "%.2f", sum / NR }' "${LATENCIES_FILE}")"
MAX_MS="$(sort -n "${LATENCIES_FILE}" | tail -n 1)"

echo "p95_ms=${P95_MS}"
echo "mean_ms=${MEAN_MS}"
echo "max_ms=${MAX_MS}"

if (( P95_MS > BUDGET_MS )); then
  echo "latency budget failed: p95 ${P95_MS} ms exceeded ${BUDGET_MS} ms" >&2
  exit 1
fi

echo "latency budget passed"
