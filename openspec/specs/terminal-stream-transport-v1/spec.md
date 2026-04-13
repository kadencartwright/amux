## MODIFIED Requirements

### Requirement: Full terminal snapshot bootstrap and resync endpoint

The system SHALL continue to expose `GET /sessions/{session_id}/terminal` as the authoritative full terminal state endpoint for bootstrap and resync, even after live rendering moves to `ghostty-web`.

#### Scenario: Initial terminal bootstrap

- **WHEN** a client requests `GET /sessions/{session_id}/terminal` for an existing session before opening that session's terminal stream
- **THEN** the system returns the current terminal state for that session
- **AND** the returned state includes the visible surface plus scrollback needed for client bootstrap

#### Scenario: Resync after reconnect or inactivity

- **WHEN** a client needs to recover after reconnect or browser visibility restoration
- **THEN** the system uses `GET /sessions/{session_id}/terminal` as the full resync source for that session

### Requirement: Session-scoped terminal stream

The system SHALL expose a bidirectional WebSocket endpoint at `GET /sessions/{session_id}/terminal/stream` for live terminal input and output for one selected session.

#### Scenario: Connect selected-session stream

- **WHEN** a client connects to `GET /sessions/{session_id}/terminal/stream` for an existing session and the protocol upgrades to WebSocket
- **THEN** the system establishes a live terminal stream for that session only

#### Scenario: Reject unknown terminal stream session

- **WHEN** a client requests `GET /sessions/{session_id}/terminal/stream` for an unknown or unavailable session
- **THEN** the system does not establish a terminal stream for that session

### Requirement: Raw byte stream uses binary frames

The system SHALL send PTY output in WebSocket binary frames and SHALL accept terminal input bytes in WebSocket binary frames.

#### Scenario: Output streamed as raw bytes

- **WHEN** the PTY produces output for the session
- **THEN** the system sends the raw bytes as WebSocket binary frames
- **AND** no JSON or UTF-8 text conversion wraps the data

#### Scenario: Input uses same WebSocket

- **WHEN** the client sends terminal input over the WebSocket
- **THEN** the system forwards the raw bytes to the PTY
- **AND** no separate HTTP endpoint is required for the primary input path

### Requirement: Control messages stay separate from terminal bytes

The system SHALL reserve WebSocket text frames for explicit control messages such as resize.

#### Scenario: Resize uses structured control frame

- **WHEN** the client needs to resize the terminal
- **THEN** the client sends a WebSocket text frame containing a structured control message
- **AND** terminal byte traffic remains in binary frames only

### Requirement: Full resync on reconnect or visibility restore

The system SHALL require clients to perform a full snapshot resync instead of incremental continuation whenever the selected-session stream reconnects or browser visibility is restored after inactivity.

#### Scenario: Reconnect requires full reload

- **WHEN** a client reconnects `GET /sessions/{session_id}/terminal/stream` after interruption
- **THEN** the client must reacquire `GET /sessions/{session_id}/terminal` before treating new live bytes as authoritative

#### Scenario: Visibility restore requires full reload

- **WHEN** the client restores browser visibility for a selected session after the terminal view was inactive
- **THEN** the client must reacquire `GET /sessions/{session_id}/terminal` before continuing live rendering

### Requirement: Backpressure favors bounded buffering and recoverable reconnect

The system SHALL tolerate slow consumers with bounded buffering and forced reconnect rather than unbounded accumulation of stale terminal bytes.

#### Scenario: Slow consumer exceeds byte budget

- **WHEN** terminal output changes faster than a connected client can consume pending bytes
- **THEN** the system may close that client's terminal stream once a bounded byte budget is exceeded
- **AND** the client recovers by refetching `GET /sessions/{session_id}/terminal` and reconnecting the live stream

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

## REMOVED Requirements

### Requirement: Row-level diff frames with monotonic sequencing

**Reason**: Replaced by raw PTY byte streaming; the stream no longer carries row-diff payloads or sequence numbers.

### Requirement: No resume-from-sequence in v1

**Reason**: Resume-from-sequence is not applicable once live updates are transmitted as a raw byte stream.

### Requirement: Terminal input remains a separate API

**Reason**: Primary terminal input now flows over the same bidirectional WebSocket as live terminal output.
