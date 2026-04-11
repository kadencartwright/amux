import test from "node:test";
import assert from "node:assert/strict";

import {
  POLL_INTERVAL_MS,
  applySessions,
  applyWorkspaces,
  buildNamedKeyRequest,
  buildTextInputRequest,
  createShellState,
  eventRequiresSessionRefresh,
  handleCreateSuccess,
  renderShell,
  selectSession,
  selectWorkspace,
  setPageVisibility,
  shouldPollTerminal,
  shouldRefetchSessionsOnSocketOpen
} from "../src/core.js";

const workspaces = [
  {
    id: "ws-alpha",
    name: "alpha",
    kind: "git",
    root_path: "/tmp/alpha"
  },
  {
    id: "ws-beta",
    name: "beta",
    kind: "none",
    root_path: "/tmp/beta"
  }
];

const sessions = [
  {
    id: "alpha-local",
    name: "alpha-local",
    kind: "local",
    created_at: "2026-04-11T10:00:00Z",
    workspace: workspaces[0],
    managed_worktree: null
  },
  {
    id: "alpha-worktree",
    name: "alpha-worktree",
    kind: "worktree",
    created_at: "2026-04-11T11:00:00Z",
    workspace: workspaces[0],
    managed_worktree: {
      id: "wt-1",
      branch_name: "feature-a",
      source_ref: "origin/main",
      path: "/tmp/.amux-worktrees/alpha/feature-a"
    }
  }
];

test("workspace selection defaults to the first registered workspace", () => {
  const state = applyWorkspaces(createShellState("/app"), workspaces);

  assert.equal(state.selectedWorkspaceId, "ws-alpha");
});

test("route restoration keeps selected session when it still exists", () => {
  const state = applySessions(
    applyWorkspaces(createShellState("/app/sessions/alpha-worktree"), workspaces),
    sessions
  );

  assert.equal(state.selectedSessionId, "alpha-worktree");
  assert.equal(state.route.pathname, "/app/sessions/alpha-worktree");
  assert.equal(state.sessionUnavailable, false);
});

test("missing selected session normalizes back to /app", () => {
  const state = applySessions(
    applyWorkspaces(createShellState("/app/sessions/missing"), workspaces),
    sessions
  );

  assert.equal(state.selectedSessionId, null);
  assert.equal(state.route.pathname, "/app");
  assert.equal(state.sessionUnavailable, true);
});

test("create auto-selection navigates to the new session and requests focus", () => {
  const state = handleCreateSuccess(
    applyWorkspaces(createShellState("/app"), workspaces),
    "alpha-worktree",
    sessions
  );

  assert.equal(state.selectedSessionId, "alpha-worktree");
  assert.equal(state.route.pathname, "/app/sessions/alpha-worktree");
  assert.equal(state.focusTerminalInput, true);
});

test("polling only runs for visible selected sessions", () => {
  let state = applySessions(
    applyWorkspaces(createShellState("/app/sessions/alpha-local"), workspaces),
    sessions
  );
  assert.equal(shouldPollTerminal(state), true);

  state = setPageVisibility(state, false);
  assert.equal(shouldPollTerminal(state), false);

  state = setPageVisibility(state, true);
  state = {
    ...state,
    terminalUnavailable: true
  };
  assert.equal(shouldPollTerminal(state), false);
  assert.equal(POLL_INTERVAL_MS, 250);
});

test("lifecycle invalidation and reconnect both imply a REST refetch", () => {
  assert.equal(
    eventRequiresSessionRefresh({ event_type: "session.created" }),
    true
  );
  assert.equal(eventRequiresSessionRefresh({ event_type: "other.event" }), false);
  assert.equal(shouldRefetchSessionsOnSocketOpen(true), true);
  assert.equal(shouldRefetchSessionsOnSocketOpen(false), false);
});

test("shell markup includes workspace registration and worktree launch controls", () => {
  const state = {
    ...selectSession(
      applySessions(
        selectWorkspace(applyWorkspaces(createShellState("/app"), workspaces), "ws-alpha"),
        sessions
      ),
      "alpha-local"
    ),
    sourceRefs: [
      { name: "main", kind: "local_branch" },
      { name: "origin/main", kind: "remote_tracking_branch" }
    ],
    managedWorktrees: [sessions[1].managed_worktree]
  };
  const markup = renderShell(state);

  assert.match(markup, /id="register-workspace-form"/);
  assert.match(markup, /data-testid="workspace-list"/);
  assert.match(markup, /Create local session/);
  assert.match(markup, /Create managed worktree/);
  assert.match(markup, /data-worktree-session-create="wt-1"/);
  assert.match(markup, /alpha · worktree:feature-a/);
});

test("selecting a different workspace clears stale workspace resources", () => {
  const state = selectWorkspace(
    {
      ...applyWorkspaces(createShellState("/app"), workspaces),
      managedWorktrees: [{ id: "wt-1" }],
      sourceRefs: [{ name: "main", kind: "local_branch" }]
    },
    "ws-beta"
  );

  assert.equal(state.selectedWorkspaceId, "ws-beta");
  assert.deepEqual(state.managedWorktrees, []);
  assert.deepEqual(state.sourceRefs, []);
});

test("terminal input helpers map text and named keys onto daemon contract", () => {
  assert.deepEqual(buildTextInputRequest("ls", false), {
    events: [{ type: "text", text: "ls" }]
  });

  assert.deepEqual(buildTextInputRequest("ls -a", false, { appendEnter: true }), {
    events: [
      { type: "text", text: "ls -a" },
      {
        type: "key",
        key: { kind: "named", key: "enter" },
        ctrl: false,
        alt: false,
        shift: false
      }
    ]
  });

  assert.deepEqual(buildTextInputRequest("c", true), {
    events: [
      {
        type: "key",
        key: { kind: "character", text: "c" },
        ctrl: true,
        alt: false,
        shift: false
      }
    ]
  });

  assert.deepEqual(buildNamedKeyRequest("enter", true), {
    events: [
      {
        type: "key",
        key: { kind: "named", key: "enter" },
        ctrl: true,
        alt: false,
        shift: false
      }
    ]
  });
});
