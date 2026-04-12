## Context

The current terminal implementation uses a custom Rust WASM renderer (`amuxterm-web`) to parse terminal state and render to canvas. Input flows through a separate HTTP POST endpoint and a visible textarea in the UI. This architecture:

- Requires maintaining complex terminal parsing code in Rust
- Provides a poor UX (separate input field instead of direct terminal focus)
- Has incomplete feature support (cursor rendering not implemented in WASM renderer)

The architecture uses `vte` for escape sequence parsing and `vt100` for terminal state management, both in `amuxd`. Terminal output is streamed as JSON snapshots and row-level diffs.

## Goals / Non-Goals

**Goals:**
- Enable direct keyboard interaction with the terminal (focus terminal, type, works like a native terminal)
- Use battle-tested terminal emulation (Ghostty's parser) instead of custom Rust code
- Simplify backend by removing terminal parsing (pass through raw PTY bytes)
- Preserve a deterministic bootstrap and recovery path for first load, reconnect, and visibility restore
- Support full keyboard shortcuts (Ctrl+C, Ctrl+D, copy/paste) on desktop
- Support mobile terminal interaction with modifier buttons and unified keyboard area

**Non-Goals:**
- Scrollback persistence across reconnects (tmux handles scrollback server-side)
- Browser-specific workarounds beyond what ghostty-web provides
- Supporting terminals without WebSocket support

## Decisions

### 1. Use ghostty-web instead of xterm.js or custom renderer

**Decision:** Adopt `ghostty-web` as the terminal frontend.

**Rationale:**
- API-compatible with xterm.js (drop-in replacement)
- Uses Ghostty's battle-tested VT100 parser (same code as native app)
- Supports XTPUSHSGR/XTPOPSGR (xterm.js lacks these)
- ~400KB bundle, zero runtime dependencies
- Created by Coder for Mux, actively maintained

**Alternatives considered:**
- xterm.js: Battle-tested but reimplements terminal emulation in JS, lacks some escape sequence support
- Custom Rust WASM: Already exists (amuxterm-web), but incomplete and requires maintenance

### 2. Raw PTY bytes over WebSocket

**Decision:** Backend streams raw PTY bytes over WebSocket for live updates, but the daemon keeps `GET /sessions/{session_id}/terminal` as the authoritative snapshot/bootstrap endpoint.

**Rationale:**
- Backend becomes a simple passthrough (no terminal state parsing)
- Client gets full escape sequence support from ghostty-web
- Snapshot bootstrap avoids blank or stale terminals after reload/reconnect into quiet sessions
- Simpler mental model for live transport: bytes in, bytes out

**Alternatives considered:**
- Live-only byte stream with no snapshot: Cannot reliably bootstrap or recover a quiet terminal
- JSON snapshots/diffs (current): Requires backend parsing, more complex
- Structured binary protocol: Over-engineered for this use case

### 3. Bidirectional WebSocket on the existing stream path

**Decision:** Keep `GET /sessions/{session_id}/terminal` for snapshots and change `GET /sessions/{session_id}/terminal/stream` into a bidirectional WebSocket for live terminal I/O.

**Message format:**
```
Client → Server:
  - Binary frames: raw terminal input bytes
  - Text frames: JSON control messages, e.g. {"type":"resize","rows":24,"cols":80}

Server → Client:
  - Binary frames: raw PTY bytes
```

**Rationale:**
- Lower latency (no HTTP handshake per keystroke)
- Keeps control messages separate from terminal input bytes
- Avoids invalid UTF-8 assumptions for PTY output
- Preserves the existing shell route split between bootstrap (`/terminal`) and live updates (`/terminal/stream`)

**Alternatives considered:**
- HTTP POST for input: Adds latency, more complex
- Magic-string control messages inside raw input: Conflicts with literal terminal input
- Separate WebSocket for input: Unnecessary complexity

### 4. Client sends initial resize on connect

**Decision:** After the WebSocket connects, the client immediately sends a JSON resize control message and resends resize on viewport changes.

**Rationale:**
- Allows client to request appropriate size based on viewport
- Server tracks dimensions per session for resize ioctl
- Supports responsive resize as browser window changes

### 5. Bounded backpressure with reconnect + snapshot recovery

**Decision:** The daemon maintains bounded pending output per connected terminal stream. When a client falls behind the byte budget, the daemon closes that stream and the shell performs snapshot-based recovery before reconnecting.

**Rationale:**
- Raw byte streams cannot safely coalesce stale bytes the way diff frames could
- Bounded buffering avoids unbounded memory growth in `amuxd`
- Snapshot recovery keeps slow-consumer failure recoverable

### 6. Unified keyboard area for mobile, direct focus on desktop

**Decision:** Mobile shows modifier buttons plus a hidden or visually minimal text input that forwards edits immediately; desktop terminals capture keyboard directly through ghostty-web.

**Mobile layout:**
```
┌─────────────────────────────────────┐
│ [Ctrl] [Esc] [Tab] [↑][↓][←][→] [↵]│
├─────────────────────────────────────┤
│ Tap to type…                        │
└─────────────────────────────────────┘
```

**Rationale:**
- Mobile virtual keyboards don't fire keydown events reliably
- Hidden/minimal input provides reliable access to the virtual keyboard
- Immediate forwarding preserves terminal-style interaction for raw-mode and full-screen apps
- Modifier buttons transform the next typed character or emit special keys directly
- Desktop gets native keyboard experience via ghostty-web

### 7. Session-scoped resize semantics are explicit

**Decision:** Terminal size remains session-scoped, and the latest valid resize command becomes the active PTY size for that session.

**Rationale:**
- A tmux-backed PTY has one active size per session
- Making last-writer-wins explicit avoids undefined behavior for multiple viewers
- Multi-viewer coordination beyond this policy remains out of scope for this slice

## Risks / Trade-offs

| Risk | Mitigation |
|------|------------|
| ghostty-web mobile support gaps | Use a hidden/minimal mobile input surface for virtual keyboard reliability |
| Binary frame handling across browser/server | Use WebSocket binary frames for terminal bytes and JSON text frames only for control |
| Connection drop during input | Shell refetches snapshot, reconnects stream, and resends resize |
| Large terminal output flooding client | Daemon uses bounded buffering and forces recoverable reconnect when exceeded |
| Multiple viewers issue conflicting resizes | Specify session-scoped last-valid-resize-wins behavior |

## Open Questions

- Measure whether tmux capture alone is sufficient for snapshot bootstrap fidelity once `vt100` is removed
- Decide whether the legacy `POST /sessions/{id}/terminal/input` endpoint should be temporarily retained behind a migration flag
