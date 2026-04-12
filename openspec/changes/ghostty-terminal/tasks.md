## 1. Backend Changes (amuxd)

- [ ] 1.1 Remove vte, vt100, unicode-width, unicode-segmentation from Cargo.toml
- [ ] 1.2 Remove terminal.rs module from amuxd
- [ ] 1.3 Simplify terminal streaming endpoint to raw byte passthrough
- [ ] 1.4 Add resize handling via ioctl on PTY
- [ ] 1.5 Parse SIZE messages from WebSocket
- [ ] 1.6 Store terminal dimensions per session
- [ ] 1.7 Remove TerminalSnapshot and related types from lib.rs
- [ ] 1.8 Update WebSocket handler for bidirectional communication
- [ ] 1.9 Remove GET /sessions/{id}/terminal endpoint (no longer needed)
- [ ] 1.10 Keep POST /sessions/{id}/terminal/input for backwards compatibility (optional)
- [ ] 1.11 Update tests for new streaming behavior

## 2. Frontend Changes (amuxshell-web)

- [ ] 2.1 Add ghostty-web to package.json dependencies
- [ ] 2.2 Replace WASM renderer import with ghostty-web
- [ ] 2.3 Update terminal initialization to use ghostty-web Terminal
- [ ] 2.4 Wire WebSocket messages to ghostty-web write()
- [ ] 2.5 Wire ghostty-web onData to WebSocket send
- [ ] 2.6 Send initial SIZE message on WebSocket connect
- [ ] 2.7 Handle resize events from browser window changes
- [ ] 2.8 Remove custom textarea terminal input
- [ ] 2.9 Remove mobile modifier button rendering (will be replaced)

## 3. Mobile Input Implementation

- [ ] 3.1 Detect mobile browsers (iOS Safari, Android Chrome)
- [ ] 3.2 Render unified keyboard area on mobile
- [ ] 3.3 Implement modifier button row (Ctrl, Esc, Tab, arrows, Enter)
- [ ] 3.4 Implement modifier latching logic
- [ ] 3.5 Add hidden textarea for mobile virtual keyboard
- [ ] 3.6 Wire mobile input to WebSocket
- [ ] 3.7 Style unified keyboard area appropriately
- [ ] 3.8 Handle orientation changes

## 4. Delete amuxterm-web

- [ ] 4.1 Remove amuxterm-web directory
- [ ] 4.2 Update any references to amuxterm-web in build scripts
- [ ] 4.3 Update documentation referencing amuxterm-web

## 5. Cleanup and Testing

- [ ] 5.1 Update README with new architecture
- [ ] 5.2 Remove any archived specs now obsolete
- [ ] 5.3 Test terminal connect/disconnect flow
- [ ] 5.4 Test resize functionality
- [ ] 5.5 Test keyboard input on desktop
- [ ] 5.6 Test mobile modifier buttons
- [ ] 5.7 Test copy/paste on desktop
- [ ] 5.8 Test reconnection behavior
- [ ] 5.9 Verify no regressions in existing session management
