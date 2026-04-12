## Why

The current terminal experience requires users to type into a separate textarea field and click "Send + Enter" to interact with the terminal. This breaks the mental model of using a native terminal. Users expect to click on the terminal and type directly, with full keyboard support including Ctrl+C, copy/paste, and modifier keys working as expected.

Additionally, the current architecture requires maintaining a complex Rust WASM renderer (`amuxterm-web`) for terminal rendering, while a battle-tested solution exists: ghostty-web provides Ghostty's VT100 parser compiled to WASM, offering better ANSI support and requiring less maintenance.

## What Changes

- **Replace terminal renderer**: Remove `amuxterm-web` (Rust WASM) and `amuxshell-web`'s custom textarea input in favor of `ghostty-web`
- **Simplify backend**: Remove terminal state parsing from `amuxd` (vte, vt100, unicode-width, unicode-segmentation dependencies)
- **Bidirectional WebSocket**: Terminal I/O flows over a single WebSocket connection with raw PTY bytes
- **Delete `amuxterm-web`**: Entire crate is replaced by ghostty-web
- **Unified keyboard area on mobile**: Modifier buttons + input area for mobile, direct keyboard capture on desktop

## Capabilities

### New Capabilities

- `ghostty-terminal-stream`: Ghostty-web receives raw PTY bytes over WebSocket, renders terminal output, and sends keystrokes back over the same connection
- `terminal-resize`: Terminal dimensions sent over WebSocket control channel to resize the PTY
- `mobile-terminal-input`: Unified keyboard area with modifier buttons and text input for mobile browsers

### Modified Capabilities

- `terminal-stream-transport-v1`: Change transport from JSON snapshots/diffs to raw PTY bytes over WebSocket
- `terminal-web-surface-v1`: Deprecate - rendering handled entirely by ghostty-web client-side

## Impact

- **amuxd**: Remove terminal.rs, vte/vt100 parsing, simplify terminal streaming endpoint to raw byte passthrough
- **amuxterm-web**: Delete entire crate
- **amuxshell-web**: Replace WASM renderer with ghostty-web, implement unified keyboard area for mobile
- **Dependencies**: Remove vte, vt100, unicode-width, unicode-segmentation from amuxd; add ghostty-web to amuxshell-web
