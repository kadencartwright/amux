## REMOVED Requirements

### Requirement: Full terminal snapshot bootstrap and resync endpoint

**Reason**: Replaced by ghostty-terminal-stream. Client bootstraps from initial PTY output after resize.

### Requirement: Session-scoped read-only terminal stream

**Reason**: Replaced by bidirectional WebSocket in ghostty-terminal-stream. Input and output use the same connection.

### Requirement: Row-level diff frames with monotonic sequencing

**Reason**: Replaced by raw PTY byte streaming. No JSON snapshots or diffs.

### Requirement: Full resync on reconnect, sequence gap, or visibility restore

**Reason**: Client reconnects and re-establishes dimensions. tmux preserves shell state server-side.

### Requirement: No resume-from-sequence in v1

**Reason**: No sequence numbers in raw byte streaming.

### Requirement: Backpressure favors fresher terminal state

**Reason**: Client (ghostty-web) handles backpressure internally.

### Requirement: Terminal input remains a separate API

**Reason**: Replaced by bidirectional WebSocket. Input flows over the same connection as output.

## ADDED Requirements

### Requirement: Raw PTY byte streaming

The system SHALL stream raw PTY bytes to the client over the WebSocket connection without serialization.

#### Scenario: Output streamed as raw bytes

- **WHEN** the PTY produces output for the session
- **THEN** the system sends the raw bytes as WebSocket TEXT messages
- **AND** no JSON or structured format wraps the data

### Requirement: Bidirectional WebSocket channel

The system SHALL use a single WebSocket connection for both terminal input and output.

#### Scenario: Input over same WebSocket

- **WHEN** the client sends terminal input over the WebSocket
- **THEN** the system forwards the raw bytes to the PTY
- **AND** no separate HTTP endpoint is required for input

### Requirement: Resize via WebSocket message

The system SHALL accept resize commands as WebSocket messages in the format `SIZE:{rows}:{cols}`.

#### Scenario: Resize message processed

- **WHEN** the client sends "SIZE:24:80"
- **THEN** the system resizes the PTY to 24 rows and 80 columns

### Requirement: Connection lifecycle

The system SHALL close the WebSocket when the session terminates.

#### Scenario: Session termination closes connection

- **WHEN** the session is terminated
- **THEN** the WebSocket connection is closed
