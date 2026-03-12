## MODIFIED Requirements

### Requirement: Orchestrator session creation
The system SHALL create a Bubble Tea TUI application with a sidebar and terminal view.

#### Scenario: Initialize orchestrator
- **WHEN** user runs `amux start`
- **THEN** system launches Bubble Tea TUI application
- **AND** TUI displays sidebar with project list
- **AND** TUI displays terminal view area
- **AND** terminal view is active by default

### Requirement: Sidebar displays project list
The system SHALL render a sidebar in the TUI showing all configured projects with their status symbols.

#### Scenario: View sidebar
- **WHEN** TUI is running
- **THEN** sidebar displays "AMUX" header
- **AND** lists all projects with status symbol and name
- **AND** shows agent type below each project name
- **AND** renders using Bubble Tea components (not shell commands)

### Requirement: Project switching via keystrokes
The system SHALL support switching to projects using number keys 1-9 in the TUI.

#### Scenario: Switch to project 1
- **WHEN** user presses "1" in TUI
- **THEN** TUI switches terminal view to project 1's PTY
- **AND** sidebar highlights project 1 as active

### Requirement: Agent session management
The system SHALL manage agent processes using PTY pairs per project.

#### Scenario: Start agent session
- **WHEN** user switches to a project
- **THEN** system creates PTY pair if not exists
- **AND** starts agent process in PTY
- **AND** PTY output is captured for terminal view

### Requirement: Static configuration loading
The system SHALL load project configuration from `~/.config/amux/config.yaml`.

#### Scenario: Load config
- **WHEN** TUI starts
- **THEN** system reads config file
- **AND** prepares PTY sessions for each configured project
- **AND** each session starts in the specified path with specified agent

## REMOVED Requirements

### Requirement: Tmux-based window linking
**Reason:** TUI manages sessions directly via PTY, no longer uses tmux panes for UI
**Migration:** Agent sessions still use tmux in background for persistence, but TUI replaces tmux-based UI
