# ghostty-terminal-stream Specification

## Purpose

WebSocket-based terminal I/O using ghostty-web on the client and raw PTY bytes on the backend.

## ADDED Requirements

### Requirement: Bidirectional terminal WebSocket endpoint

The system SHALL expose a bidirectional WebSocket endpoint at `GET /sessions/{session_id}/terminal/stream` for live terminal I/O for one selected session.

#### Scenario: Connect to terminal session

- **WHEN** a client connects to `GET /sessions/{session_id}/terminal/stream` and the WebSocket protocol upgrades
- **THEN** the system establishes a bidirectional terminal connection for that session
- **AND** the client receives live PTY output over that connection
- **AND** the client can send terminal input and control messages over that connection

#### Scenario: Reject unknown session

- **WHEN** a client requests `GET /sessions/{session_id}/terminal/stream` for an unknown or unavailable session
- **THEN** the system closes the WebSocket connection without establishing a terminal session

### Requirement: Binary terminal data frames

The system SHALL use WebSocket binary frames for raw terminal bytes in both directions.

#### Scenario: Terminal output streamed as raw bytes

- **WHEN** the PTY produces output for the session
- **THEN** the system sends the raw bytes directly to the client in a WebSocket binary frame
- **AND** no JSON wrapping or UTF-8 text conversion is applied to the terminal data

#### Scenario: Keystroke input forwarded to PTY

- **WHEN** the client sends a WebSocket binary frame containing terminal input bytes (for example UTF-8 encoded text, escape sequences, or control bytes)
- **THEN** the system writes the raw bytes to the PTY
- **AND** no JSON wrapping or transformation is applied

#### Scenario: Escape sequences handled by client

- **WHEN** the client sends escape sequences (for example `\x1b[A` for arrow up) in a binary frame
- **THEN** the system forwards them to the PTY unchanged
- **AND** the client (ghostty-web) is responsible for interpreting keyboard events and generating appropriate escape sequences

### Requirement: Text frames are reserved for control messages

The system SHALL reserve WebSocket text frames for explicit control messages and SHALL NOT overload terminal input with magic strings.

#### Scenario: Resize uses an explicit control frame

- **WHEN** the client needs to resize the terminal
- **THEN** the client sends a WebSocket text frame containing a JSON control message
- **AND** terminal input bytes continue to use binary frames unchanged

### Requirement: WebSocket closes on session termination

The system SHALL close the terminal WebSocket when the session terminates or becomes unavailable.

#### Scenario: Session termination closes WebSocket

- **WHEN** the selected session is terminated while a client is connected to `GET /sessions/{session_id}/terminal/stream`
- **THEN** the system closes that session's WebSocket connection

### Requirement: Authentication stays outside daemon

The system SHALL keep terminal WebSocket authentication outside `amuxd`, behind a gateway or equivalent fronting layer.

#### Scenario: Terminal stream uses gateway-owned security

- **WHEN** AMUX is exposed beyond the local daemon boundary
- **THEN** authentication is enforced outside `amuxd`
- **AND** `amuxd` does not implement authentication logic for terminal streams
