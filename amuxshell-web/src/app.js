import { FitAddon, Terminal, init as initGhostty } from "./vendor/ghostty-web.js";
import {
  acknowledgeInputFocus,
  applySessions,
  applyWorkspaceResources,
  applyWorkspaces,
  clearCtrlModifier,
  clearNotice,
  createShellState,
  detectMobileBrowser,
  eventRequiresSessionRefresh,
  handleCreateSuccess,
  parseShellRoute,
  renderShell,
  selectSession,
  selectWorkspace,
  sessionRoute,
  setMobileNavOpen,
  setNotice,
  setPageVisibility,
  setSocketStatus,
  setTerminalSurface,
  setTerminalStreamStatus,
  setTerminalUnavailable,
  shouldConnectTerminalStream,
  shouldResyncTerminalOnVisibilityRestore,
  shouldRefetchSessionsOnSocketOpen,
  TERMINAL_STREAM_RECONNECT_DELAY_MS,
  toggleCtrlModifier
} from "./core.js";

const root = document.querySelector("#shell-root");

let state = createShellState(
  window.location.pathname,
  document.visibilityState,
  detectMobileBrowser(window.navigator.userAgent)
);
let terminal = null;
let terminalHost = null;
let fitAddon = null;
let terminalBootstrapFingerprint = null;
const textEncoder = new TextEncoder();
let socket = null;
let reconnectTimer = null;
let terminalSocket = null;
let terminalSocketSessionId = null;
let terminalExpectedCloseSocket = null;
let terminalReconnectTimer = null;
let terminalStreamReady = false;
let terminalBufferedFrames = [];
let terminalFrameQueue = Promise.resolve();
let terminalRequestId = 0;
let terminalReconnectFailures = 0;
let terminalLastOpenedAt = 0;

root.addEventListener("click", (event) => {
  void runAction(() => handleClick(event));
});
root.addEventListener("submit", (event) => {
  void runAction(() => handleSubmit(event));
});
root.addEventListener("input", handleInput);
root.addEventListener("keydown", handleKeyDown);
window.addEventListener("popstate", () => {
  void runAction(handlePopState);
});
window.addEventListener("resize", handleResize);
window.addEventListener("keydown", handleDesktopKeyDown);
document.addEventListener("visibilitychange", () => {
  void runAction(handleVisibilityChange);
});

boot().catch((error) => {
  console.error("shell boot failed", error);
  root.innerHTML = `
    <main class="shell shell--fatal">
      <section class="shell-empty">
        <p class="shell-empty__title">Shell boot failed</p>
        <p>${escapeHtml(error.message || String(error))}</p>
      </section>
    </main>
  `;
});

async function boot() {
  render();
  await initGhostty();
  await refetchWorkspaces();
  await refetchSessions({ replaceHistory: true });
  connectSocket(false);
  await ensureSelectedTerminalTransport(false, true);
}

async function runAction(action) {
  try {
    state = clearNotice(state);
    await action();
  } catch (error) {
    console.error("shell action failed", error);
    state = setNotice(state, error.message || String(error), "error");
    render();
  }
}

function render() {
  root.innerHTML = renderShell(state);
  if (state.focusTerminalInput) {
    requestAnimationFrame(() => {
      root.querySelector("#terminal-input")?.focus();
    });
    state = acknowledgeInputFocus(state);
  }
  paintTerminal();
}

function paintTerminal() {
  const host = root.querySelector("#terminal-canvas");
  if (!host || !state.terminalSurface || state.terminalUnavailable) {
    terminal = null;
    terminalHost = null;
    fitAddon = null;
    terminalBootstrapFingerprint = null;
    return;
  }

  if (!terminal || terminalHost !== host) {
    terminalHost = host;
    terminal = new Terminal({
      fontFamily: "IBM Plex Mono, monospace",
      fontSize: 14,
      cursorBlink: true,
      scrollback: 0,
      theme: {
        background: "#061018",
        foreground: "#e5edf0"
      }
    });
    fitAddon = new FitAddon();
    terminal.loadAddon(fitAddon);
    terminal.open(host);
    fitAddon.fit();
    terminal.onData((data) => {
      sendTerminalBytes(textEncoder.encode(data));
    });
    terminalBootstrapFingerprint = null;
  }

  const fingerprint = `${state.selectedSessionId || ""}:${state.terminalSurface.snapshot?.plain_text || ""}`;
  if (terminalBootstrapFingerprint !== fingerprint) {
    terminal.reset();
    terminal.write(visibleSnapshotText(state.terminalSurface));
    terminalBootstrapFingerprint = fingerprint;
  }
  fitAddon?.fit();
  terminal.focus();
}

function visibleSnapshotText(surface) {
  if (!surface?.snapshot?.lines) {
    return surface?.snapshot?.plain_text || "";
  }
  return surface.snapshot.lines
    .map((line) => (line.cells || []).map((cell) => cell.text || "").join(""))
    .join("\n");
}

async function handleClick(event) {
  const button = event.target.closest("button");
  if (!button) {
    return;
  }

  if (button.dataset.action === "open-mobile-nav") {
    state = setMobileNavOpen(state, true);
    render();
    return;
  }

  if (button.dataset.action === "close-mobile-nav") {
    state = setMobileNavOpen(state, false);
    render();
    return;
  }

  if (button.dataset.action === "toggle-ctrl") {
    state = toggleCtrlModifier(state);
    render();
    return;
  }

  if (button.dataset.workspaceSelect) {
    state = selectWorkspace(state, button.dataset.workspaceSelect);
    render();
    await refreshSelectedWorkspaceResources();
    return;
  }

  if (button.dataset.sessionSelect) {
    state = selectSession(state, button.dataset.sessionSelect);
    state = setTerminalStreamStatus(state, "connecting");
    navigate(state.route.pathname);
    render();
    await ensureSelectedTerminalTransport(false, true);
    return;
  }

  if (button.dataset.sessionTerminate) {
    await apiDelete(`/sessions/${button.dataset.sessionTerminate}`);
    await refetchSessions({ replaceHistory: true });
    await ensureSelectedTerminalTransport(false, true);
    return;
  }

  if (button.dataset.worktreeSessionCreate) {
    if (!state.selectedWorkspaceId) {
      throw new Error("Select a workspace before starting a worktree session.");
    }

    const created = await apiPost("/sessions", {
      name: button.dataset.worktreeSessionName || undefined,
      workspace_id: state.selectedWorkspaceId,
      kind: "worktree",
      managed_worktree_id: button.dataset.worktreeSessionCreate
    });
    const sessions = await apiGet("/sessions");
    state = handleCreateSuccess(state, created.id, sessions);
    state = setTerminalStreamStatus(state, "connecting");
    navigate(sessionRoute(created.id));
    render();
    await ensureSelectedTerminalTransport(false, true);
    return;
  }

  if (button.dataset.terminalKey) {
    if (!state.selectedSessionId || !terminalSocket || terminalSocket.readyState !== WebSocket.OPEN) {
      return;
    }

    sendTerminalNamedKey(button.dataset.terminalKey, state.ctrlModifierLatched);
    state = clearCtrlModifier(state);
    render();
    return;
  }
}

async function handleSubmit(event) {
  const form = event.target.closest("form");
  if (!form) {
    return;
  }

  event.preventDefault();

  if (form.id === "register-workspace-form") {
    const formData = new FormData(form);
    const name = String(formData.get("workspace-name") || "").trim();
    const rootPath = String(formData.get("workspace-root-path") || "").trim();
    const created = await apiPost("/workspaces", {
      name: name || undefined,
      root_path: rootPath
    });
    const workspaces = await apiGet("/workspaces");
    state = selectWorkspace(applyWorkspaces(state, workspaces), created.id);
    render();
    await refreshSelectedWorkspaceResources();
    state = setNotice(state, `Workspace registered: ${created.name}`, "success");
    render();
    form.reset();
    return;
  }

  if (form.id === "create-local-session-form") {
    if (!state.selectedWorkspaceId) {
      throw new Error("Select a workspace before creating a local session.");
    }

    const formData = new FormData(form);
    const name = String(formData.get("session-name") || "").trim();
    const created = await apiPost("/sessions", {
      name: name || undefined,
      workspace_id: state.selectedWorkspaceId,
      kind: "local"
    });
    const sessions = await apiGet("/sessions");
    state = handleCreateSuccess(state, created.id, sessions);
    state = setTerminalStreamStatus(state, "connecting");
    navigate(sessionRoute(created.id));
    render();
    await ensureSelectedTerminalTransport(false, true);
    form.reset();
    return;
  }

  if (form.id === "create-worktree-form") {
    if (!state.selectedWorkspaceId) {
      throw new Error("Select a workspace before creating a managed worktree.");
    }

    const formData = new FormData(form);
    const sourceRef = String(formData.get("worktree-source-ref") || "").trim();
    const branchName = String(formData.get("worktree-branch-name") || "").trim();
    const created = await apiPost(
      `/workspaces/${state.selectedWorkspaceId}/worktrees`,
      {
        source_ref: sourceRef,
        branch_name: branchName
      }
    );
    await refreshSelectedWorkspaceResources();
    state = setNotice(
      state,
      `Managed worktree created: ${created.branch_name}`,
      "success"
    );
    render();
    form.reset();
    return;
  }

}

function handleInput(event) {
  if (event.target.id !== "terminal-mobile-input") {
    return;
  }

  const value = event.target.value;
  if (!value) {
    return;
  }

  sendTextInput(value, state.ctrlModifierLatched);
  state = clearCtrlModifier(state);
  event.target.value = "";
  render();
}

function handleKeyDown(event) {
  if (event.target.id !== "terminal-mobile-input") {
    return;
  }

  if (event.key === "Enter") {
    event.preventDefault();
    sendTerminalNamedKey("enter", state.ctrlModifierLatched);
    state = clearCtrlModifier(state);
    event.target.value = "";
    render();
  }
}

function handleDesktopKeyDown(event) {
  if (state.isMobileBrowser) {
    return;
  }
  if (!state.selectedSessionId || !terminalSocket || terminalSocket.readyState !== WebSocket.OPEN) {
    return;
  }
  if (isEditableTarget(event.target)) {
    return;
  }

  const key = event.key;
  if (event.ctrlKey && !event.metaKey && !event.altKey && key.length === 1) {
    const upper = key.toUpperCase();
    const code = upper.charCodeAt(0);
    if (code >= 0x40 && code <= 0x5f) {
      event.preventDefault();
      sendTerminalBytes(Uint8Array.from([code & 0x1f]));
      return;
    }
  }

  const special = {
    Enter: "\r",
    Tab: "\t",
    Escape: "\u001b",
    ArrowUp: "\u001b[A",
    ArrowDown: "\u001b[B",
    ArrowRight: "\u001b[C",
    ArrowLeft: "\u001b[D",
    Backspace: "\u007f"
  };
  if (special[key]) {
    event.preventDefault();
    sendTerminalBytes(textEncoder.encode(special[key]));
  }
}

function isEditableTarget(target) {
  if (!(target instanceof HTMLElement)) {
    return false;
  }
  const tag = target.tagName;
  return (
    tag === "INPUT" ||
    tag === "TEXTAREA" ||
    tag === "SELECT" ||
    target.isContentEditable
  );
}

async function handlePopState() {
  state = applySessions(
    {
      ...state,
      route: parseShellRoute(window.location.pathname),
      terminalSurface: null,
      terminalUnavailable: false,
      terminalLastSequence: null
    },
    state.sessions
  );
  state = setTerminalStreamStatus(
    state,
    state.selectedSessionId ? "connecting" : "idle"
  );
  render();
  await ensureSelectedTerminalTransport(false, true);
}

function handleResize() {
  fitAddon?.fit();
  sendResizeControl();
}

async function handleVisibilityChange() {
  const wasVisible = state.pageVisible;
  state = setPageVisibility(state, document.visibilityState !== "hidden");
  if (!state.pageVisible) {
    disconnectTerminalStream();
    render();
    return;
  }

  if (shouldResyncTerminalOnVisibilityRestore(wasVisible, state)) {
    state = setTerminalStreamStatus(state, "reconnecting");
    render();
    await ensureSelectedTerminalTransport(true, true);
    return;
  }

  render();
}

async function refetchWorkspaces() {
  const workspaces = await apiGet("/workspaces");
  state = applyWorkspaces(state, workspaces);
  render();
  await refreshSelectedWorkspaceResources();
}

async function refreshSelectedWorkspaceResources() {
  if (!state.selectedWorkspaceId) {
    state = applyWorkspaceResources(state, {
      managedWorktrees: [],
      sourceRefs: []
    });
    render();
    return;
  }

  const [managedWorktrees, sourceRefs] = await Promise.all([
    apiGet(`/workspaces/${state.selectedWorkspaceId}/worktrees`),
    apiGet(`/workspaces/${state.selectedWorkspaceId}/source-refs`)
  ]);
  state = applyWorkspaceResources(state, {
    managedWorktrees,
    sourceRefs
  });
  render();
}

async function refetchSessions({ replaceHistory = false } = {}) {
  const sessions = await apiGet("/sessions");
  const previousPath = state.route.pathname;
  state = applySessions(state, sessions);
  if (replaceHistory || previousPath !== state.route.pathname) {
    navigate(state.route.pathname, true);
  }
  render();
}

async function refreshTerminalSnapshot(sessionId, forceRender) {
  const requestId = ++terminalRequestId;
  try {
    const surface = await apiGet(`/sessions/${sessionId}/terminal`);
    if (requestId !== terminalRequestId || state.selectedSessionId !== sessionId) {
      return false;
    }

    const firstSurface = !state.terminalSurface || state.terminalUnavailable;
    state = setTerminalSurface(state, surface);
    if (forceRender || firstSurface) {
      render();
    } else {
      paintTerminal();
    }
    return true;
  } catch (error) {
    if (error.status !== 404) {
      throw error;
    }

    const selectedSessionId = sessionId;
    await refetchSessions({ replaceHistory: true });
    if (state.selectedSessionId === selectedSessionId && selectedSessionId) {
      state = setTerminalUnavailable(state, true);
      render();
    }
    disconnectTerminalStream();
    return false;
  }
}

function sendTerminalBytes(bytes) {
  if (!terminalSocket || terminalSocket.readyState !== WebSocket.OPEN) {
    return;
  }
  terminalSocket.send(bytes);
}

function sendTextInput(value, ctrlModifierLatched) {
  if (!value) {
    return;
  }

  if (ctrlModifierLatched && [...value].length === 1) {
    const ch = value.charCodeAt(0);
    if (ch >= 0x40 && ch <= 0x7f) {
      sendTerminalBytes(Uint8Array.from([ch & 0x1f]));
      return;
    }
  }

  sendTerminalBytes(textEncoder.encode(value));
}

function sendTerminalNamedKey(namedKey, ctrlModifierLatched) {
  const value = mapNamedKeyToBytes(namedKey, ctrlModifierLatched);
  if (!value) {
    return;
  }
  sendTerminalBytes(textEncoder.encode(value));
}

function mapNamedKeyToBytes(namedKey, ctrlModifierLatched) {
  const mapped = {
    escape: "\u001b",
    tab: "\t",
    enter: "\r",
    arrow_up: ctrlModifierLatched ? "\u001b[1;5A" : "\u001b[A",
    arrow_down: ctrlModifierLatched ? "\u001b[1;5B" : "\u001b[B",
    arrow_right: ctrlModifierLatched ? "\u001b[1;5C" : "\u001b[C",
    arrow_left: ctrlModifierLatched ? "\u001b[1;5D" : "\u001b[D"
  };
  return mapped[namedKey] || null;
}

function sendResizeControl() {
  if (!terminalSocket || terminalSocket.readyState !== WebSocket.OPEN || !terminal) {
    return;
  }

  terminalSocket.send(
    JSON.stringify({
      type: "resize",
      rows: terminal.rows,
      cols: terminal.cols
    })
  );
}

async function ensureSelectedTerminalTransport(reconnected, forceRender) {
  disconnectTerminalStream({ preserveStatus: true });

  if (!shouldConnectTerminalStream(state)) {
    state = setTerminalStreamStatus(
      state,
      state.selectedSessionId ? "disconnected" : "idle"
    );
    render();
    return;
  }

  state = setTerminalStreamStatus(state, reconnected ? "reconnecting" : "connecting");
  render();

  const sessionId = state.selectedSessionId;
  const snapshotLoaded = await refreshTerminalSnapshot(sessionId, forceRender);
  if (
    !snapshotLoaded ||
    state.selectedSessionId !== sessionId ||
    !shouldConnectTerminalStream(state)
  ) {
    return;
  }

  connectTerminalStream(sessionId);
}

async function reconcileSelectedTerminalTransport() {
  if (!shouldConnectTerminalStream(state)) {
    disconnectTerminalStream();
    state = setTerminalStreamStatus(
      state,
      state.selectedSessionId ? "disconnected" : "idle"
    );
    render();
    return;
  }

  if (terminalSocket && terminalSocketSessionId === state.selectedSessionId) {
    return;
  }

  await ensureSelectedTerminalTransport(false, true);
}

function connectTerminalStream(sessionId) {
  if (!sessionId) {
    return;
  }

  const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
  const nextSocket = new WebSocket(
    `${protocol}//${window.location.host}/sessions/${encodeURIComponent(sessionId)}/terminal/stream`
  );
  nextSocket.binaryType = "arraybuffer";

  terminalSocket = nextSocket;
  terminalSocketSessionId = sessionId;
  terminalStreamReady = false;
  terminalBufferedFrames = [];

  nextSocket.addEventListener("open", () => {
    if (terminalSocket !== nextSocket || state.selectedSessionId !== sessionId) {
      return;
    }

    state = setTerminalStreamStatus(state, "connected");
    terminalStreamReady = true;
    terminalLastOpenedAt = Date.now();
    terminalReconnectFailures = 0;
    sendResizeControl();
    render();

    const bufferedFrames = terminalBufferedFrames.splice(0);
    for (const payload of bufferedFrames) {
      queueTerminalPayload(nextSocket, sessionId, payload);
    }
  });

  nextSocket.addEventListener("message", (message) => {
    if (terminalSocket !== nextSocket) {
      return;
    }

    if (!terminalStreamReady) {
      terminalBufferedFrames.push(message.data);
      return;
    }

    queueTerminalPayload(nextSocket, sessionId, message.data);
  });

  nextSocket.addEventListener("close", () => {
    const expectedClose = terminalExpectedCloseSocket === nextSocket;
    if (expectedClose) {
      terminalExpectedCloseSocket = null;
    }

    if (terminalSocket === nextSocket) {
      terminalSocket = null;
      terminalSocketSessionId = null;
      terminalStreamReady = false;
      terminalBufferedFrames = [];
    }

    if (expectedClose) {
      return;
    }

    if (terminalSocket !== null) {
      return;
    }

    void runAction(async () => {
      await refetchSessions({ replaceHistory: true });
      if (state.selectedSessionId === sessionId && shouldConnectTerminalStream(state)) {
        const livedMs = terminalLastOpenedAt ? Date.now() - terminalLastOpenedAt : 0;
        if (livedMs > 0 && livedMs < 1500) {
          terminalReconnectFailures += 1;
        } else {
          terminalReconnectFailures = 1;
        }
        if (terminalReconnectFailures >= 3) {
          state = setTerminalStreamStatus(state, "disconnected");
          state = setNotice(
            state,
            "Terminal stream keeps closing immediately. Check daemon logs and refresh after restart.",
            "warning"
          );
          render();
          return;
        }
        state = setTerminalStreamStatus(state, "reconnecting");
        render();
        scheduleTerminalReconnect();
        return;
      }

      state = setTerminalStreamStatus(
        state,
        state.selectedSessionId ? "disconnected" : "idle"
      );
      render();
    });
  });

  nextSocket.addEventListener("error", () => {
    nextSocket.close();
  });
}

function queueTerminalPayload(nextSocket, sessionId, payload) {
  terminalFrameQueue = terminalFrameQueue
    .catch(() => undefined)
    .then(() => runAction(() => handleTerminalStreamPayload(nextSocket, sessionId, payload)));
}

async function handleTerminalStreamPayload(nextSocket, sessionId, payload) {
  if (terminalSocket !== nextSocket || state.selectedSessionId !== sessionId) {
    return;
  }

  if (!(payload instanceof ArrayBuffer)) {
    return;
  }

  terminal?.write(new Uint8Array(payload));
}

function disconnectTerminalStream({ preserveStatus = false } = {}) {
  if (terminalReconnectTimer) {
    window.clearTimeout(terminalReconnectTimer);
    terminalReconnectTimer = null;
  }

  terminalStreamReady = false;
  terminalBufferedFrames = [];
  terminalSocketSessionId = null;
  terminalLastOpenedAt = 0;

  if (terminalSocket) {
    const activeSocket = terminalSocket;
    terminalExpectedCloseSocket = activeSocket;
    terminalSocket = null;
    activeSocket.close();
  } else {
    terminalExpectedCloseSocket = null;
  }

  if (!preserveStatus) {
    state = setTerminalStreamStatus(
      state,
      state.selectedSessionId ? "disconnected" : "idle"
    );
  }
}

function connectSocket(reconnected) {
  if (socket) {
    socket.close();
  }

  state = setSocketStatus(state, reconnected ? "reconnecting" : "connecting");
  render();

  const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
  const nextSocket = new WebSocket(`${protocol}//${window.location.host}/ws/events`);
  socket = nextSocket;

  nextSocket.addEventListener("open", async () => {
    if (socket !== nextSocket) {
      return;
    }

    state = setSocketStatus(state, "connected");
    render();
    if (shouldRefetchSessionsOnSocketOpen(reconnected)) {
      await runAction(() => refetchSessions({ replaceHistory: true }));
    }
  });

  nextSocket.addEventListener("message", async (message) => {
    if (socket !== nextSocket) {
      return;
    }

    const payload = JSON.parse(message.data);
    if (eventRequiresSessionRefresh(payload)) {
      await runAction(async () => {
        const previousSelectedSessionId = state.selectedSessionId;
        await refetchSessions({ replaceHistory: true });
        if (state.selectedSessionId !== previousSelectedSessionId) {
          await ensureSelectedTerminalTransport(false, true);
        } else {
          await reconcileSelectedTerminalTransport();
        }
      });
    }
  });

  nextSocket.addEventListener("close", () => {
    if (socket !== nextSocket) {
      return;
    }

    state = setSocketStatus(state, "reconnecting");
    render();
    scheduleReconnect();
  });

  nextSocket.addEventListener("error", () => {
    nextSocket.close();
  });
}

function scheduleReconnect() {
  if (reconnectTimer) {
    window.clearTimeout(reconnectTimer);
  }

  reconnectTimer = window.setTimeout(() => {
    reconnectTimer = null;
    connectSocket(true);
  }, 1000);
}

function scheduleTerminalReconnect() {
  if (terminalReconnectTimer) {
    window.clearTimeout(terminalReconnectTimer);
  }

  terminalReconnectTimer = window.setTimeout(() => {
    terminalReconnectTimer = null;
    void runAction(() => ensureSelectedTerminalTransport(true, false));
  }, TERMINAL_STREAM_RECONNECT_DELAY_MS);
}

function navigate(pathname, replace = false) {
  if (window.location.pathname === pathname) {
    return;
  }

  if (replace) {
    window.history.replaceState({}, "", pathname);
  } else {
    window.history.pushState({}, "", pathname);
  }
}

async function apiGet(pathname) {
  return apiRequest(pathname, {
    method: "GET"
  });
}

async function apiPost(pathname, body) {
  return apiRequest(pathname, {
    method: "POST",
    headers: {
      "content-type": "application/json"
    },
    body: JSON.stringify(body)
  });
}

async function apiDelete(pathname) {
  return apiRequest(pathname, {
    method: "DELETE"
  });
}

async function apiRequest(pathname, options) {
  const response = await fetch(pathname, options);
  if (!response.ok) {
    let message = `${response.status} ${response.statusText}`;
    try {
      const payload = await response.json();
      if (payload?.error?.message) {
        message = payload.error.message;
      }
    } catch {
      // Preserve the default HTTP status text fallback.
    }
    const error = new Error(message);
    error.status = response.status;
    throw error;
  }

  if (response.status === 204) {
    return null;
  }

  return response.json();
}

function escapeHtml(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}
