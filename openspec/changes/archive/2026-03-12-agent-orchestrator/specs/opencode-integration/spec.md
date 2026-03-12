## ADDED Requirements

### Requirement: Opencode writes status to file
The opencode agent SHALL write status updates to `~/.local/share/amux/status/<project>.json`.

#### Scenario: Agent starts working
- **WHEN** opencode begins processing a user request
- **THEN** opencode SHALL write `{"status": "running", "timestamp": "<iso8601>"}` to status file

#### Scenario: Agent needs review
- **WHEN** opencode completes a task and waits for user input
- **THEN** opencode SHALL write `{"status": "needs_review", "timestamp": "<iso8601>"}` to status file

#### Scenario: Agent goes idle
- **WHEN** opencode finishes and returns to prompt
- **THEN** opencode SHALL write `{"status": "idle", "timestamp": "<iso8601>"}` to status file

### Requirement: Orchestrator reads opencode status
The orchestrator SHALL read status from file for opencode projects.

#### Scenario: Check opencode status
- **WHEN** checking status for an opencode project
- **THEN** system SHALL read `~/.local/share/amux/status/<project>.json`
- **AND** use the status field as the project's status
- **AND** fall back to process detection if file missing

### Requirement: Status file format
The status file SHALL contain valid JSON with status and timestamp fields.

#### Scenario: Valid status file
- **WHEN** reading status file
- **THEN** system SHALL parse JSON
- **AND** extract "status" field (running, idle, or needs_review)
- **AND** extract "timestamp" field (ISO8601 format)

### Requirement: Status file creation
The orchestrator SHALL ensure status directory exists for opencode projects.

#### Scenario: Initialize status directory
- **WHEN** creating an opencode agent session
- **THEN** system SHALL create `~/.local/share/amux/status/` directory if not exists
- **AND** set appropriate permissions (0700)

### Requirement: Status staleness detection
The orchestrator SHALL detect stale status files.

#### Scenario: Stale status
- **WHEN** reading status file
- **AND** timestamp is older than 30 seconds
- **THEN** system SHALL ignore file and use process detection
- **AND** log warning about stale status
