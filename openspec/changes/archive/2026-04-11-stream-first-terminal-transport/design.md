## Context

The current repo already has the main pieces this change needs:

- `amuxd/src/lib.rs` exposes `GET /sessions/{session_id}/terminal` and `POST /sessions/{session_id}/terminal/input`.
- `amuxd/src/terminal.rs` already builds full `TerminalSnapshot` values from `vt100`, including configurable scrollback in the state core.
- `amuxshell-web/src/app.js` currently refreshes the selected terminal by polling `GET /sessions/{session_id}/terminal` every 250 ms and uses `GET /ws/events` only for lifecycle invalidation.

That polling baseline proved the shell loop, but it is now the wrong transport shape for the selected terminal view. The next slice should keep the existing full-snapshot endpoint, add a dedicated selected-session stream, and define a deterministic resync story that does not require resume tokens or server-side replay state in v1.

## Goals / Non-Goals

**Goals:**
- Replace selected-session terminal polling with a stream-first transport grounded in the current daemon and shell architecture.
- Keep `GET /sessions/{session_id}/terminal` as the authoritative full-state snapshot used for initial load and all resync cases.
- Define a dedicated read-only WebSocket stream for one selected terminal session at a time.
- Define row-level diff frame semantics with monotonic sequence numbers and explicit gap handling.
- Keep lifecycle events, terminal input, and auth/exposure boundaries separate from the terminal transport.
- Define server behavior under slow-consumer backpressure without requiring unbounded buffering.

**Non-Goals:**
- Resume-from-sequence or replay-window support in v1.
- Multiplexing multiple terminal sessions over one WebSocket.
- Moving terminal input onto the terminal stream.
- Replacing the existing lifecycle WebSocket with terminal frames.
- Adding daemon-owned auth, public internet exposure, or gateway behavior.

## Decisions

1. **Keep the snapshot endpoint as the resync authority**
   - Decision: `GET /sessions/{session_id}/terminal` remains the authoritative full terminal state endpoint and must include the initial visible terminal plus scrollback.
   - Rationale: the endpoint already exists, the terminal core already produces full snapshots, and using one HTTP snapshot path for both first load and recovery keeps v1 deterministic.
   - Alternatives considered:
      - Send the initial full snapshot over the WebSocket: rejected because reconnect, visibility restore, and gap recovery would still need a full-state authority.
      - Add resume tokens plus replay windows now: rejected because it adds transport state and recovery complexity before the stream contract is proven.

2. **Use one dedicated read-only WebSocket per selected terminal session**
   - Decision: expose a selected-session stream endpoint under the session terminal namespace, with the browser opening it only for the currently selected session.
   - Rationale: this matches the existing single-selected-session shell model and avoids mixing terminal frames into the generic lifecycle stream.
   - Alternatives considered:
      - Reuse `GET /ws/events` for terminal output: rejected because lifecycle invalidation and terminal rendering have different rate, payload, and backpressure characteristics.
      - Multiplex all session terminals over one socket: rejected because the current product flow renders one selected session and does not need that complexity yet.

3. **Stream row-level diffs with monotonic sequence numbers**
   - Decision: the server emits row-level diff frames ordered by a per-stream monotonic `sequence`, with each frame carrying only the rows and metadata needed to advance the client from the last known snapshot.
   - Rationale: `vt100` snapshots are already row-oriented enough for this contract, and row-level diffs are a good first step that reduces transport size without forcing cell-level patch machinery.
   - Alternatives considered:
      - Full-snapshot streaming only: rejected because it keeps the waste profile too close to polling.
      - Cell-level patch frames: rejected because it increases protocol and renderer complexity for limited v1 gain.

4. **Resync by full reload, not replay**
   - Decision: the client performs a full snapshot reload whenever it reconnects the stream, detects a sequence gap, or restores page visibility after the terminal was inactive.
   - Rationale: this makes recovery rules simple and predictable while avoiding replay buffers and resume negotiation.
   - Alternatives considered:
      - Best-effort continue after reconnect without resync: rejected because it risks silent divergence.
      - Resume from last seen sequence: rejected in v1 to keep the daemon stateless between WebSocket connections beyond the current live session stream.

5. **Backpressure drops stale intermediate diffs in favor of fresher state**
   - Decision: when the client cannot keep up, the server may coalesce pending diffs by replacing older unsent row updates with newer row updates for the same session, preserving monotonic sequence order for frames that are actually sent.
   - Rationale: the selected terminal view benefits more from freshness than from guaranteed delivery of every intermediate render step.
   - Alternatives considered:
      - Block session capture until the socket drains: rejected because a slow browser should not stall the selected session transport path.
      - Buffer all diffs indefinitely: rejected due to unbounded memory risk.

6. **Keep terminal input and remote security boundaries unchanged**
   - Decision: terminal input remains on `POST /sessions/{session_id}/terminal/input`, and any remote auth or public exposure remains outside `amuxd` behind a gateway.
   - Rationale: this keeps the terminal stream read-only, preserves the current API boundary, and avoids expanding the daemon's trust surface in this transport change.
   - Alternatives considered:
      - Bidirectional terminal stream socket: rejected because it couples input and output evolution and complicates browser recovery behavior.
      - Add auth/exposure behavior now: rejected because the repo's current direction keeps those concerns out of the daemon baseline.

7. **Selected-session termination must collapse the stream cleanly**
   - Decision: if the selected session terminates, the daemon closes the session stream and the shell normalizes to a stable non-selected state rather than trying to keep a dead terminal surface mounted.
   - Rationale: this matches the route-based selected-session model and avoids stale terminal state after session removal.
   - Alternatives considered:
      - Keep the last terminal frame visible after termination: rejected because it creates an ambiguous dead-but-selected state.

## Risks / Trade-offs

- [Row-level diffs may still be larger than ideal during heavy repaint] -> Start with row-level frames and keep the snapshot endpoint as the recovery mechanism if coalescing becomes aggressive.
- [Coalescing skips intermediate visual states] -> Prefer freshness and require deterministic resync on any detected divergence.
- [Visibility-based resync adds extra snapshot fetches] -> Limit them to visibility restoration and other explicit recovery cases rather than periodic polling.
- [Separate lifecycle and terminal sockets increase client coordination] -> Keep lifecycle WebSocket semantics unchanged and make the selected-session stream fully route-driven.
- [No resume-from-sequence means reconnects cost a full snapshot] -> Accept this in v1 to avoid premature replay infrastructure.

## Migration Plan

1. Define the new terminal transport capability for snapshot, stream, sequencing, recovery, backpressure, termination, and gateway boundary rules.
2. Define the shell capability for selected-session snapshot-plus-stream consumption, minimal overview rendering, and stable deselection behavior.
3. Implement the dedicated terminal stream route in `amuxd` alongside the existing snapshot and input routes.
4. Replace selected-session polling in `amuxshell-web` with snapshot bootstrap plus selected-session WebSocket handling.
5. Keep `GET /ws/events` for lifecycle invalidation and list refreshes.
6. Validate initial load, reconnect, visibility restore, sequence-gap recovery, slow-consumer behavior, and selected-session termination.

Rollback strategy:
- If the stream path is unstable, disable the selected-session stream and temporarily fall back to the existing snapshot path while preserving the terminal snapshot and input APIs.

## Open Questions

- Should the terminal stream endpoint use `GET /sessions/{session_id}/terminal/stream` or another session-scoped path, as long as it stays distinct from `GET /ws/events`?
- Do we want the server to emit an explicit terminal-end frame before closing on termination, or is close-plus-lifecycle invalidation sufficient for v1?
