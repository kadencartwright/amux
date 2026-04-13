## MODIFIED Requirements

### Requirement: Selected-session shell uses snapshot plus dedicated stream

The system SHALL have the browser shell load a selected session by fetching `GET /sessions/{session_id}/terminal` and then consuming `GET /sessions/{session_id}/terminal/stream` as a bidirectional live terminal connection, instead of relying on repeating terminal polling.

#### Scenario: Session selection bootstraps snapshot then stream

- **WHEN** a user selects a session in the shell
- **THEN** the shell fetches `GET /sessions/{session_id}/terminal` for that session
- **AND** the shell opens `GET /sessions/{session_id}/terminal/stream` for live terminal updates and terminal input

### Requirement: Shell performs full resync for transport recovery cases

The system SHALL have the shell perform a full selected-session snapshot reload on stream reconnect, forced slow-consumer reconnect, or browser visibility restoration before applying further live bytes.

#### Scenario: Reconnect triggers selected-session resync

- **WHEN** the selected-session terminal stream reconnects after interruption
- **THEN** the shell refetches `GET /sessions/{session_id}/terminal` before accepting new live bytes as authoritative

#### Scenario: Slow-consumer reconnect triggers selected-session resync

- **WHEN** the daemon closes the selected-session terminal stream because that client exceeded the bounded pending-byte budget
- **THEN** the shell refetches `GET /sessions/{session_id}/terminal` before reconnecting the live stream

#### Scenario: Visibility restore triggers selected-session resync

- **WHEN** the shell restores visibility for a selected session after the page was hidden or inactive
- **THEN** the shell refetches `GET /sessions/{session_id}/terminal` before continuing selected-session streaming
