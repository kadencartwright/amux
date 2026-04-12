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

**Decision:** Backend streams raw PTY bytes over WebSocket, client parses with ghostty-web.

**Rationale:**
- Backend becomes a simple passthrough (no terminal state parsing)
- Client gets full escape sequence support from ghostty-web
- Simpler mental model: bytes in, bytes out

**Alternatives considered:**
- JSON snapshots/diffs (current): Requires backend parsing, more complex
- Structured binary protocol: Over-engineered for this use case

### 3. Single bidirectional WebSocket channel

**Decision:** Input, output, and resize all flow over the same WebSocket connection.

**Message format:**
```
Client → Server:
  - Raw keystrokes as UTF-8 text (e.g., "hello\n", "\x03" for Ctrl+C)
  - Resize: "SIZE:{rows}:{cols}" (e.g., "SIZE:24:80")

Server → Client:
  - Raw PTY bytes (terminal output)
```

**Rationale:**
- Lower latency (no HTTP handshake per keystroke)
- Simpler connection lifecycle
- Works well with ghostty-web's onData API

**Alternatives considered:**
- HTTP POST for input: Adds latency, more complex
- Separate WebSocket for input: Unnecessary complexity

### 4. Client sends initial resize on connect

**Decision:** After WebSocket connects, client immediately sends terminal dimensions.

**Rationale:**
- Allows client to request appropriate size based on viewport
- Server tracks dimensions per session for resize ioctl
- Supports responsive resize as browser window changes

### 5. Unified keyboard area for mobile, direct focus on desktop

**Decision:** Mobile shows modifier buttons + textarea; desktop terminal captures keyboard directly.

**Mobile layout:**
```
┌─────────────────────────────────────┐
│ [Ctrl] [Esc] [Tab] [↑][↓][←][→] [↵]│
├─────────────────────────────────────┤
│ Type here...                        │
└─────────────────────────────────────┘
```

**Rationale:**
- Mobile virtual keyboards don't fire keydown events reliably
- Textarea provides reliable input on mobile
- Modifier buttons prepend ctrl/alt to typed characters
- Desktop gets native keyboard experience via ghostty-web

## Risks / Trade-offs

| Risk | Mitigation |
|------|------------|
| ghostty-web mobile support gaps | ghostty-web has touch support; fallback to textarea for problematic keys |
| WebSocket binary data handling | Terminal output is UTF-8, safe for text frames |
| Connection drop during input | Auto-reconnect, client resends resize, tmux preserves shell state |
| Large terminal output flooding client | ghostty-web handles backpressure internally |

## Open Questions

- Confirm ghostty-web's mobile touch keyboard support meets requirements
- Verify resize format ("SIZE:rows:cols") doesn't conflict with valid terminal input
- Consider whether to keep `POST /sessions/{id}/terminal/input` for backwards compatibility
