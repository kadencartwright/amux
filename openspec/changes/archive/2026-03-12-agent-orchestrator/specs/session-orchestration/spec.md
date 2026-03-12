## ADDED Requirements

### Requirement: Orchestrator session creation
The system SHALL create a tmux session named "amux" with a sidebar pane and workspace pane.

#### Scenario: Initialize orchestrator
- **WHEN** user runs `amux start`
- **THEN** system creates tmux session "amux" with two panes
- **AND** the left pane (sidebar) has width from config
- **AND** the right pane (workspace) is active by default

### Requirement: Sidebar displays project list
The system SHALL render a sidebar showing all configured projects with their status symbols.

#### Scenario: View sidebar
- **WHEN** orchestrator is running
- **THEN** sidebar displays "AMUX" header
- **AND** lists all projects with status symbol and name
- **AND** shows agent type below each project name

### Requirement: Project switching via keystrokes
The system SHALL support switching to projects using number keys 1-9.

#### Scenario: Switch to project 1
- **WHEN** user presses "1" in orchestrator session
- **THEN** system switches workspace pane to project 1's agent window
- **AND** sidebar remains visible

### Requirement: Agent session linking
The system SHALL link agent sessions into the orchestrator workspace using tmux link-window.

#### Scenario: Link agent window
- **WHEN** user switches to a project
- **THEN** system links the agent session's window to orchestrator workspace
- **AND** both sessions see the same content (shared view)

### Requirement: Static configuration loading
The system SHALL load project configuration from `~/.config/amux/config.yaml`.

#### Scenario: Load config
- **WHEN** orchestrator starts
- **THEN** system reads config file
- **AND** creates agent sessions for each configured project
- **AND** each session starts in the specified path with specified agent
