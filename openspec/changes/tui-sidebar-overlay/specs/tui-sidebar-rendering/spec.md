## ADDED Requirements

### Requirement: Sidebar displays project list with status indicators
The TUI sidebar SHALL display a list of all configured projects with their current status represented by colored symbols.

#### Scenario: Projects are displayed with correct status symbols
- **WHEN** the sidebar TUI is running
- **THEN** it displays each project name with a status symbol:
  - ● (green) for running status
  - ○ (gray) for idle status
  - ◐ (yellow) for needs_review status
  - ✗ (red) for stopped status

### Requirement: Sidebar uses ANSI color codes
The TUI sidebar SHALL use ANSI escape codes for all color formatting, not tmux formatting syntax.

#### Scenario: Colors render correctly in terminal
- **WHEN** the sidebar renders project status
- **THEN** it uses ANSI color codes (e.g., \x1b[32m for green)
- **AND** colors display correctly in standard terminal emulators

### Requirement: Sidebar shows header and legend
The TUI sidebar SHALL display a header showing "AMUX" and "PROJECTS" sections, plus a status legend explaining each symbol.

#### Scenario: Header and legend are visible
- **WHEN** the sidebar is visible
- **THEN** it displays:
  - "AMUX" header at top
  - "PROJECTS" section label
  - Project list with status
  - "STATUS LEGEND" section with symbol explanations

### Requirement: Sidebar updates reflect status changes
The TUI sidebar SHALL poll for status file changes and update the display when project statuses change.

#### Scenario: Status changes are reflected in real-time
- **WHEN** a project's status file is updated
- **THEN** the sidebar updates the status symbol within 2 seconds
- **AND** the display redraws without flickering
