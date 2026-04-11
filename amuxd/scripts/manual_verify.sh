#!/usr/bin/env bash
set -euo pipefail

ADDR="${AMUXD_ADDR:-127.0.0.1:8080}"
BASE_URL="http://${ADDR}"
WS_URL="ws://${ADDR}/ws/events"
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DATA_DIR="${AMUXD_DATA_DIR:-/tmp/amuxd-manual-verify}"
LOG_FILE="${DATA_DIR}/amuxd.log"
WS_LOG="${DATA_DIR}/ws-events.log"
NON_GIT_DIR="${DATA_DIR}/workspace-none"
GIT_DIR="${DATA_DIR}/workspace-git"
REMOTE_DIR="${DATA_DIR}/origin.git"
WS_PID=""
DAEMON_PID=""

need_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    printf 'error: missing required command %s\n' "$1" >&2
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

  printf 'error: daemon failed to become healthy\n' >&2
  printf '--- daemon log ---\n' >&2
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
    printf 'assertion failed: %s\n' "${msg}" >&2
    printf 'json: %s\n' "${json}" >&2
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

  printf 'warning: neither websocat nor wscat found; skipping websocket verification\n' >&2
  return 1
}

run_git() {
  git -C "$1" "${@:2}" >/dev/null
}

run_git_commit() {
  GIT_AUTHOR_NAME='AMUX Manual Verify' \
    GIT_AUTHOR_EMAIL='manual@example.com' \
    GIT_COMMITTER_NAME='AMUX Manual Verify' \
    GIT_COMMITTER_EMAIL='manual@example.com' \
    git -C "$1" "${@:2}" >/dev/null
}

json_post() {
  curl -fsS -X POST "$1" -H 'content-type: application/json' -d "$2"
}

echo '==> checking required tools'
need_cmd cargo
need_cmd curl
need_cmd jq
need_cmd tmux
need_cmd git

echo '==> building daemon'
cargo build --manifest-path "${ROOT_DIR}/Cargo.toml" >/dev/null

trap cleanup EXIT

echo '==> preparing workspaces'
rm -rf "${DATA_DIR}"
mkdir -p "${NON_GIT_DIR}" "${GIT_DIR}"
printf 'notes\n' >"${NON_GIT_DIR}/README.txt"
run_git "${GIT_DIR}" init -b main
printf 'hello\n' >"${GIT_DIR}/README.md"
run_git "${GIT_DIR}" add README.md
run_git_commit "${GIT_DIR}" commit -m init
run_git "${GIT_DIR}" branch local-base
git init --bare "${REMOTE_DIR}" >/dev/null
run_git "${GIT_DIR}" remote add origin "${REMOTE_DIR}"
run_git "${GIT_DIR}" push -u origin main

echo '==> starting daemon'
start_daemon

echo '==> 1) health'
HEALTH_JSON="$(curl -fsS "${BASE_URL}/health")"
assert_jq "${HEALTH_JSON}" '.status == "ok" and .ready == true' 'health should be ready'
assert_jq "${HEALTH_JSON}" '.now | test("Z$")' 'health.now should be UTC RFC3339'

echo '==> 2) register non-git workspace and create local session'
NON_GIT_WS_JSON="$(json_post "${BASE_URL}/workspaces" "$(jq -nc --arg root "${NON_GIT_DIR}" '{name:"plain",root_path:$root}')")"
NON_GIT_WS_ID="$(jq -r '.id' <<<"${NON_GIT_WS_JSON}")"
assert_jq "${NON_GIT_WS_JSON}" '.kind == "none"' 'non-git workspace should be classified as none'

NON_GIT_LOCAL_JSON="$(json_post "${BASE_URL}/sessions" "$(jq -nc --arg workspace_id "${NON_GIT_WS_ID}" '{name:"plain-local",workspace_id:$workspace_id,kind:"local"}')")"
NON_GIT_LOCAL_ID="$(jq -r '.id' <<<"${NON_GIT_LOCAL_JSON}")"
assert_jq "${NON_GIT_LOCAL_JSON}" '.kind == "local" and .workspace.kind == "none"' 'non-git local session shape'

NON_GIT_REFS_JSON="$(curl -fsS "${BASE_URL}/workspaces/${NON_GIT_WS_ID}/source-refs")"
assert_jq "${NON_GIT_REFS_JSON}" 'length == 0' 'non-git workspace should expose no source refs'

NON_GIT_WORKTREE_CODE="$(curl -s -o "${DATA_DIR}/non-git-worktree-error.json" -w '%{http_code}' -X POST "${BASE_URL}/workspaces/${NON_GIT_WS_ID}/worktrees" -H 'content-type: application/json' -d '{"source_ref":"main","branch_name":"invalid"}')"
[[ "${NON_GIT_WORKTREE_CODE}" == '400' ]] || { printf 'expected non-git worktree create to fail with 400\n' >&2; exit 1; }

echo '==> 3) register git workspace and inspect source refs'
GIT_WS_JSON="$(json_post "${BASE_URL}/workspaces" "$(jq -nc --arg root "${GIT_DIR}" '{name:"repo",root_path:$root}')")"
GIT_WS_ID="$(jq -r '.id' <<<"${GIT_WS_JSON}")"
assert_jq "${GIT_WS_JSON}" '.kind == "git"' 'git workspace should be classified as git'

SOURCE_REFS_JSON="$(curl -fsS "${BASE_URL}/workspaces/${GIT_WS_ID}/source-refs")"
assert_jq "${SOURCE_REFS_JSON}" 'map(select(.name == "main" and .kind == "local_branch")) | length == 1' 'local branch should be listed'
assert_jq "${SOURCE_REFS_JSON}" 'map(select(.name == "local-base" and .kind == "local_branch")) | length == 1' 'extra local branch should be listed'
assert_jq "${SOURCE_REFS_JSON}" 'map(select(.name == "origin/main" and .kind == "remote_tracking_branch")) | length == 1' 'remote tracking branch should be listed'

echo '==> 4) create local git session and managed worktrees from local and remote refs'
GIT_LOCAL_JSON="$(json_post "${BASE_URL}/sessions" "$(jq -nc --arg workspace_id "${GIT_WS_ID}" '{name:"git-local",workspace_id:$workspace_id,kind:"local"}')")"
GIT_LOCAL_ID="$(jq -r '.id' <<<"${GIT_LOCAL_JSON}")"
assert_jq "${GIT_LOCAL_JSON}" '.workspace.kind == "git" and .kind == "local"' 'git local session shape'

LOCAL_WORKTREE_JSON="$(json_post "${BASE_URL}/workspaces/${GIT_WS_ID}/worktrees" '{"source_ref":"main","branch_name":"feature-local"}')"
LOCAL_WORKTREE_ID="$(jq -r '.id' <<<"${LOCAL_WORKTREE_JSON}")"
assert_jq "${LOCAL_WORKTREE_JSON}" '.branch_name == "feature-local" and .source_ref == "main"' 'local-source managed worktree shape'
[[ -d "$(jq -r '.path' <<<"${LOCAL_WORKTREE_JSON}")" ]] || { printf 'expected local managed worktree path to exist\n' >&2; exit 1; }

REMOTE_WORKTREE_JSON="$(json_post "${BASE_URL}/workspaces/${GIT_WS_ID}/worktrees" '{"source_ref":"origin/main","branch_name":"feature-remote"}')"
REMOTE_WORKTREE_ID="$(jq -r '.id' <<<"${REMOTE_WORKTREE_JSON}")"
assert_jq "${REMOTE_WORKTREE_JSON}" '.branch_name == "feature-remote" and .source_ref == "origin/main"' 'remote-source managed worktree shape'
[[ -d "$(jq -r '.path' <<<"${REMOTE_WORKTREE_JSON}")" ]] || { printf 'expected remote managed worktree path to exist\n' >&2; exit 1; }

WORKTREE_LIST_JSON="$(curl -fsS "${BASE_URL}/workspaces/${GIT_WS_ID}/worktrees")"
assert_jq "${WORKTREE_LIST_JSON}" "map(select(.id == \"${LOCAL_WORKTREE_ID}\")) | length == 1" 'local managed worktree should be listed'
assert_jq "${WORKTREE_LIST_JSON}" "map(select(.id == \"${REMOTE_WORKTREE_ID}\")) | length == 1" 'remote managed worktree should be listed'

DUPLICATE_CODE="$(curl -s -o "${DATA_DIR}/duplicate-branch-error.json" -w '%{http_code}' -X POST "${BASE_URL}/workspaces/${GIT_WS_ID}/worktrees" -H 'content-type: application/json' -d '{"source_ref":"main","branch_name":"feature-local"}')"
[[ "${DUPLICATE_CODE}" == '409' ]] || { printf 'expected duplicate managed branch to fail with 409\n' >&2; exit 1; }

echo '==> 5) create worktree session'
WORKTREE_SESSION_JSON="$(json_post "${BASE_URL}/sessions" "$(jq -nc --arg workspace_id "${GIT_WS_ID}" --arg managed_worktree_id "${REMOTE_WORKTREE_ID}" '{name:"remote-worktree-session",workspace_id:$workspace_id,kind:"worktree",managed_worktree_id:$managed_worktree_id}')")"
WORKTREE_SESSION_ID="$(jq -r '.id' <<<"${WORKTREE_SESSION_JSON}")"
assert_jq "${WORKTREE_SESSION_JSON}" ".kind == \"worktree\" and .managed_worktree.id == \"${REMOTE_WORKTREE_ID}\"" 'worktree session should bind to managed worktree'

echo '==> 6) websocket events'
WS_ENABLED=0
if collect_ws_events; then
  WS_ENABLED=1
  sleep 0.8
  WS_CREATE_JSON="$(json_post "${BASE_URL}/sessions" "$(jq -nc --arg workspace_id "${GIT_WS_ID}" '{name:"ws-check",workspace_id:$workspace_id,kind:"local"}')")"
  WS_SESSION_ID="$(jq -r '.id' <<<"${WS_CREATE_JSON}")"
  curl -fsS -X DELETE "${BASE_URL}/sessions/${WS_SESSION_ID}" >/dev/null
fi

if [[ "${WS_ENABLED}" == '1' ]]; then
  sleep 1
  CREATED_EVENT_COUNT="$(grep -c 'session.created' "${WS_LOG}" || true)"
  TERM_EVENT_COUNT="$(grep -c 'session.terminated' "${WS_LOG}" || true)"
  [[ "${CREATED_EVENT_COUNT}" -ge 1 ]] || { printf 'expected session.created in websocket stream\n' >&2; exit 1; }
  [[ "${TERM_EVENT_COUNT}" -ge 1 ]] || { printf 'expected session.terminated in websocket stream\n' >&2; exit 1; }
fi

echo '==> 7) restart visibility'
restart_daemon

WORKSPACES_AFTER_RESTART="$(curl -fsS "${BASE_URL}/workspaces")"
assert_jq "${WORKSPACES_AFTER_RESTART}" "map(select(.id == \"${NON_GIT_WS_ID}\")) | length == 1" 'non-git workspace should persist after restart'
assert_jq "${WORKSPACES_AFTER_RESTART}" "map(select(.id == \"${GIT_WS_ID}\")) | length == 1" 'git workspace should persist after restart'

WORKTREES_AFTER_RESTART="$(curl -fsS "${BASE_URL}/workspaces/${GIT_WS_ID}/worktrees")"
assert_jq "${WORKTREES_AFTER_RESTART}" "map(select(.id == \"${LOCAL_WORKTREE_ID}\")) | length == 1" 'local managed worktree should persist after restart'
assert_jq "${WORKTREES_AFTER_RESTART}" "map(select(.id == \"${REMOTE_WORKTREE_ID}\")) | length == 1" 'remote managed worktree should persist after restart'

SESSIONS_AFTER_RESTART="$(curl -fsS "${BASE_URL}/sessions")"
assert_jq "${SESSIONS_AFTER_RESTART}" "map(select(.id == \"${NON_GIT_LOCAL_ID}\" and .kind == \"local\")) | length == 1" 'non-git local session should remain visible after restart'
assert_jq "${SESSIONS_AFTER_RESTART}" "map(select(.id == \"${GIT_LOCAL_ID}\" and .workspace.id == \"${GIT_WS_ID}\")) | length == 1" 'git local session should remain visible after restart'
assert_jq "${SESSIONS_AFTER_RESTART}" "map(select(.id == \"${WORKTREE_SESSION_ID}\" and .managed_worktree.id == \"${REMOTE_WORKTREE_ID}\")) | length == 1" 'worktree session binding should remain visible after restart'

echo
echo 'manual verification passed'
echo "- daemon log: ${LOG_FILE}"
echo "- websocket log: ${WS_LOG}"
