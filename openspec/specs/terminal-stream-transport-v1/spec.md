## ADDED Requirements

### Requirement: Full terminal snapshot bootstrap and resync endpoint
The system SHALL expose `GET /sessions/{session_id}/terminal` as the authoritative full terminal state endpoint for the selected session, and that response SHALL include the current terminal surface plus scrollback needed for client bootstrap and full resync.

#### Scenario: Initial terminal bootstrap
- **WHEN** a client requests `GET /sessions/{session_id}/terminal` for an existing session before opening that session's terminal stream
- **THEN** the system returns the full terminal state for that session
- **AND** the returned state includes scrollback in addition to the currently visible rows

#### Scenario: Resync after incremental state loss
- **WHEN** a client needs to recover after reconnect, sequence-gap detection, or visibility restoration
- **THEN** the system uses `GET /sessions/{session_id}/terminal` as the full resync source for that session

### Requirement: Session-scoped read-only terminal stream
The system SHALL expose a dedicated read-only WebSocket endpoint at `GET /sessions/{session_id}/terminal/stream` for terminal output updates for one selected session.

#### Scenario: Connect selected-session stream
- **WHEN** a client connects to `GET /sessions/{session_id}/terminal/stream` for an existing session and the protocol upgrades to WebSocket
- **THEN** the system establishes a terminal update stream for that session only

#### Scenario: Reject unknown terminal stream session
- **WHEN** a client requests `GET /sessions/{session_id}/terminal/stream` for an unknown or unavailable session
- **THEN** the system does not establish a terminal stream for that session

### Requirement: Row-level diff frames with monotonic sequencing
The system SHALL emit terminal stream frames as row-level diffs ordered by a per-session monotonic sequence number.

#### Scenario: Diff frame advances sequence
- **WHEN** the system emits multiple diff frames for the same session stream connection
- **THEN** each emitted frame carries a sequence number greater than the previous emitted frame for that session connection

#### Scenario: Diff frame contains row-level updates
- **WHEN** terminal output changes are streamed for the selected session
- **THEN** each incremental frame carries row-level terminal changes rather than a mandatory full snapshot payload

### Requirement: Full resync on reconnect, sequence gap, or visibility restore
The system SHALL require clients to perform a full snapshot resync instead of incremental continuation whenever the selected-session stream reconnects, a sequence gap is detected, or browser visibility is restored after inactivity.

#### Scenario: Reconnect requires full reload
- **WHEN** a client reconnects `GET /sessions/{session_id}/terminal/stream` after interruption
- **THEN** the client must reacquire `GET /sessions/{session_id}/terminal` before treating new diff frames as authoritative

#### Scenario: Sequence gap requires full reload
- **WHEN** a client observes that the next diff frame sequence is not the expected immediate successor for that session stream
- **THEN** the client must discard its incremental terminal state and reacquire `GET /sessions/{session_id}/terminal`

#### Scenario: Visibility restore requires full reload
- **WHEN** the client restores browser visibility for a selected session after the terminal view was inactive
- **THEN** the client must reacquire `GET /sessions/{session_id}/terminal` before continuing incremental rendering

### Requirement: No resume-from-sequence in v1
The system SHALL not require or support resume-from-sequence negotiation in v1 terminal streaming.

#### Scenario: Reconnect without resume token
- **WHEN** a client reconnects the selected-session terminal stream in v1
- **THEN** the system does not depend on a last-seen sequence value to resume delivery

### Requirement: Backpressure favors fresher terminal state
The system SHALL tolerate slow consumers by coalescing newer unsent row diffs under backpressure instead of blocking the selected-session terminal transport behind stale incremental frames.

#### Scenario: Slow consumer receives newer coalesced diff
- **WHEN** terminal output changes faster than a connected client can consume pending diff frames
- **THEN** the system may replace older pending row diffs with newer row diffs for that session
- **AND** the terminal stream remains live without unbounded buffering of stale frames

### Requirement: Terminal input remains a separate API
The system SHALL keep terminal input outside the selected-session output stream and continue to accept input through the existing terminal input endpoint.

#### Scenario: Input is not sent over terminal stream
- **WHEN** a client needs to send terminal input for a selected session
- **THEN** the client uses `POST /sessions/{session_id}/terminal/input`
- **AND** the read-only terminal stream is not used as the input transport

### Requirement: Stream closes when selected session terminates
The system SHALL close the selected-session terminal stream when that session terminates or otherwise becomes unavailable.

#### Scenario: Session termination closes terminal stream
- **WHEN** the selected session is terminated while a client is connected to `GET /sessions/{session_id}/terminal/stream`
- **THEN** the system closes that session's terminal stream

### Requirement: Remote auth and exposure stay outside the daemon
The system SHALL keep terminal-stream authentication and public remote exposure concerns outside `amuxd`, behind a separate gateway or equivalent fronting layer.

#### Scenario: Terminal stream uses gateway-owned remote security boundary
- **WHEN** AMUX is exposed beyond the local daemon boundary
- **THEN** authentication and remote exposure controls are enforced outside `amuxd`
- **AND** the terminal stream capability does not require daemon-owned remote auth behavior in v1
