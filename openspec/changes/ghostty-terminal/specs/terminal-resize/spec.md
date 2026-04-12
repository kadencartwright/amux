# terminal-resize Specification

## Purpose

Handle terminal dimension changes via WebSocket control messages.

## ADDED Requirements

### Requirement: Resize via WebSocket control message

The system SHALL accept terminal resize commands from the client over the same WebSocket connection.

#### Scenario: Client sends resize command

- **WHEN** the client sends a WebSocket text frame containing `{"type":"resize","rows":24,"cols":80}`
- **THEN** the system parses the dimensions from the message
- **AND** the system calls ioctl to resize the PTY window to the specified dimensions

#### Scenario: Invalid resize format ignored

- **WHEN** the client sends a WebSocket text frame that is not a valid resize control message
- **THEN** the system ignores or rejects that control frame
- **AND** terminal input bytes continue to flow only through WebSocket binary frames

### Requirement: Initial resize on connect

The system SHALL have the client send an initial resize command immediately after the terminal WebSocket connects.

#### Scenario: Client sends initial dimensions

- **WHEN** a client connects to the terminal WebSocket
- **THEN** the client immediately sends a resize control message describing the current terminal dimensions
- **AND** the server applies that resize before treating subsequent live rendering as authoritative for that connection

#### Scenario: Client sends dimensions before output

- **WHEN** the client sends a valid resize control message after connect or after a viewport change
- **THEN** the system resizes the PTY to those dimensions
- **AND** subsequent PTY output reflects the latest accepted dimensions

### Requirement: Daemon tracks session dimensions

The system SHALL maintain the current terminal dimensions for each active session.

#### Scenario: Resize updates stored dimensions

- **WHEN** the client sends a valid resize control message
- **THEN** the system updates the stored dimensions for that session
- **AND** subsequent resize messages replace the previous dimensions

### Requirement: Session-scoped resize policy is explicit

The system SHALL define resize semantics for multiple viewers of the same session.

#### Scenario: Latest valid resize wins for a session

- **WHEN** multiple clients are connected to the same session and more than one client sends a valid resize command
- **THEN** the latest valid resize command becomes the active PTY size for that session
- **AND** later valid resize commands replace earlier ones

### Requirement: Resize within session bounds

The system SHALL validate resize dimensions to reasonable bounds.

#### Scenario: Resize within acceptable range

- **WHEN** the client sends a resize control message with rows between 1 and 500 and cols between 1 and 500
- **THEN** the system accepts and applies the resize

#### Scenario: Resize outside bounds rejected

- **WHEN** the client sends dimensions outside acceptable range
- **THEN** the system ignores the resize command
- **OR** applies a clamped dimension
