## 1. Backend Changes (amuxd)

- [x] 1.1 Remove vte, vt100, unicode-width, unicode-segmentation from Cargo.toml
- [x] 1.2 Remove terminal.rs module from amuxd
- [x] 1.3 Keep `GET /sessions/{id}/terminal` as the authoritative bootstrap/resync endpoint using tmux capture or equivalent session capture
- [x] 1.4 Convert `GET /sessions/{id}/terminal/stream` to a bidirectional WebSocket
- [x] 1.5 Stream PTY output as WebSocket binary frames
- [x] 1.6 Accept terminal input as WebSocket binary frames and forward bytes to the PTY unchanged
- [x] 1.7 Accept JSON resize control messages over WebSocket text frames and apply PTY resize via ioctl
- [x] 1.8 Validate and store terminal dimensions per session with explicit last-valid-resize-wins semantics
- [x] 1.9 Add bounded output buffering/backpressure handling that closes slow streams for client resync
- [x] 1.10 Decide whether to retain `POST /sessions/{id}/terminal/input` behind a migration flag during rollout
- [x] 1.11 Update tests for snapshot bootstrap, reconnect recovery, binary transport, resize bounds, slow-consumer disconnect, and session termination

## 2. Frontend Changes (amuxshell-web)

- [x] 2.1 Add ghostty-web to package.json dependencies
- [x] 2.2 Replace WASM renderer import with ghostty-web
- [x] 2.3 Update terminal initialization to use ghostty-web Terminal
- [x] 2.4 Keep snapshot bootstrap on `GET /sessions/{id}/terminal` before opening the live stream
- [x] 2.5 Reconnect live transport over `GET /sessions/{id}/terminal/stream`
- [x] 2.6 Configure the WebSocket for binary PTY frames and write them into ghostty-web
- [x] 2.7 Encode ghostty-web `onData` output into binary frames and send it over the WebSocket
- [x] 2.8 Send an initial JSON resize control message on stream connect and resend resize on viewport changes
- [x] 2.9 Resync from the snapshot endpoint on reconnect, visibility restore, or forced slow-consumer reconnect
- [x] 2.10 Remove the desktop textarea-based terminal input path

## 3. Mobile Input Implementation

- [x] 3.1 Detect mobile browsers (iOS Safari, Android Chrome)
- [x] 3.2 Render unified keyboard area on mobile
- [x] 3.3 Implement modifier button row (Ctrl, Esc, Tab, arrows, Enter)
- [x] 3.4 Implement modifier latching logic
- [x] 3.5 Add hidden or visually minimal text input for the mobile virtual keyboard
- [x] 3.6 Forward mobile edits and paste events immediately to the terminal WebSocket without submit buffering
- [x] 3.7 Emit special keys directly from the modifier row and clear latched modifiers after use
- [x] 3.8 Style the unified keyboard area appropriately
- [x] 3.9 Handle orientation changes and preserve coherent terminal/mobile input state

## 4. Delete amuxterm-web

- [x] 4.1 Remove amuxterm-web directory
- [x] 4.2 Update any references to amuxterm-web in build scripts
- [x] 4.3 Update documentation referencing amuxterm-web

## 5. Cleanup and Testing

- [x] 5.1 Update README with new architecture
- [x] 5.2 Update affected OpenSpec deltas (`terminal-stream-transport-v1`, `selected-session-shell-stream-v1`, `terminal-web-surface-v1`)
- [x] 5.3 Test bootstrap into an existing quiet terminal session
- [ ] 5.4 Test resize functionality, including the last-valid-resize-wins session policy
- [ ] 5.5 Test keyboard input and copy/paste on desktop
- [ ] 5.6 Test mobile modifier buttons, immediate input forwarding, paste, and orientation changes
- [x] 5.7 Test reconnection and visibility-restore resync behavior
- [ ] 5.8 Test slow-consumer disconnect and snapshot-based recovery
- [ ] 5.9 Verify no regressions in existing session management
