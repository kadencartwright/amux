import test from "node:test";
import assert from "node:assert/strict";

import {
  POLL_INTERVAL_MS,
  applySessions,
  buildNamedKeyRequest,
  buildTextInputRequest,
  createShellState,
  eventRequiresSessionRefresh,
  handleCreateSuccess,
  renderShell,
  selectSession,
  setPageVisibility,
  shouldPollTerminal,
  shouldRefetchSessionsOnSocketOpen
} from "../src/core.js";

const sessions = [
  {
    id: "alpha",
    name: "alpha",
    created_at: "2026-04-11T10:00:00Z"
  },
  {
    id: "beta",
    name: "beta",
    created_at: "2026-04-11T11:00:00Z"
  }
];

test("route restoration keeps selected session when it still exists", () => {
  const state = applySessions(createShellState("/app/sessions/beta"), sessions);

  assert.equal(state.selectedSessionId, "beta");
  assert.equal(state.route.pathname, "/app/sessions/beta");
  assert.equal(state.sessionUnavailable, false);
});

test("missing selected session normalizes back to /app", () => {
  const state = applySessions(createShellState("/app/sessions/missing"), sessions);

  assert.equal(state.selectedSessionId, null);
  assert.equal(state.route.pathname, "/app");
  assert.equal(state.sessionUnavailable, true);
});

test("create auto-selection navigates to the new session and requests focus", () => {
  const state = handleCreateSuccess(createShellState("/app"), "beta", sessions);

  assert.equal(state.selectedSessionId, "beta");
  assert.equal(state.route.pathname, "/app/sessions/beta");
  assert.equal(state.focusTerminalInput, true);
});

test("polling only runs for visible selected sessions", () => {
  let state = applySessions(createShellState("/app/sessions/alpha"), sessions);
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
  assert.equal(
    eventRequiresSessionRefresh({ event_type: "other.event" }),
    false
  );
  assert.equal(shouldRefetchSessionsOnSocketOpen(true), true);
  assert.equal(shouldRefetchSessionsOnSocketOpen(false), false);
});

test("mobile shell markup includes drawer controls and modifier buttons", () => {
  const state = selectSession(
    applySessions(createShellState("/app"), sessions),
    "alpha"
  );
  const markup = renderShell(state);

  assert.match(markup, /data-action="open-mobile-nav"/);
  assert.match(markup, /data-testid="mobile-modifiers"/);
  assert.match(markup, /Ctrl/);
  assert.match(markup, /Esc/);
  assert.match(markup, /Enter/);
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
