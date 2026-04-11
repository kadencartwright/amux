export const POLL_INTERVAL_MS = 250;

export const MOBILE_MODIFIER_CONTROLS = [
  { label: "Ctrl", action: "toggle-ctrl" },
  { label: "Esc", namedKey: "escape" },
  { label: "Tab", namedKey: "tab" },
  { label: "Up", namedKey: "arrow_up" },
  { label: "Down", namedKey: "arrow_down" },
  { label: "Left", namedKey: "arrow_left" },
  { label: "Right", namedKey: "arrow_right" },
  { label: "Enter", namedKey: "enter" }
];

export function parseShellRoute(pathname) {
  const normalized = normalizePath(pathname);
  const match = normalized.match(/^\/app\/sessions\/([^/]+)$/);
  if (match) {
    return {
      kind: "session",
      pathname: normalized,
      sessionId: decodeURIComponent(match[1])
    };
  }

  return {
    kind: "index",
    pathname: "/app"
  };
}

export function sessionRoute(sessionId) {
  return `/app/sessions/${encodeURIComponent(sessionId)}`;
}

export function createShellState(pathname, visibilityState = "visible") {
  return {
    route: parseShellRoute(pathname),
    sessions: [],
    selectedSessionId: null,
    sessionUnavailable: false,
    terminalUnavailable: false,
    terminalSurface: null,
    pageVisible: visibilityState !== "hidden",
    mobileNavOpen: false,
    socketStatus: "connecting",
    focusTerminalInput: false,
    ctrlModifierLatched: false,
    inputDraft: ""
  };
}

export function applySessions(state, sessions) {
  const previousSelection = state.selectedSessionId;
  const next = normalizeSelection({
    ...state,
    sessions: [...sessions]
  });

  if (next.selectedSessionId !== previousSelection) {
    return {
      ...next,
      terminalSurface: null,
      terminalUnavailable: false,
      ctrlModifierLatched: false
    };
  }

  return next;
}

export function selectSession(state, sessionId) {
  return normalizeSelection({
    ...state,
    route: parseShellRoute(sessionRoute(sessionId)),
    sessionUnavailable: false,
    terminalUnavailable: false,
    terminalSurface: null,
    mobileNavOpen: false,
    focusTerminalInput: true,
    ctrlModifierLatched: false
  });
}

export function handleCreateSuccess(state, sessionId, sessions) {
  return applySessions(
    {
      ...state,
      route: parseShellRoute(sessionRoute(sessionId)),
      sessionUnavailable: false,
      focusTerminalInput: true,
      mobileNavOpen: false
    },
    sessions
  );
}

export function setPageVisibility(state, visible) {
  return {
    ...state,
    pageVisible: visible
  };
}

export function setMobileNavOpen(state, open) {
  return {
    ...state,
    mobileNavOpen: open
  };
}

export function acknowledgeInputFocus(state) {
  return {
    ...state,
    focusTerminalInput: false
  };
}

export function setSocketStatus(state, socketStatus) {
  return {
    ...state,
    socketStatus
  };
}

export function setInputDraft(state, inputDraft) {
  return {
    ...state,
    inputDraft
  };
}

export function setTerminalSurface(state, terminalSurface) {
  return {
    ...state,
    terminalSurface,
    terminalUnavailable: false
  };
}

export function setTerminalUnavailable(state, unavailable) {
  return {
    ...state,
    terminalUnavailable: unavailable,
    terminalSurface: unavailable ? null : state.terminalSurface
  };
}

export function toggleCtrlModifier(state) {
  return {
    ...state,
    ctrlModifierLatched: !state.ctrlModifierLatched
  };
}

export function clearCtrlModifier(state) {
  return {
    ...state,
    ctrlModifierLatched: false
  };
}

export function shouldPollTerminal(state) {
  return Boolean(
    state.selectedSessionId && state.pageVisible && !state.terminalUnavailable
  );
}

export function eventRequiresSessionRefresh(event) {
  return Boolean(event && typeof event.event_type === "string" && event.event_type.startsWith("session."));
}

export function shouldRefetchSessionsOnSocketOpen(reconnected) {
  return Boolean(reconnected);
}

export function buildTextInputRequest(
  draft,
  ctrlModifierLatched,
  { appendEnter = false } = {}
) {
  if (!draft) {
    return { events: [] };
  }

  if (ctrlModifierLatched && [...draft].length === 1) {
    return {
      events: [
        {
          type: "key",
          key: {
            kind: "character",
            text: draft
          },
          ctrl: true,
          alt: false,
          shift: false
        }
      ]
    };
  }

  return {
    events: [
      {
        type: "text",
        text: draft
      },
      ...(appendEnter
        ? [
            {
              type: "key",
              key: {
                kind: "named",
                key: "enter"
              },
              ctrl: false,
              alt: false,
              shift: false
            }
          ]
        : [])
    ]
  };
}

export function buildNamedKeyRequest(namedKey, ctrlModifierLatched) {
  return {
    events: [
      {
        type: "key",
        key: {
          kind: "named",
          key: namedKey
        },
        ctrl: ctrlModifierLatched,
        alt: false,
        shift: false
      }
    ]
  };
}

export function renderShell(state) {
  const selectedSession = state.sessions.find(
    (session) => session.id === state.selectedSessionId
  );

  const sidebar = renderSidebar(state, selectedSession);
  const banner = state.sessionUnavailable
    ? `<div class="shell-banner shell-banner--warning">The selected session is no longer available. The shell has been normalized back to <code>/app</code>.</div>`
    : "";

  const main = renderMainPane(state, selectedSession);

  return `
    <div class="shell ${state.mobileNavOpen ? "shell--nav-open" : ""}">
      <div class="shell__backdrop" data-action="close-mobile-nav"></div>
      <aside class="shell__sidebar" aria-label="Sessions">
        ${sidebar}
      </aside>
      <main class="shell__main">
        <header class="shell__header">
          <div>
            <p class="shell__eyebrow">Daemon-served browser shell</p>
            <h1>AMUX Shell</h1>
          </div>
          <div class="shell__header-actions">
            <span class="shell__status shell__status--${escapeAttribute(state.socketStatus)}">${escapeHtml(
              state.socketStatus
            )}</span>
            <button
              class="shell__nav-toggle"
              type="button"
              data-action="open-mobile-nav"
            >
              Sessions
            </button>
          </div>
        </header>
        ${banner}
        ${main}
      </main>
    </div>
  `;
}

function renderSidebar(state, selectedSession) {
  const sessionItems = state.sessions.length
    ? state.sessions
        .map((session) => {
          const selected = selectedSession && session.id === selectedSession.id;
          return `
            <li class="session-list__item ${selected ? "session-list__item--selected" : ""}">
              <button
                type="button"
                class="session-list__select"
                data-session-select="${escapeAttribute(session.id)}"
              >
                <span class="session-list__name">${escapeHtml(session.name)}</span>
                <span class="session-list__meta">${escapeHtml(relativeTimestamp(session.created_at))}</span>
              </button>
              <button
                type="button"
                class="session-list__terminate"
                data-session-terminate="${escapeAttribute(session.id)}"
                aria-label="Terminate ${escapeAttribute(session.name)}"
              >
                End
              </button>
            </li>
          `;
        })
        .join("")
    : `<li class="session-list__empty">No sessions yet. Create one to start the shell loop.</li>`;

  return `
    <div class="shell__sidebar-header">
      <div>
        <p class="shell__eyebrow">Single-session control rail</p>
        <h2>Sessions</h2>
      </div>
      <button
        class="shell__sidebar-close"
        type="button"
        data-action="close-mobile-nav"
        aria-label="Close session drawer"
      >
        Close
      </button>
    </div>
    <form id="create-session-form" class="session-create-form">
      <label class="session-create-form__label" for="session-name">New session</label>
      <input
        id="session-name"
        name="session-name"
        type="text"
        placeholder="session name"
        autocomplete="off"
      />
      <button type="submit">Create session</button>
    </form>
    <ul class="session-list">${sessionItems}</ul>
  `;
}

function renderMainPane(state, selectedSession) {
  if (!selectedSession) {
    return `
      <section class="shell-empty">
        <p class="shell-empty__title">No active session selected</p>
        <p>Select a session from the rail or create a new one to start polling terminal state.</p>
      </section>
    `;
  }

  const unavailable = state.terminalUnavailable
    ? `<div class="terminal-unavailable">
         <p class="terminal-unavailable__title">Terminal surface unavailable</p>
         <p>The daemon session controls are still live, but terminal routes are disabled or unavailable for this session.</p>
       </div>`
    : `<div class="terminal-frame">
         <canvas id="terminal-canvas" class="terminal-frame__canvas" aria-label="Terminal surface"></canvas>
       </div>`;

  const terminalMeta = state.terminalUnavailable
    ? "Terminal polling paused"
    : state.terminalSurface
      ? `Polling every ${POLL_INTERVAL_MS} ms while visible`
      : "Waiting for the first terminal snapshot";

  const inputControls = state.terminalUnavailable
    ? ""
    : `
      <form id="terminal-input-form" class="terminal-input">
        <label for="terminal-input">Terminal input</label>
        <textarea
          id="terminal-input"
          rows="3"
          placeholder="Type text for the selected session"
        >${escapeHtml(state.inputDraft)}</textarea>
        <div class="terminal-input__actions">
          <button type="submit">Send + Enter</button>
          <span class="terminal-input__hint">The submit button sends the text and presses Enter, then refreshes the terminal.</span>
        </div>
      </form>
      <div class="terminal-modifiers" data-testid="mobile-modifiers">
        ${MOBILE_MODIFIER_CONTROLS.map((control) => renderModifierButton(control, state.ctrlModifierLatched)).join("")}
      </div>
    `;

  return `
    <section class="terminal-shell">
      <div class="terminal-shell__header">
        <div>
          <p class="shell__eyebrow">Selected session</p>
          <h2>${escapeHtml(selectedSession.name)}</h2>
        </div>
        <p class="terminal-shell__meta" data-role="terminal-meta">${escapeHtml(terminalMeta)}</p>
      </div>
      ${unavailable}
      ${inputControls}
    </section>
  `;
}

function renderModifierButton(control, ctrlModifierLatched) {
  if (control.action === "toggle-ctrl") {
    return `
      <button
        type="button"
        class="terminal-modifiers__button ${ctrlModifierLatched ? "terminal-modifiers__button--active" : ""}"
        data-action="toggle-ctrl"
      >
        ${escapeHtml(control.label)}
      </button>
    `;
  }

  return `
    <button
      type="button"
      class="terminal-modifiers__button"
      data-terminal-key="${escapeAttribute(control.namedKey)}"
    >
      ${escapeHtml(control.label)}
    </button>
  `;
}

function normalizeSelection(state) {
  if (state.route.kind !== "session") {
    return {
      ...state,
      route: { kind: "index", pathname: "/app" },
      selectedSessionId: null,
      sessionUnavailable: false
    };
  }

  const found = state.sessions.find((session) => session.id === state.route.sessionId);
  if (!found) {
    return {
      ...state,
      route: { kind: "index", pathname: "/app" },
      selectedSessionId: null,
      sessionUnavailable: true
    };
  }

  return {
    ...state,
    selectedSessionId: found.id,
    sessionUnavailable: false
  };
}

function normalizePath(pathname) {
  if (!pathname || pathname === "/") {
    return "/app";
  }

  const trimmed = pathname.replace(/\/+$/, "");
  return trimmed || "/app";
}

function escapeHtml(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}

function escapeAttribute(value) {
  return escapeHtml(value).replaceAll("'", "&#39;");
}

function relativeTimestamp(timestamp) {
  if (!timestamp) {
    return "just now";
  }

  const date = new Date(timestamp);
  if (Number.isNaN(date.valueOf())) {
    return timestamp;
  }

  const diffMs = Date.now() - date.valueOf();
  const diffMinutes = Math.round(diffMs / 60_000);
  if (Math.abs(diffMinutes) < 1) {
    return "just now";
  }
  if (Math.abs(diffMinutes) < 60) {
    return `${diffMinutes}m ago`;
  }

  const diffHours = Math.round(diffMinutes / 60);
  if (Math.abs(diffHours) < 24) {
    return `${diffHours}h ago`;
  }

  const diffDays = Math.round(diffHours / 24);
  return `${diffDays}d ago`;
}
