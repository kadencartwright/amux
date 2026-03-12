## ADDED Requirements

### Requirement: Number keys switch to corresponding project
The TUI sidebar SHALL execute tmux commands to switch windows when the user presses number keys 1-9.

#### Scenario: User switches to project by number
- **GIVEN** there are 3 projects configured (project-a, project-b, project-c)
- **WHEN** the user presses '2' in the sidebar
- **THEN** the TUI executes: tmux select-window -t amux-orchestrator:project-b
- **AND** the main workspace switches to show project-b's agent

### Requirement: 'r' key refreshes the sidebar
The TUI sidebar SHALL refresh the project list and status display when the user presses 'r'.

#### Scenario: User refreshes the sidebar
- **WHEN** the user presses 'r'
- **THEN** the TUI reloads the configuration
- **AND** re-reads all status files
- **AND** redraws the sidebar with updated information

### Requirement: Sidebar communicates only via tmux commands
The TUI sidebar SHALL NOT use direct process communication, sockets, or signals to interact with tmux; only tmux CLI commands are permitted.

#### Scenario: All interactions use tmux CLI
- **WHEN** the sidebar needs to switch projects
- **THEN** it executes 'tmux select-window' command
- **AND** does not use any other IPC mechanism

### Requirement: Sidebar handles tmux command failures gracefully
The TUI sidebar SHALL handle failures from tmux commands without crashing and display appropriate error messages.

#### Scenario: Tmux command fails
- **GIVEN** a tmux command returns an error
- **WHEN** the sidebar attempts to switch to a project
- **THEN** the TUI displays an error message in the sidebar
- **AND** continues running without crashing
