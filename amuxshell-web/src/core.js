export const TERMINAL_STREAM_RECONNECT_DELAY_MS = 1000;

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

export function createShellState(
  pathname,
  visibilityState = "visible",
  isMobileBrowser = false
) {
  return {
    route: parseShellRoute(pathname),
    sessions: [],
    workspaces: [],
    managedWorktrees: [],
    sourceRefs: [],
    selectedSessionId: null,
    selectedWorkspaceId: null,
    sessionUnavailable: false,
    terminalUnavailable: false,
    terminalSurface: null,
    terminalStreamStatus: "idle",
    terminalLastSequence: null,
    pageVisible: visibilityState !== "hidden",
    mobileNavOpen: false,
    socketStatus: "connecting",
    focusTerminalInput: false,
    isMobileBrowser,
    ctrlModifierLatched: false,
    inputDraft: "",
    notice: null
  };
}

export function detectMobileBrowser(userAgent = "") {
  const normalized = String(userAgent).toLowerCase();
  return /iphone|ipad|ipod|android/.test(normalized);
}

export function applyWorkspaces(state, workspaces) {
  const next = normalizeWorkspaceSelection({
    ...state,
    workspaces: [...workspaces]
  });

  if (next.selectedWorkspaceId !== state.selectedWorkspaceId) {
    return {
      ...next,
      managedWorktrees: [],
      sourceRefs: []
    };
  }

  return next;
}

export function applyWorkspaceResources(state, { managedWorktrees, sourceRefs }) {
  return {
    ...state,
    managedWorktrees: [...managedWorktrees],
    sourceRefs: [...sourceRefs]
  };
}

export function selectWorkspace(state, workspaceId) {
  return normalizeWorkspaceSelection({
    ...state,
    selectedWorkspaceId: workspaceId,
    managedWorktrees: [],
    sourceRefs: [],
    mobileNavOpen: false
  });
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
      terminalStreamStatus: next.selectedSessionId ? "disconnected" : "idle",
      terminalLastSequence: null,
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
    terminalUnavailable: false,
    terminalLastSequence: null
  };
}

export function setTerminalUnavailable(state, unavailable) {
  return {
    ...state,
    terminalUnavailable: unavailable,
    terminalSurface: unavailable ? null : state.terminalSurface,
    terminalStreamStatus: unavailable ? "disconnected" : state.terminalStreamStatus,
    terminalLastSequence: unavailable ? null : state.terminalLastSequence
  };
}

export function setTerminalStreamStatus(state, terminalStreamStatus) {
  return {
    ...state,
    terminalStreamStatus
  };
}

export function setTerminalSequence(state, terminalLastSequence) {
  return {
    ...state,
    terminalLastSequence
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

export function setNotice(state, message, tone = "info") {
  return {
    ...state,
    notice: {
      message,
      tone
    }
  };
}

export function clearNotice(state) {
  return {
    ...state,
    notice: null
  };
}

export function shouldConnectTerminalStream(state) {
  return Boolean(
    state.selectedSessionId && state.pageVisible && !state.terminalUnavailable
  );
}

export function shouldResyncTerminalOnVisibilityRestore(wasVisible, state) {
  return Boolean(!wasVisible && state.pageVisible && state.selectedSessionId);
}

export function eventRequiresSessionRefresh(event) {
  return Boolean(
    event && typeof event.event_type === "string" && event.event_type.startsWith("session.")
  );
}

export function shouldRefetchSessionsOnSocketOpen(reconnected) {
  return Boolean(reconnected);
}

export function applyTerminalStreamFrame(state, frame) {
  if (!state.terminalSurface) {
    return {
      state,
      needsResync: true,
      reason: "missing_snapshot"
    };
  }

  if (
    state.terminalLastSequence !== null &&
    frame.sequence !== state.terminalLastSequence + 1
  ) {
    return {
      state,
      needsResync: true,
      reason: "sequence_gap"
    };
  }

  const previousSnapshot = state.terminalSurface.snapshot;
  const baseLines =
    previousSnapshot.rows === frame.rows && previousSnapshot.cols === frame.cols
      ? previousSnapshot.lines.map((line) => ({
          row: line.row,
          wrapped: line.wrapped,
          cells: line.cells.map((cell) => ({ ...cell }))
        }))
      : createBlankTerminalLines(frame.rows, frame.cols);
  const lineIndex = new Map(baseLines.map((line, index) => [line.row, index]));

  for (const line of frame.lines) {
    const replacement = {
      row: line.row,
      wrapped: line.wrapped,
      cells: line.cells.map((cell) => ({ ...cell }))
    };
    const existingIndex = lineIndex.get(line.row);
    if (existingIndex === undefined) {
      baseLines.push(replacement);
      continue;
    }
    baseLines[existingIndex] = replacement;
  }

  baseLines.sort((left, right) => left.row - right.row);

  return {
    state: {
      ...state,
      terminalUnavailable: false,
      terminalLastSequence: frame.sequence,
      terminalSurface: {
        ...state.terminalSurface,
        snapshot: {
          ...previousSnapshot,
          rows: frame.rows,
          cols: frame.cols,
          cursor: frame.cursor,
          modes: frame.modes,
          escape_sequence_metrics: frame.escape_sequence_metrics,
          lines: baseLines
        }
      }
    },
    needsResync: false,
    reason: null
  };
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
  const selectedWorkspace = state.workspaces.find(
    (workspace) => workspace.id === state.selectedWorkspaceId
  );

  const banners = [
    state.notice
      ? `<div class="shell-banner shell-banner--${escapeAttribute(state.notice.tone)}">${escapeHtml(
          state.notice.message
        )}</div>`
      : "",
    state.sessionUnavailable
      ? `<div class="shell-banner shell-banner--warning">The selected session is no longer available. The shell has been normalized back to <code>/app</code>.</div>`
      : ""
  ]
    .filter(Boolean)
    .join("");

  return `
    <div class="shell ${state.mobileNavOpen ? "shell--nav-open" : ""}">
      <div class="shell__backdrop" data-action="close-mobile-nav"></div>
      <aside class="shell__sidebar" aria-label="Workspaces and sessions">
        ${renderSidebar(state, selectedWorkspace, selectedSession)}
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
              Workspaces
            </button>
          </div>
        </header>
        ${banners}
        ${renderMainPane(state, selectedWorkspace, selectedSession)}
      </main>
    </div>
  `;
}

function renderSidebar(state, selectedWorkspace, selectedSession) {
  const workspaceItems = state.workspaces.length
    ? state.workspaces
        .map((workspace) => {
          const selected = workspace.id === state.selectedWorkspaceId;
          return `
            <li class="workspace-list__item ${selected ? "workspace-list__item--selected" : ""}">
              <button
                type="button"
                class="workspace-list__select"
                data-workspace-select="${escapeAttribute(workspace.id)}"
              >
                <span class="workspace-list__name">${escapeHtml(workspace.name)}</span>
                <span class="workspace-list__meta">${escapeHtml(workspace.kind)} · ${escapeHtml(
                  workspace.root_path
                )}</span>
              </button>
            </li>
          `;
        })
        .join("")
    : `<li class="workspace-list__empty">Register a workspace to create local or worktree sessions.</li>`;

  const selectedWorkspaceName = selectedWorkspace
    ? escapeHtml(selectedWorkspace.name)
    : "Select a workspace";
  const localSessionDisabled = selectedWorkspace ? "" : "disabled";

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
                <span class="session-list__meta">${escapeHtml(sessionContextLabel(session))}</span>
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
    : `<li class="session-list__empty">No sessions yet. Create a local session or launch one from a managed worktree.</li>`;

  return `
    <div class="shell__sidebar-header">
      <div>
        <p class="shell__eyebrow">Workspace-scoped shell rail</p>
        <h2>Workspaces</h2>
      </div>
      <button
        class="shell__sidebar-close"
        type="button"
        data-action="close-mobile-nav"
        aria-label="Close shell drawer"
      >
        Close
      </button>
    </div>
    <form id="register-workspace-form" class="session-create-form workspace-register-form">
      <label class="session-create-form__label" for="workspace-root-path">Register workspace</label>
      <input
        id="workspace-root-path"
        name="workspace-root-path"
        type="text"
        placeholder="/path/to/workspace"
        autocomplete="off"
      />
      <input
        name="workspace-name"
        type="text"
        placeholder="optional display name"
        autocomplete="off"
      />
      <button type="submit">Add workspace</button>
    </form>
    <ul class="workspace-list" data-testid="workspace-list">${workspaceItems}</ul>
    <form id="create-local-session-form" class="session-create-form">
      <label class="session-create-form__label" for="session-name">Local session</label>
      <p class="session-create-form__hint">${selectedWorkspaceName}</p>
      <input
        id="session-name"
        name="session-name"
        type="text"
        placeholder="session name"
        autocomplete="off"
        ${localSessionDisabled}
      />
      <button type="submit" ${localSessionDisabled}>Create local session</button>
    </form>
    <div class="shell__sidebar-header shell__sidebar-header--sessions">
      <div>
        <p class="shell__eyebrow">Runtime sessions</p>
        <h2>Sessions</h2>
      </div>
    </div>
    <ul class="session-list">${sessionItems}</ul>
  `;
}

function renderMainPane(state, selectedWorkspace, selectedSession) {
  return `
    <div class="shell-stack">
      ${selectedSession ? renderTerminalPane(state, selectedSession) : renderEmptyState(selectedWorkspace)}
      ${renderWorkspacePanel(state, selectedWorkspace)}
    </div>
  `;
}

function renderEmptyState(selectedWorkspace) {
  if (selectedWorkspace) {
    return `
      <section class="shell-empty">
        <p class="shell-empty__title">No active session selected</p>
        <p>Create a local session for <strong>${escapeHtml(
          selectedWorkspace.name
        )}</strong> or launch one from a managed worktree.</p>
      </section>
    `;
  }

  return `
    <section class="shell-empty">
      <p class="shell-empty__title">No workspace selected</p>
      <p>Register a workspace from the rail to unlock local sessions, source-ref discovery, and managed worktrees.</p>
    </section>
  `;
}

function renderTerminalPane(state, selectedSession) {
  const unavailable = state.terminalUnavailable
    ? `<div class="terminal-unavailable">
         <p class="terminal-unavailable__title">Terminal surface unavailable</p>
         <p>The daemon session controls are still live, but terminal routes are disabled or unavailable for this session.</p>
       </div>`
    : `<div class="terminal-frame">
         <div id="terminal-canvas" class="terminal-frame__canvas" aria-label="Terminal surface"></div>
       </div>`;

  const terminalMeta = terminalStreamLabel(state.terminalStreamStatus);

  const inputControls = state.terminalUnavailable
    ? ""
    : `
      <div class="terminal-input terminal-input--mobile ${
        state.isMobileBrowser ? "" : "terminal-input--hidden"
      }" data-testid="mobile-input-area">
        <div class="terminal-modifiers" data-testid="mobile-modifiers">
          ${MOBILE_MODIFIER_CONTROLS.map((control) => renderModifierButton(control, state.ctrlModifierLatched)).join("")}
        </div>
        <label class="terminal-input__label" for="terminal-mobile-input">Tap to type</label>
        <input
          id="terminal-mobile-input"
          class="terminal-input__mobile"
          type="text"
          autocapitalize="off"
          autocomplete="off"
          autocorrect="off"
          spellcheck="false"
          inputmode="text"
          placeholder=""
          value=""
        />
      </div>
    `;

  return `
    <section class="terminal-shell">
      <div class="terminal-shell__header">
        <div>
          <p class="shell__eyebrow">Selected session</p>
          <h2>${escapeHtml(selectedSession.name)}</h2>
        </div>
        <span class="shell__status shell__status--${escapeAttribute(state.terminalStreamStatus)}" data-role="terminal-stream-status">${escapeHtml(terminalMeta)}</span>
      </div>
      ${unavailable}
      ${inputControls}
    </section>
  `;
}

function renderWorkspacePanel(state, selectedWorkspace) {
  if (!selectedWorkspace) {
    return `
      <section class="workspace-panel shell-empty">
        <p class="shell-empty__title">Workspace control plane</p>
        <p>Once you register a workspace, this panel will show source refs, managed worktrees, and worktree session launch controls.</p>
      </section>
    `;
  }

  const worktreeItems = state.managedWorktrees.length
    ? state.managedWorktrees
        .map((worktree) => {
          return `
            <li class="worktree-list__item">
              <div>
                <p class="worktree-list__branch">${escapeHtml(worktree.branch_name)}</p>
                <p class="worktree-list__meta">Base ${escapeHtml(worktree.source_ref)}</p>
                <p class="worktree-list__meta">${escapeHtml(worktree.path)}</p>
              </div>
              <button
                type="button"
                class="worktree-list__launch"
                data-worktree-session-create="${escapeAttribute(worktree.id)}"
                data-worktree-session-name="${escapeAttribute(worktree.branch_name)}"
              >
                Start session
              </button>
            </li>
          `;
        })
        .join("")
    : `<li class="worktree-list__empty">No managed worktrees yet.</li>`;

  const sourceRefOptions = state.sourceRefs.length
    ? state.sourceRefs
        .map((sourceRef) => {
          return `<option value="${escapeAttribute(sourceRef.name)}">${escapeHtml(
            sourceRef.name
          )} (${escapeHtml(sourceRef.kind)})</option>`;
        })
        .join("")
    : `<option value="">No source refs available</option>`;

  const worktreeCreateForm =
    selectedWorkspace.kind === "git"
      ? `
        <form id="create-worktree-form" class="session-create-form workspace-worktree-form">
          <label class="session-create-form__label" for="worktree-source-ref">Create managed worktree</label>
          <select
            id="worktree-source-ref"
            name="worktree-source-ref"
            ${state.sourceRefs.length ? "" : "disabled"}
          >
            ${sourceRefOptions}
          </select>
          <input
            name="worktree-branch-name"
            type="text"
            placeholder="new branch name"
            autocomplete="off"
          />
          <button type="submit" ${state.sourceRefs.length ? "" : "disabled"}>Create managed worktree</button>
        </form>
      `
      : `
        <div class="workspace-panel__note">
          <p class="workspace-panel__note-title">Managed worktrees unavailable</p>
          <p>This workspace is classified as <code>none</code>, so only local sessions are supported.</p>
        </div>
      `;

  return `
    <section class="workspace-panel" data-testid="workspace-panel">
      <div class="workspace-panel__header">
        <div>
          <p class="shell__eyebrow">Selected workspace</p>
          <h2>${escapeHtml(selectedWorkspace.name)}</h2>
        </div>
        <div class="workspace-panel__meta">
          <span class="workspace-panel__pill">${escapeHtml(selectedWorkspace.kind)}</span>
          <code>${escapeHtml(selectedWorkspace.root_path)}</code>
        </div>
      </div>
      ${worktreeCreateForm}
      <div class="workspace-panel__section">
        <div class="workspace-panel__section-header">
          <h3>Managed worktrees</h3>
          <span class="workspace-panel__section-meta">${escapeHtml(String(state.managedWorktrees.length))} tracked</span>
        </div>
        <ul class="worktree-list" data-testid="managed-worktree-list">${worktreeItems}</ul>
      </div>
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

function normalizeWorkspaceSelection(state) {
  if (!state.workspaces.length) {
    return {
      ...state,
      selectedWorkspaceId: null,
      managedWorktrees: [],
      sourceRefs: []
    };
  }

  if (state.workspaces.some((workspace) => workspace.id === state.selectedWorkspaceId)) {
    return state;
  }

  return {
    ...state,
    selectedWorkspaceId: state.workspaces[0].id
  };
}

function sessionContextLabel(session) {
  const workspaceName = session.workspace?.name || "workspace";
  if (session.kind === "worktree" && session.managed_worktree) {
    return `${workspaceName} · worktree:${session.managed_worktree.branch_name}`;
  }

  return `${workspaceName} · local`;
}

function createBlankTerminalLines(rows, cols) {
  return Array.from({ length: rows }, (_, row) => ({
    row,
    wrapped: false,
    cells: Array.from({ length: cols }, (_, column) => ({
      column,
      text: "",
      column_span: 1,
      unicode_width: 0,
      grapheme_count: 0,
      is_wide: false,
      is_wide_continuation: false,
      foreground: { kind: "default" },
      background: { kind: "default" },
      bold: false,
      italic: false,
      underline: false,
      inverse: false
    }))
  }));
}

function terminalStreamLabel(status) {
  switch (status) {
    case "connected":
      return "Stream connected";
    case "connecting":
      return "Stream connecting";
    case "reconnecting":
      return "Stream reconnecting";
    case "disconnected":
      return "Stream disconnected";
    default:
      return "Stream idle";
  }
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
