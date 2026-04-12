## Why

The current terminal experience requires users to type into a separate textarea field and click "Send + Enter" to interact with the terminal. This breaks the mental model of using a native terminal. Users expect to click on the terminal and type directly, with full keyboard support including Ctrl+C, copy/paste, and modifier keys working as expected.

Additionally, the current architecture requires maintaining a complex Rust WASM renderer (`amuxterm-web`) for terminal rendering, while a battle-tested solution exists: ghostty-web provides Ghostty's VT100 parser compiled to WASM, offering better ANSI support and requiring less maintenance.

## What Changes

- **Replace terminal renderer**: Remove `amuxterm-web` (Rust WASM) and move browser rendering to `ghostty-web`
- **Preserve deterministic bootstrap/resync**: Keep `GET /sessions/{session_id}/terminal` as the authoritative snapshot endpoint, but source it from tmux capture or equivalent backend capture rather than `vt100` state
- **Upgrade live transport**: Convert `GET /sessions/{session_id}/terminal/stream` into a bidirectional WebSocket for live terminal I/O
- **Use a safe wire protocol**: Stream raw PTY/input bytes in WebSocket binary frames and reserve WebSocket text frames for JSON control messages such as resize
- **Delete `amuxterm-web`**: Entire crate is replaced by `ghostty-web`
- **Improve input UX**: Desktop terminals capture keyboard input directly; mobile uses a unified modifier row plus hidden text input that forwards edits immediately

## Capabilities

### New Capabilities

- `ghostty-terminal-stream`: ghostty-web receives live PTY bytes over WebSocket, renders terminal output, and sends keyboard input back over the same connection
- `terminal-resize`: Terminal dimensions are sent as explicit WebSocket control messages to resize the PTY
- `mobile-terminal-input`: Mobile browsers get a unified keyboard area with modifier buttons and immediate input forwarding

### Modified Capabilities

- `terminal-stream-transport-v1`: Keep snapshot bootstrap/resync, but replace diff streaming with bidirectional raw-byte streaming
- `selected-session-shell-stream-v1`: Keep snapshot-first shell loading, but reconnect the selected session through the new bidirectional stream contract
- `terminal-web-surface-v1`: Replace the renderer baseline while retaining browser, reliability, and performance expectations

## Impact

- **amuxd**: Remove `terminal.rs` and the `vte`/`vt100` parsing stack, keep `GET /sessions/{session_id}/terminal` for bootstrap/resync capture, and convert `/terminal/stream` to a bidirectional binary/control WebSocket
- **amuxterm-web**: Delete entire crate
- **amuxshell-web**: Replace the WASM renderer with `ghostty-web`, reconnect the shell through snapshot + bidirectional stream, and implement unified mobile input
- **Dependencies**: Remove `vte`, `vt100`, `unicode-width`, and `unicode-segmentation` from `amuxd`; add `ghostty-web` to `amuxshell-web`
