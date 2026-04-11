#!/usr/bin/env bash
set -euo pipefail

ADDR="${AMUXD_ADDR:-127.0.0.1:8080}"
BASE_URL="http://${ADDR}"
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DATA_DIR="${AMUXD_DATA_DIR:-/tmp/amuxd-terminal-manual-verify}"
LOG_FILE="${DATA_DIR}/amuxd-terminal.log"
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
  local terminal_flag="${1:-0}"

  mkdir -p "${DATA_DIR}"
  : >"${LOG_FILE}"

  if [[ -n "${DAEMON_PID}" ]] && kill -0 "${DAEMON_PID}" >/dev/null 2>&1; then
    kill "${DAEMON_PID}" >/dev/null 2>&1 || true
    wait "${DAEMON_PID}" 2>/dev/null || true
  fi

  AMUXD_ADDR="${ADDR}" \
  AMUXD_DATA_DIR="${DATA_DIR}" \
  AMUXD_TERMINAL_RENDERER_V1="${terminal_flag}" \
  "${ROOT_DIR}/target/debug/amuxd" >"${LOG_FILE}" 2>&1 &
  DAEMON_PID="$!"

  for _ in {1..50}; do
    if curl -fsS "${BASE_URL}/health" >/dev/null 2>&1; then
      return 0
    fi
    sleep 0.2
  done

  echo "error: daemon failed to become healthy" >&2
  echo "--- daemon log ---" >&2
  cat "${LOG_FILE}" >&2
  exit 1
}

assert_jq() {
  local json="$1"
  local expr="$2"
  local msg="$3"
  if ! jq -e "${expr}" <<<"${json}" >/dev/null; then
    echo "assertion failed: ${msg}" >&2
    echo "json: ${json}" >&2
    exit 1
  fi
}

echo "==> checking required tools"
need_cmd cargo
need_cmd curl
need_cmd jq
need_cmd tmux

echo "==> building daemon"
cargo build --manifest-path "${ROOT_DIR}/Cargo.toml" >/dev/null

trap cleanup EXIT

echo "==> 1) terminal routes are hidden without feature flag"
start_daemon 0
CREATE_JSON="$(curl -fsS -X POST "${BASE_URL}/sessions" -H 'content-type: application/json' -d '{"name":"term-verify-off"}')"
SESSION_ID="$(jq -r '.id' <<<"${CREATE_JSON}")"
OFF_CODE="$(curl -s -o /tmp/amuxd-terminal-off.json -w '%{http_code}' "${BASE_URL}/sessions/${SESSION_ID}/terminal")"
[[ "${OFF_CODE}" == "404" ]] || {
  echo "expected terminal route to be disabled without feature flag, got ${OFF_CODE}" >&2
  exit 1
}
curl -s -X DELETE "${BASE_URL}/sessions/${SESSION_ID}" >/dev/null
SESSION_ID=""

echo "==> 2) terminal routes are available with feature flag"
start_daemon 1
CREATE_JSON="$(curl -fsS -X POST "${BASE_URL}/sessions" -H 'content-type: application/json' -d '{"name":"term-verify-on"}')"
SESSION_ID="$(jq -r '.id' <<<"${CREATE_JSON}")"

SURFACE_JSON="$(curl -fsS "${BASE_URL}/sessions/${SESSION_ID}/terminal")"
assert_jq "${SURFACE_JSON}" '.session_id == "'"${SESSION_ID}"'"' "surface should include created session id"
assert_jq "${SURFACE_JSON}" '.stack.escape_parser == "vte"' "surface should advertise vte parser"
assert_jq "${SURFACE_JSON}" '.stack.state_core == "vt100"' "surface should advertise vt100 state core"
assert_jq "${SURFACE_JSON}" '.stack.width_engine == "unicode-width"' "surface should advertise unicode-width"
assert_jq "${SURFACE_JSON}" '.stack.grapheme_engine == "unicode-segmentation"' "surface should advertise unicode-segmentation"
assert_jq "${SURFACE_JSON}" '.fallback_policy.alternate_state_core == "alacritty_terminal"' "fallback state core should be exposed"
assert_jq "${SURFACE_JSON}" '.fallback_policy.consecutive_milestones_required == 2' "fallback threshold should be exposed"
assert_jq "${SURFACE_JSON}" '.input_capabilities.text == true and .input_capabilities.paste == true and .input_capabilities.resize == true' "baseline input capabilities should be enabled"
assert_jq "${SURFACE_JSON}" '.input_capabilities.named_keys == ["ctrl","escape","tab","arrow_up","arrow_down","arrow_left","arrow_right","enter"]' "named key set should match spec"
assert_jq "${SURFACE_JSON}" 'has("runtime_name") | not' "surface should not expose tmux runtime name"
assert_jq "${SURFACE_JSON}" 'has("pane_id") | not' "surface should not expose tmux pane ids"
assert_jq "${SURFACE_JSON}" '.snapshot.rows > 0 and .snapshot.cols > 0' "snapshot dimensions should be present"
assert_jq "${SURFACE_JSON}" '.snapshot.cursor.row >= 0 and .snapshot.cursor.col >= 0' "snapshot cursor should be present"

echo "==> 3) send text and key input through terminal contract"
MARKER="AMUX_TERM_$(date +%s)"
INPUT_JSON="$(curl -fsS -X POST "${BASE_URL}/sessions/${SESSION_ID}/terminal/input" \
  -H 'content-type: application/json' \
  -d '{
    "events": [
      {"type":"text","text":"printf \"'"${MARKER}"'\\n\""},
      {"type":"key","key":{"kind":"named","key":"enter"},"ctrl":false,"alt":false,"shift":false}
    ]
  }')"
assert_jq "${INPUT_JSON}" '.accepted_events == 2' "input endpoint should acknowledge both events"

FOUND_MARKER=0
for _ in {1..25}; do
  SURFACE_JSON="$(curl -fsS "${BASE_URL}/sessions/${SESSION_ID}/terminal")"
  if jq -e '.snapshot.plain_text | contains("'"${MARKER}"'")' <<<"${SURFACE_JSON}" >/dev/null; then
    FOUND_MARKER=1
    break
  fi
  sleep 0.2
done

[[ "${FOUND_MARKER}" == "1" ]] || {
  echo "expected terminal snapshot to contain marker '${MARKER}'" >&2
  echo "last surface json: ${SURFACE_JSON}" >&2
  exit 1
}

echo "==> 4) paste path is accepted"
PASTE_JSON="$(curl -fsS -X POST "${BASE_URL}/sessions/${SESSION_ID}/terminal/input" \
  -H 'content-type: application/json' \
  -d '{
    "events": [
      {"type":"paste","text":"echo pasted-path-check"}
    ]
  }')"
assert_jq "${PASTE_JSON}" '.accepted_events == 1' "paste event should be accepted"

echo
echo "terminal manual verification passed"
echo "- session id: ${SESSION_ID}"
echo "- daemon log: ${LOG_FILE}"
