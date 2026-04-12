## ADDED Requirements

### Requirement: Selected-session shell uses snapshot plus dedicated stream
The system SHALL have the browser shell load a selected session by fetching `GET /sessions/{session_id}/terminal` and then consuming `GET /sessions/{session_id}/terminal/stream` for incremental updates, instead of relying on repeating terminal polling.

#### Scenario: Session selection bootstraps snapshot then stream
- **WHEN** a user selects a session in the shell
- **THEN** the shell fetches `GET /sessions/{session_id}/terminal` for that session
- **AND** the shell opens `GET /sessions/{session_id}/terminal/stream` for subsequent terminal updates

#### Scenario: No selected session has no terminal stream
- **WHEN** the shell has no selected session
- **THEN** the shell does not keep a selected-session terminal stream open

### Requirement: Selected-session overview remains minimal
The system SHALL keep the shell's selected-session overview minimal, showing the session name and whether the selected-session terminal stream is currently connected.

#### Scenario: Selected-session overview shows minimal fields
- **WHEN** a selected session is rendered in the shell
- **THEN** the shell overview shows that session's name
- **AND** the shell overview shows whether the selected-session terminal stream is connected

### Requirement: Shell performs full resync for transport recovery cases
The system SHALL have the shell perform a full selected-session snapshot reload on stream reconnect, detected sequence gap, or browser visibility restoration before applying further diff frames.

#### Scenario: Reconnect triggers selected-session resync
- **WHEN** the selected-session terminal stream reconnects after interruption
- **THEN** the shell refetches `GET /sessions/{session_id}/terminal` before accepting new diff frames as authoritative

#### Scenario: Sequence gap triggers selected-session resync
- **WHEN** the shell detects that an incoming selected-session diff frame skipped the expected next sequence number
- **THEN** the shell discards the incremental terminal state and refetches `GET /sessions/{session_id}/terminal`

#### Scenario: Visibility restore triggers selected-session resync
- **WHEN** the shell restores visibility for a selected session after the page was hidden or inactive
- **THEN** the shell refetches `GET /sessions/{session_id}/terminal` before continuing selected-session streaming

### Requirement: Selected-session termination returns shell to stable non-selected state
The system SHALL close the selected-session terminal stream and normalize the shell back to a stable non-selected state when the selected session terminates or becomes unavailable.

#### Scenario: Selected session termination deselects shell state
- **WHEN** the currently selected session terminates while the shell is displaying it
- **THEN** the shell closes that session's terminal stream
- **AND** the shell clears the selected terminal view and returns to a stable non-selected state

#### Scenario: Selected session unavailable route normalizes
- **WHEN** the shell reloads or refreshes a selected-session route for a session that no longer exists
- **THEN** the shell normalizes away from that selected-session route
- **AND** the shell remains usable in a non-selected state
