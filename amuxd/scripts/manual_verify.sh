#!/usr/bin/env bash
set -euo pipefail

ADDR="${AMUXD_ADDR:-127.0.0.1:8080}"
BASE_URL="http://${ADDR}"
WS_URL="ws://${ADDR}/ws/events"
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DATA_DIR="${AMUXD_DATA_DIR:-/tmp/amuxd-manual-verify}"
LOG_FILE="${DATA_DIR}/amuxd.log"
WS_LOG="${DATA_DIR}/ws-events.log"
DAEMON_PID=""
WS_PID=""

need_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "error: missing required command '$1'" >&2
    exit 1
  fi
}

cleanup() {
  if [[ -n "${WS_PID}" ]] && kill -0 "${WS_PID}" >/dev/null 2>&1; then
    kill "${WS_PID}" >/dev/null 2>&1 || true
  fi
  if [[ -n "${DAEMON_PID}" ]] && kill -0 "${DAEMON_PID}" >/dev/null 2>&1; then
    kill "${DAEMON_PID}" >/dev/null 2>&1 || true
  fi
}

start_daemon() {
  mkdir -p "${DATA_DIR}"
  : >"${LOG_FILE}"
  AMUXD_ADDR="${ADDR}" AMUXD_DATA_DIR="${DATA_DIR}" "${ROOT_DIR}/target/debug/amuxd" >"${LOG_FILE}" 2>&1 &
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

restart_daemon() {
  if [[ -n "${DAEMON_PID}" ]] && kill -0 "${DAEMON_PID}" >/dev/null 2>&1; then
    kill "${DAEMON_PID}" >/dev/null 2>&1 || true
    wait "${DAEMON_PID}" 2>/dev/null || true
  fi
  DAEMON_PID=""
  start_daemon
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

collect_ws_events() {
  : >"${WS_LOG}"
  if command -v websocat >/dev/null 2>&1; then
    timeout 12s websocat "${WS_URL}" >"${WS_LOG}" 2>/dev/null &
    WS_PID="$!"
    return 0
  fi

  if command -v wscat >/dev/null 2>&1; then
    timeout 12s wscat -c "${WS_URL}" >"${WS_LOG}" 2>/dev/null &
    WS_PID="$!"
    return 0
  fi

  echo "warning: neither websocat nor wscat found; skipping websocket verification" >&2
  return 1
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

echo "==> 1) health"
HEALTH_JSON="$(curl -fsS "${BASE_URL}/health")"
assert_jq "${HEALTH_JSON}" '.status == "ok" and .ready == true' "health should be ready"
assert_jq "${HEALTH_JSON}" '.now | test("Z$")' "health.now should be UTC RFC3339"

echo "==> 2) create/list/get"
CREATE_JSON="$(curl -fsS -X POST "${BASE_URL}/sessions" -H 'content-type: application/json' -d '{"name":"manual-check"}')"
SESSION_ID="$(jq -r '.id' <<<"${CREATE_JSON}")"
assert_jq "${CREATE_JSON}" '.id != null and .name == "manual-check" and .state == "running"' "create response shape"
assert_jq "${CREATE_JSON}" '.created_at | test("Z$")' "created_at should be UTC RFC3339"
assert_jq "${CREATE_JSON}" '.last_activity_at | test("Z$")' "last_activity_at should be UTC RFC3339"

LIST_JSON="$(curl -fsS "${BASE_URL}/sessions")"
assert_jq "${LIST_JSON}" "map(select(.id == \"${SESSION_ID}\")) | length == 1" "list should include created session"

GET_JSON="$(curl -fsS "${BASE_URL}/sessions/${SESSION_ID}")"
assert_jq "${GET_JSON}" ".id == \"${SESSION_ID}\"" "get should return created session"

echo "==> 3) websocket events"
WS_ENABLED=0
if collect_ws_events; then
  WS_ENABLED=1
  sleep 0.8

  WS_CREATE_JSON="$(curl -fsS -X POST "${BASE_URL}/sessions" -H 'content-type: application/json' -d '{"name":"ws-check"}')"
  WS_SESSION_ID="$(jq -r '.id' <<<"${WS_CREATE_JSON}")"
  curl -fsS -X DELETE "${BASE_URL}/sessions/${WS_SESSION_ID}" >/dev/null
fi

echo "==> 4) terminate + not-found envelope"
DELETE_CODE="$(curl -s -o /dev/null -w '%{http_code}' -X DELETE "${BASE_URL}/sessions/${SESSION_ID}")"
[[ "${DELETE_CODE}" == "204" ]] || { echo "expected DELETE 204, got ${DELETE_CODE}" >&2; exit 1; }

NOT_FOUND_CODE="$(curl -s -o /tmp/amuxd-not-found.json -w '%{http_code}' "${BASE_URL}/sessions/${SESSION_ID}")"
[[ "${NOT_FOUND_CODE}" == "404" ]] || { echo "expected GET missing 404, got ${NOT_FOUND_CODE}" >&2; exit 1; }
NOT_FOUND_JSON="$(cat /tmp/amuxd-not-found.json)"
assert_jq "${NOT_FOUND_JSON}" '.error.code == "session_not_found" and (.error.message | type == "string")' "not-found envelope"

if [[ "${WS_ENABLED}" == "1" ]]; then
  sleep 1
  CREATED_EVENT_COUNT="$(grep -c 'session.created' "${WS_LOG}" || true)"
  TERM_EVENT_COUNT="$(grep -c 'session.terminated' "${WS_LOG}" || true)"
  [[ "${CREATED_EVENT_COUNT}" -ge 1 ]] || { echo "expected session.created in websocket stream" >&2; exit 1; }
  [[ "${TERM_EVENT_COUNT}" -ge 1 ]] || { echo "expected session.terminated in websocket stream" >&2; exit 1; }
  if grep -E '"occurred_at"\s*:\s*"[^"]*Z"' "${WS_LOG}" >/dev/null 2>&1; then
    :
  else
    echo "expected UTC RFC3339 occurred_at in websocket events" >&2
    exit 1
  fi
fi

echo "==> 5) restart visibility"
CREATE2_JSON="$(curl -fsS -X POST "${BASE_URL}/sessions" -H 'content-type: application/json' -d '{"name":"restart-check"}')"
SESSION_ID2="$(jq -r '.id' <<<"${CREATE2_JSON}")"

restart_daemon

LIST2_JSON="$(curl -fsS "${BASE_URL}/sessions")"
assert_jq "${LIST2_JSON}" "map(select(.id == \"${SESSION_ID2}\")) | length == 1" "session should remain visible after restart"

echo
echo "manual verification passed"
echo "- daemon log: ${LOG_FILE}"
echo "- websocket log: ${WS_LOG}"
