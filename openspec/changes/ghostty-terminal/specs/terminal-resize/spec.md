# terminal-resize Specification

## Purpose

Handle terminal dimension changes via WebSocket control messages.

## ADDED Requirements

### Requirement: Resize via WebSocket control message

The system SHALL accept terminal resize commands from the client over the same WebSocket connection.

#### Scenario: Client sends resize command

- **WHEN** the client sends a WebSocket message matching the pattern `SIZE:{rows}:{cols}` (e.g., "SIZE:24:80")
- **THEN** the system parses the dimensions from the message
- **AND** the system calls ioctl to resize the PTY window to the specified dimensions

#### Scenario: Invalid resize format ignored

- **WHEN** the client sends a message that does not match `SIZE:{rows}:{cols}`
- **THEN** the system treats it as terminal input (not a resize command)

### Requirement: Initial resize on connect

The system SHALL wait for an initial resize message from the client before streaming terminal output.

#### Scenario: Client sends initial dimensions

- **WHEN** a client connects to the terminal WebSocket
- **THEN** the system waits for the client to send a `SIZE` message
- **AND** the system does not send terminal output until dimensions are established

#### Scenario: Client sends dimensions before output

- **WHEN** the client sends a `SIZE` message
- **THEN** the system resizes the PTY to those dimensions
- **AND** begins streaming PTY output from that point

### Requirement: Daemon tracks session dimensions

The system SHALL maintain the current terminal dimensions for each active session.

#### Scenario: Resize updates stored dimensions

- **WHEN** the client sends a valid `SIZE` message
- **THEN** the system updates the stored dimensions for that session
- **AND** subsequent resize messages replace the previous dimensions

### Requirement: Resize within session bounds

The system SHALL validate resize dimensions to reasonable bounds.

#### Scenario: Resize within acceptable range

- **WHEN** the client sends `SIZE:{rows}:{cols}` with rows between 1 and 500 and cols between 1 and 500
- **THEN** the system accepts and applies the resize

#### Scenario: Resize outside bounds rejected

- **WHEN** the client sends dimensions outside acceptable range
- **THEN** the system ignores the resize command
- **OR** applies a clamped dimension
