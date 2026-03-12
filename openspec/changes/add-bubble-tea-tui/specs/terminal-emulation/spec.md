## ADDED Requirements

### Requirement: PTY creation per project
The system SHALL create a PTY (pseudo-terminal) pair for each configured project when the project is first activated.

#### Scenario: Create PTY on first activation
- **WHEN** user switches to a project for the first time
- **THEN** system creates a new PTY pair
- **AND** system starts the configured agent in the PTY
- **AND** PTY is assigned to the project for the session lifetime

### Requirement: PTY output capture
The system SHALL capture stdout from each PTY and buffer it for display in the TUI.

#### Scenario: Capture agent output
- **WHEN** an agent writes to its PTY stdout
- **THEN** system captures the output
- **AND** if project is active, displays output in terminal view
- **AND** maintains scrollback buffer for the session

### Requirement: PTY input forwarding
The system SHALL forward user keystrokes from TUI to the active project's PTY stdin.

#### Scenario: Forward keyboard input
- **WHEN** user types while a project is active
- **AND** input is not a TUI navigation command
- **THEN** system forwards keystrokes to active PTY stdin
- **AND** agent receives input as if from real terminal

### Requirement: Terminal resize propagation
The system SHALL propagate terminal resize events to the active PTY.

#### Scenario: Resize terminal
- **WHEN** user resizes the terminal window
- **THEN** system updates TUI layout
- **AND** system sends resize signal (SIGWINCH) to active PTY
- **AND** agent process sees new terminal dimensions

### Requirement: PTY cleanup
The system SHALL properly cleanup PTY resources when shutting down.

#### Scenario: Cleanup on exit
- **WHEN** user exits the TUI
- **THEN** system closes all PTY file descriptors
- **AND** system signals agent processes to terminate gracefully
- **AND** agent processes continue running in background tmux sessions
