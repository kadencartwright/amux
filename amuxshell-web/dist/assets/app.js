import initRendererModule, {
  WasmCanvasRenderer
} from "./vendor/amuxterm_web.js";
import {
  POLL_INTERVAL_MS,
  acknowledgeInputFocus,
  applySessions,
  buildNamedKeyRequest,
  buildTextInputRequest,
  clearCtrlModifier,
  createShellState,
  eventRequiresSessionRefresh,
  handleCreateSuccess,
  parseShellRoute,
  renderShell,
  selectSession,
  sessionRoute,
  setInputDraft,
  setMobileNavOpen,
  setPageVisibility,
  setSocketStatus,
  setTerminalSurface,
  setTerminalUnavailable,
  shouldPollTerminal,
  shouldRefetchSessionsOnSocketOpen,
  toggleCtrlModifier
} from "./core.js";

const root = document.querySelector("#shell-root");

let state = createShellState(window.location.pathname, document.visibilityState);
let renderer = null;
let rendererCanvas = null;
let pollingTimer = null;
let socket = null;
let reconnectTimer = null;
let terminalRequestId = 0;

root.addEventListener("click", handleClick);
root.addEventListener("submit", handleSubmit);
root.addEventListener("input", handleInput);
window.addEventListener("popstate", handlePopState);
window.addEventListener("resize", handleResize);
document.addEventListener("visibilitychange", handleVisibilityChange);

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
  await refetchSessions({ replaceHistory: true });
  connectSocket(false);
  if (state.selectedSessionId) {
    await refreshTerminal(true);
  }
  updatePolling();
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

  if (button.dataset.sessionSelect) {
    state = selectSession(state, button.dataset.sessionSelect);
    navigate(state.route.pathname);
    render();
    await refreshTerminal(true);
    updatePolling();
    return;
  }

  if (button.dataset.sessionTerminate) {
    await apiDelete(`/sessions/${button.dataset.sessionTerminate}`);
    await refetchSessions({ replaceHistory: true });
    if (state.selectedSessionId) {
      await refreshTerminal(true);
    }
    updatePolling();
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

  if (form.id === "create-session-form") {
    const formData = new FormData(form);
    const name = String(formData.get("session-name") || "").trim();
    const created = await apiPost("/sessions", {
      name: name || undefined
    });
    const sessions = await apiGet("/sessions");
    state = handleCreateSuccess(state, created.id, sessions);
    navigate(sessionRoute(created.id));
    render();
    await refreshTerminal(true);
    updatePolling();
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

function handlePopState() {
  state = applySessions(
    {
      ...state,
      route: parseShellRoute(window.location.pathname),
      terminalSurface: null,
      terminalUnavailable: false
    },
    state.sessions
  );
  render();
  if (state.selectedSessionId) {
    refreshTerminal(true);
  }
  updatePolling();
}

function handleResize() {
  paintTerminal();
}

function handleVisibilityChange() {
  const wasVisible = state.pageVisible;
  state = setPageVisibility(state, document.visibilityState !== "hidden");
  updatePolling();
  if (!wasVisible && state.pageVisible && state.selectedSessionId) {
    refreshTerminal(false);
  }
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

async function refreshTerminal(forceRender) {
  if (!state.selectedSessionId) {
    return;
  }

  const requestId = ++terminalRequestId;
  try {
    const surface = await apiGet(`/sessions/${state.selectedSessionId}/terminal`);
    if (requestId !== terminalRequestId || !state.selectedSessionId) {
      return;
    }

    const firstSurface = !state.terminalSurface || state.terminalUnavailable;
    state = setTerminalSurface(state, surface);
    if (forceRender || firstSurface) {
      render();
    } else {
      paintTerminal();
    }
  } catch (error) {
    if (error.status !== 404) {
      console.error("terminal refresh failed", error);
      return;
    }

    const selectedSessionId = state.selectedSessionId;
    await refetchSessions({ replaceHistory: true });
    if (state.selectedSessionId === selectedSessionId && selectedSessionId) {
      state = setTerminalUnavailable(state, true);
      render();
    }
    updatePolling();
  }
}

async function submitTerminalInput(payload) {
  await apiPost(`/sessions/${state.selectedSessionId}/terminal/input`, payload);
  await refreshTerminal(false);
}

function updatePolling() {
  if (pollingTimer) {
    clearInterval(pollingTimer);
    pollingTimer = null;
  }

  if (!shouldPollTerminal(state)) {
    return;
  }

  pollingTimer = window.setInterval(() => {
    refreshTerminal(false);
  }, POLL_INTERVAL_MS);
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
      await refetchSessions({ replaceHistory: true });
    }
  });

  nextSocket.addEventListener("message", async (message) => {
    if (socket !== nextSocket) {
      return;
    }

    const payload = JSON.parse(message.data);
    if (eventRequiresSessionRefresh(payload)) {
      await refetchSessions({ replaceHistory: true });
      if (state.selectedSessionId) {
        await refreshTerminal(true);
      }
      updatePolling();
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
    const error = new Error(`${response.status} ${response.statusText}`);
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
