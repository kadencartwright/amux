## ADDED Requirements

### Requirement: Sidebar displays project list
The TUI SHALL render a sidebar showing all configured projects with their current status indicators.

#### Scenario: View project list
- **WHEN** the TUI is running
- **THEN** the sidebar displays all projects from config
- **AND** each project shows a status symbol (● ○ ◐ ✗)
- **AND** each project shows the project name
- **AND** each project shows the agent type

### Requirement: Sidebar has visual layout
The sidebar SHALL occupy a configurable portion of the terminal width with clear visual separation.

#### Scenario: Default sidebar layout
- **WHEN** TUI starts with default configuration
- **THEN** sidebar uses 25% of terminal width
- **AND** sidebar has a vertical border separating it from terminal view
- **AND** sidebar shows "AMUX" header at top

### Requirement: Terminal view displays active session
The TUI SHALL display the output from the currently active project's PTY in the main terminal view area.

#### Scenario: Display active project
- **WHEN** a project is selected as active
- **THEN** terminal view shows PTY output from that project's session
- **AND** terminal view updates in real-time as new output arrives

### Requirement: Status indicators in sidebar
The sidebar SHALL display color-coded status indicators for each project.

#### Scenario: Running project indicator
- **WHEN** a project has "running" status
- **THEN** sidebar displays green ● symbol next to project name

#### Scenario: Idle project indicator
- **WHEN** a project has "idle" status
- **THEN** sidebar displays gray ○ symbol next to project name

#### Scenario: Needs review indicator
- **WHEN** a project has "needs_review" status
- **THEN** sidebar displays yellow ◐ symbol next to project name

#### Scenario: Stopped project indicator
- **WHEN** a project has "stopped" status
- **THEN** sidebar displays red ✗ symbol next to project name
