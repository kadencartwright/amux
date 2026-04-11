## Why

The repo's current web shell baseline polls `GET /sessions/{session_id}/terminal` every 250 ms for the selected session, which is simple but too wasteful and too lossy for the next real browser transport slice. AMUX now needs a stream-first selected-session terminal contract that preserves a deterministic resync path, keeps terminal input separate, and fits the existing daemon and shell architecture without pulling auth or remote exposure into `amuxd`.

## What Changes

- Replace the selected-session polling baseline with a stream-first terminal transport centered on an initial terminal snapshot plus a dedicated read-only WebSocket stream for the selected session.
- Define `GET /sessions/{session_id}/terminal` as the authoritative full terminal snapshot endpoint, including scrollback, for initial load and all required resync cases.
- Define row-level diff stream frames with monotonic sequence numbers and explicit client behavior for reconnects, visibility restore, and detected sequence gaps.
- Keep v1 resync simple: reconnects and gaps trigger a full snapshot reload rather than resume-from-sequence support.
- Keep terminal input on the separate terminal input API and require the server to coalesce newer diffs under backpressure instead of blocking the selected-session stream.
- Specify shell-facing behavior for a minimal session overview, selected-session stream teardown on termination, and stable non-selected-state normalization after the selected session exits.
- Keep remote authentication and internet exposure out of `amuxd`; any such concerns remain the responsibility of a gateway in front of the daemon.

## Capabilities

### New Capabilities
- `terminal-stream-transport-v1`: Selected-session terminal snapshot and read-only WebSocket diff transport semantics, including sequencing, resync, backpressure, and termination behavior.
- `selected-session-shell-stream-v1`: Browser-shell behavior for consuming the selected-session stream, keeping session overview data minimal, and normalizing shell state when the selected session becomes unavailable.

### Modified Capabilities

## Impact

- Affects `amuxd` terminal routes and WebSocket transport design.
- Affects `amuxshell-web` selected-session data flow, replacing polling with snapshot-plus-stream behavior.
- Builds directly on the existing terminal snapshot model in `amuxd/src/terminal.rs`, the daemon router in `amuxd/src/lib.rs`, and the current shell transport flow in `amuxshell-web/src/app.js`.
- Preserves the existing separate lifecycle stream at `GET /ws/events` and the separate terminal input API at `POST /sessions/{session_id}/terminal/input`.
- Does not add daemon-owned auth, remote exposure, or resume-from-sequence state in v1.
