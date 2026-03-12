## ADDED Requirements

### Requirement: Number key project switching
The TUI SHALL support switching to projects using number keys 1-9.

#### Scenario: Switch to project 1
- **WHEN** user presses "1" key
- **THEN** TUI switches terminal view to project 1's PTY
- **AND** project 1 is highlighted in sidebar
- **AND** terminal view shows project 1's output

#### Scenario: Invalid project number
- **WHEN** user presses "5" key
- **AND** there are only 3 projects configured
- **THEN** TUI ignores the keypress
- **AND** no change occurs

### Requirement: Arrow key navigation
The TUI SHALL support navigating the project list using arrow keys.

#### Scenario: Navigate down
- **WHEN** user presses Down arrow
- **THEN** sidebar selection moves to next project
- **AND** current selection is visually highlighted

#### Scenario: Navigate up
- **WHEN** user presses Up arrow
- **THEN** sidebar selection moves to previous project
- **AND** current selection is visually highlighted

### Requirement: Enter to activate
The TUI SHALL activate the selected project when user presses Enter.

#### Scenario: Activate with Enter
- **WHEN** user navigates to a project in sidebar
- **AND** user presses Enter
- **THEN** TUI switches terminal view to that project
- **AND** subsequent keystrokes go to that project's PTY

### Requirement: Quit command
The TUI SHALL exit when user presses 'q' while in sidebar navigation mode.

#### Scenario: Quit TUI
- **WHEN** user presses 'q' key in sidebar mode
- **THEN** TUI exits cleanly
- **AND** all PTYs are closed
- **AND** terminal is restored to normal state

### Requirement: Mode switching
The TUI SHALL support switching focus between sidebar and terminal view.

#### Scenario: Switch to sidebar mode
- **WHEN** user presses Ctrl+A
- **THEN** focus switches to sidebar navigation mode
- **AND** keystrokes are captured by TUI for navigation

#### Scenario: Switch to terminal mode
- **WHEN** user presses Enter on a project
- **THEN** focus switches to terminal input mode
- **AND** keystrokes are forwarded to PTY
