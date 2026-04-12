import test from "node:test";
import assert from "node:assert/strict";

import {
  TERMINAL_STREAM_RECONNECT_DELAY_MS,
  applySessions,
  applyTerminalStreamFrame,
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
  setTerminalSurface,
  shouldConnectTerminalStream,
  shouldRefetchSessionsOnSocketOpen,
  shouldResyncTerminalOnVisibilityRestore
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

test("terminal stream only connects for visible selected sessions", () => {
  let state = applySessions(
    applyWorkspaces(createShellState("/app/sessions/alpha-local"), workspaces),
    sessions
  );
  assert.equal(shouldConnectTerminalStream(state), true);

  state = setPageVisibility(state, false);
  assert.equal(shouldConnectTerminalStream(state), false);

  state = setPageVisibility(state, true);
  state = {
    ...state,
    terminalUnavailable: true
  };
  assert.equal(shouldConnectTerminalStream(state), false);
  assert.equal(TERMINAL_STREAM_RECONNECT_DELAY_MS, 1000);
});

test("lifecycle invalidation and recovery triggers drive the expected resyncs", () => {
  const visibleState = applySessions(
    applyWorkspaces(createShellState("/app/sessions/alpha-local", "hidden"), workspaces),
    sessions
  );
  const restoredState = setPageVisibility(visibleState, true);

  assert.equal(
    eventRequiresSessionRefresh({ event_type: "session.created" }),
    true
  );
  assert.equal(eventRequiresSessionRefresh({ event_type: "other.event" }), false);
  assert.equal(shouldRefetchSessionsOnSocketOpen(true), true);
  assert.equal(shouldRefetchSessionsOnSocketOpen(false), false);
  assert.equal(shouldResyncTerminalOnVisibilityRestore(false, restoredState), true);
});

test("stream frames merge row diffs and advance the selected-session sequence", () => {
  const selected = selectSession(
    applySessions(
      selectWorkspace(applyWorkspaces(createShellState("/app"), workspaces), "ws-alpha"),
      sessions
    ),
    "alpha-local"
  );
  const surface = sampleTerminalSurface("alpha-local");

  let state = setTerminalSurface(selected, surface);
  let result = applyTerminalStreamFrame(state, {
    session_id: "alpha-local",
    sequence: 1,
    rows: 2,
    cols: 3,
    cursor: { row: 1, col: 2, visible: true },
    modes: surface.snapshot.modes,
    escape_sequence_metrics: surface.snapshot.escape_sequence_metrics,
    lines: [
      {
        row: 1,
        wrapped: false,
        cells: [
          blankCell(0, "p"),
          blankCell(1, "w"),
          blankCell(2, "d")
        ]
      }
    ]
  });

  assert.equal(result.needsResync, false);
  state = result.state;
  assert.equal(state.terminalLastSequence, 1);
  assert.deepEqual(
    state.terminalSurface.snapshot.lines[1].cells.map((cell) => cell.text),
    ["p", "w", "d"]
  );

  result = applyTerminalStreamFrame(state, {
    session_id: "alpha-local",
    sequence: 2,
    rows: 2,
    cols: 3,
    cursor: { row: 0, col: 1, visible: true },
    modes: surface.snapshot.modes,
    escape_sequence_metrics: surface.snapshot.escape_sequence_metrics,
    lines: [
      {
        row: 0,
        wrapped: false,
        cells: [
          blankCell(0, "o"),
          blankCell(1, "k"),
          blankCell(2, "!")
        ]
      }
    ]
  });

  assert.equal(result.needsResync, false);
  assert.equal(result.state.terminalLastSequence, 2);
  assert.deepEqual(
    result.state.terminalSurface.snapshot.lines[0].cells.map((cell) => cell.text),
    ["o", "k", "!"]
  );
});

test("sequence gaps force a full selected-session resync", () => {
  const baseState = setTerminalSurface(
    selectSession(
      applySessions(
        selectWorkspace(applyWorkspaces(createShellState("/app"), workspaces), "ws-alpha"),
        sessions
      ),
      "alpha-local"
    ),
    sampleTerminalSurface("alpha-local")
  );
  const state = {
    ...baseState,
    terminalLastSequence: 2
  };

  const result = applyTerminalStreamFrame(state, {
    session_id: "alpha-local",
    sequence: 4,
    rows: 2,
    cols: 3,
    cursor: { row: 1, col: 0, visible: true },
    modes: state.terminalSurface.snapshot.modes,
    escape_sequence_metrics: state.terminalSurface.snapshot.escape_sequence_metrics,
    lines: []
  });

  assert.equal(result.needsResync, true);
  assert.equal(result.reason, "sequence_gap");
});

test("shell markup keeps the selected-session overview minimal and stream-focused", () => {
  const state = {
    ...setTerminalSurface(
      selectSession(
        applySessions(
          selectWorkspace(applyWorkspaces(createShellState("/app"), workspaces), "ws-alpha"),
          sessions
        ),
        "alpha-local"
      ),
      sampleTerminalSurface("alpha-local")
    ),
    terminalStreamStatus: "connected",
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
  assert.match(markup, /data-role="terminal-stream-status">Stream connected/);
  assert.doesNotMatch(markup, /terminal-shell__context/);
});

test("losing the selected session clears stale terminal state", () => {
  const selectedState = {
    ...setTerminalSurface(
      selectSession(
        applySessions(
          selectWorkspace(applyWorkspaces(createShellState("/app"), workspaces), "ws-alpha"),
          sessions
        ),
        "alpha-local"
      ),
      sampleTerminalSurface("alpha-local")
    ),
    terminalLastSequence: 7
  };
  const nextState = applySessions(selectedState, [sessions[1]]);

  assert.equal(nextState.selectedSessionId, null);
  assert.equal(nextState.route.pathname, "/app");
  assert.equal(nextState.terminalSurface, null);
  assert.equal(nextState.terminalLastSequence, null);
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

function sampleTerminalSurface(sessionId) {
  return {
    session_id: sessionId,
    snapshot: {
      rows: 2,
      cols: 3,
      cursor: { row: 0, col: 0, visible: true },
      modes: {
        application_cursor: false,
        application_keypad: false,
        bracketed_paste: true,
        alternate_screen: false
      },
      escape_sequence_metrics: {
        print: 0,
        execute: 0,
        csi: 0,
        esc: 0,
        osc: 0,
        dcs: 0
      },
      lines: [
        {
          row: 0,
          wrapped: false,
          cells: [blankCell(0, "a"), blankCell(1, "b"), blankCell(2, "c")]
        },
        {
          row: 1,
          wrapped: false,
          cells: [blankCell(0, "x"), blankCell(1, "y"), blankCell(2, "z")]
        }
      ],
      plain_text: "abc\nxyz"
    }
  };
}

function blankCell(column, text) {
  return {
    column,
    text,
    column_span: 1,
    unicode_width: text ? 1 : 0,
    grapheme_count: text ? 1 : 0,
    is_wide: false,
    is_wide_continuation: false,
    foreground: { kind: "default" },
    background: { kind: "default" },
    bold: false,
    italic: false,
    underline: false,
    inverse: false
  };
}
