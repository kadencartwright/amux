# ghostty-terminal-stream Specification

## Purpose

WebSocket-based terminal I/O using ghostty-web on the client and raw PTY bytes on the backend.

## ADDED Requirements

### Requirement: Bidirectional terminal WebSocket endpoint

The system SHALL expose a WebSocket endpoint at `GET /sessions/{session_id}/terminal` that handles both terminal output and input over a single bidirectional connection.

#### Scenario: Connect to terminal session

- **WHEN** a client connects to `GET /sessions/{session_id}/terminal` and the WebSocket protocol upgrades
- **THEN** the system establishes a bidirectional terminal connection for that session
- **AND** the client receives raw PTY output bytes as WebSocket messages
- **AND** the client can send terminal input as WebSocket messages

#### Scenario: Reject unknown session

- **WHEN** a client requests `GET /sessions/{session_id}/terminal` for an unknown or unavailable session
- **THEN** the system closes the WebSocket connection without establishing a terminal session

### Requirement: Raw PTY bytes as terminal output

The system SHALL send raw PTY bytes to the client as WebSocket TEXT messages without transformation.

#### Scenario: Terminal output streamed as raw bytes

- **WHEN** the PTY produces output for the session
- **THEN** the system sends the raw bytes directly to the client over the WebSocket
- **AND** no JSON wrapping or serialization is applied to the terminal data

### Requirement: Raw keystrokes as terminal input

The system SHALL accept raw keystroke data from the client as WebSocket TEXT messages and forward them to the PTY.

#### Scenario: Keystroke input forwarded to PTY

- **WHEN** the client sends a WebSocket message containing keystroke data (e.g., "hello\n", "\x03")
- **THEN** the system writes the raw bytes to the PTY
- **AND** no JSON wrapping or transformation is applied

#### Scenario: Escape sequences handled by client

- **WHEN** the client sends escape sequences (e.g., "\x1b[A" for arrow up)
- **THEN** the system forwards them to the PTY unchanged
- **AND** the client (ghostty-web) is responsible for interpreting keyboard events and generating appropriate escape sequences

### Requirement: WebSocket closes on session termination

The system SHALL close the terminal WebSocket when the session terminates or becomes unavailable.

#### Scenario: Session termination closes WebSocket

- **WHEN** the selected session is terminated while a client is connected to `GET /sessions/{session_id}/terminal`
- **THEN** the system closes that session's WebSocket connection

### Requirement: Authentication stays outside daemon

The system SHALL keep terminal WebSocket authentication outside `amuxd`, behind a gateway or equivalent fronting layer.

#### Scenario: Terminal stream uses gateway-owned security

- **WHEN** AMUX is exposed beyond the local daemon boundary
- **THEN** authentication is enforced outside `amuxd`
- **AND** `amuxd` does not implement authentication logic for terminal streams
