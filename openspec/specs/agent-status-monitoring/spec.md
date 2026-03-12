# agent-status-monitoring Specification

## Purpose
TBD - created by archiving change agent-orchestrator. Update Purpose after archive.
## Requirements
### Requirement: Detect stopped sessions
The system SHALL detect when an agent session does not exist.

#### Scenario: Session not running
- **WHEN** checking status for a project
- **AND** tmux session "agent-<project>" does not exist
- **THEN** status SHALL be "stopped" (symbol: ✗)

### Requirement: Detect running agents
The system SHALL detect when an agent process is actively running.

#### Scenario: Process is running
- **WHEN** checking status for a project
- **AND** pane PID shows agent process in process list
- **THEN** status SHALL be "running" (symbol: ●)

### Requirement: Detect idle agents
The system SHALL detect when a session exists but agent is not running.

#### Scenario: Session exists, process exited
- **WHEN** checking status for a project
- **AND** tmux session exists
- **AND** no agent process found
- **THEN** status SHALL be "idle" (symbol: ○)

### Requirement: Detect needs-review via patterns
The system SHALL detect "needs review" status via output pattern matching for non-opencode agents.

#### Scenario: Pattern match for waiting input
- **WHEN** checking status for claude or codex project
- **AND** last 20 lines contain "waiting for your response" or similar
- **THEN** status SHALL be "needs-review" (symbol: ◐)

### Requirement: Periodic status refresh
The system SHALL update sidebar status indicators every 2 seconds.

#### Scenario: Status changes
- **WHEN** agent status changes from running to idle
- **THEN** within 2 seconds, sidebar SHALL reflect new status
- **AND** status symbol and color SHALL update

### Requirement: Status color coding
The system SHALL display status symbols with appropriate colors.

#### Scenario: Color display
- **WHEN** rendering sidebar
- **THEN** running status SHALL be green (colour46)
- **AND** idle status SHALL be gray (colour244)
- **AND** needs-review status SHALL be yellow (colour226)
- **AND** stopped status SHALL be red (colour196)

