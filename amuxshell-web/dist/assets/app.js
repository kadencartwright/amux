import initRendererModule, {
  WasmCanvasRenderer
} from "./vendor/amuxterm_web.js";
import {
  acknowledgeInputFocus,
  applyTerminalStreamFrame,
  applySessions,
  applyWorkspaceResources,
  applyWorkspaces,
  buildNamedKeyRequest,
  buildTextInputRequest,
  clearCtrlModifier,
  clearNotice,
  createShellState,
  eventRequiresSessionRefresh,
  handleCreateSuccess,
  parseShellRoute,
  renderShell,
  selectSession,
  selectWorkspace,
  sessionRoute,
  setInputDraft,
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

let state = createShellState(window.location.pathname, document.visibilityState);
let renderer = null;
let rendererCanvas = null;
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

root.addEventListener("click", (event) => {
  void runAction(() => handleClick(event));
});
root.addEventListener("submit", (event) => {
  void runAction(() => handleSubmit(event));
});
root.addEventListener("input", handleInput);
window.addEventListener("popstate", () => {
  void runAction(handlePopState);
});
window.addEventListener("resize", handleResize);
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
  await initRendererModule(
    new URL("./vendor/amuxterm_web_bg.wasm", import.meta.url)
  );
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
  const canvas = root.querySelector("#terminal-canvas");
  if (!canvas || !state.terminalSurface || state.terminalUnavailable) {
    renderer = null;
    rendererCanvas = null;
    return;
  }

  if (!renderer || rendererCanvas !== canvas) {
    renderer = new WasmCanvasRenderer(canvas);
    rendererCanvas = canvas;
  }

  const rect = canvas.getBoundingClientRect();
  const width = rect.width || canvas.clientWidth || 720;
  const height = rect.height || canvas.clientHeight || 480;
  const orientation =
    window.innerWidth >= window.innerHeight ? "landscape" : "portrait";

  renderer.handle_viewport_change(
    width,
    height,
    window.devicePixelRatio || 1,
    orientation
  );
  renderer.render_surface_json(JSON.stringify(state.terminalSurface));
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
    if (!state.selectedSessionId) {
      return;
    }
    const payload = buildNamedKeyRequest(
      button.dataset.terminalKey,
      state.ctrlModifierLatched
    );
    state = clearCtrlModifier(state);
    render();
    await submitTerminalInput(payload);
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

  if (form.id === "terminal-input-form") {
    if (!state.selectedSessionId) {
      return;
    }

    const payload = buildTextInputRequest(
      state.inputDraft,
      state.ctrlModifierLatched,
      { appendEnter: true }
    );
    if (!payload.events.length) {
      return;
    }

    state = clearCtrlModifier(setInputDraft(state, ""));
    render();
    await submitTerminalInput(payload);
  }
}

function handleInput(event) {
  if (event.target.id !== "terminal-input") {
    return;
  }

  state = setInputDraft(state, event.target.value);
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
  paintTerminal();
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

async function submitTerminalInput(payload) {
  await apiPost(`/sessions/${state.selectedSessionId}/terminal/input`, payload);
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
    render();

    const bufferedFrames = terminalBufferedFrames.splice(0);
    for (const payload of bufferedFrames) {
      queueTerminalPayload(nextSocket, sessionId, payload);
    }
  });

  nextSocket.addEventListener("message", (message) => {
    if (terminalSocket !== nextSocket || typeof message.data !== "string") {
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

function queueTerminalPayload(nextSocket, sessionId, payloadText) {
  terminalFrameQueue = terminalFrameQueue
    .catch(() => undefined)
    .then(() => runAction(() => handleTerminalStreamPayload(nextSocket, sessionId, payloadText)));
}

async function handleTerminalStreamPayload(nextSocket, sessionId, payloadText) {
  if (terminalSocket !== nextSocket || state.selectedSessionId !== sessionId) {
    return;
  }

  const payload = JSON.parse(payloadText);
  if (payload.session_id !== sessionId) {
    return;
  }

  const result = applyTerminalStreamFrame(state, payload);
  if (result.needsResync) {
    await ensureSelectedTerminalTransport(true, true);
    return;
  }

  state = result.state;
  paintTerminal();
}

function disconnectTerminalStream({ preserveStatus = false } = {}) {
  if (terminalReconnectTimer) {
    window.clearTimeout(terminalReconnectTimer);
    terminalReconnectTimer = null;
  }

  terminalStreamReady = false;
  terminalBufferedFrames = [];
  terminalSocketSessionId = null;

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
