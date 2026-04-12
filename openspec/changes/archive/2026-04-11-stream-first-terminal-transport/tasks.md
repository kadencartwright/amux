## 1. Daemon Terminal Transport

- [x] 1.1 Extend `amuxd` terminal state handling so `GET /sessions/{session_id}/terminal` returns the authoritative full snapshot needed for bootstrap and resync, including scrollback.
- [x] 1.2 Add a dedicated read-only WebSocket route for `GET /sessions/{session_id}/terminal/stream` and validate session existence before establishing the stream.
- [x] 1.3 Implement row-level terminal diff frame generation with monotonic per-stream sequence numbers.
- [x] 1.4 Add slow-consumer handling that coalesces newer pending row diffs under backpressure without unbounded buffering.
- [x] 1.5 Close the selected-session terminal stream when the session terminates or becomes unavailable, while keeping terminal input on `POST /sessions/{session_id}/terminal/input`.

## 2. Shell Stream Integration

- [x] 2.1 Replace selected-session terminal polling in `amuxshell-web` with snapshot bootstrap plus a dedicated selected-session WebSocket stream.
- [x] 2.2 Add shell-side sequence tracking and force a full snapshot resync on reconnect, sequence gap, and visibility restoration.
- [x] 2.3 Keep the selected-session overview minimal by rendering only the session name and connected-state indicator for the terminal stream.
- [x] 2.4 Normalize the shell to a stable non-selected state when the selected session terminates or becomes unavailable, and ensure the stream is torn down cleanly.

## 3. Verification And Documentation

- [x] 3.1 Add or update daemon tests for snapshot bootstrap, stream connection, monotonic sequencing, backpressure coalescing, and termination-driven stream closure.
- [x] 3.2 Add or update shell tests for snapshot-plus-stream bootstrap, reconnect/gap/visibility resync, and selected-session deselection behavior.
- [x] 3.3 Update manual verification guidance to cover the stream-first selected-session flow and confirm that auth and remote exposure remain gateway-owned concerns outside `amuxd`.
