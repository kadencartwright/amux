## ADDED Requirements

### Requirement: Sidebar toggle key is configurable
The TUI sidebar hotkey SHALL be configurable via the `sidebar_toggle_key` configuration option, defaulting to "A" (uppercase A).

#### Scenario: Default toggle key works
- **GIVEN** the user has not configured a custom toggle key
- **WHEN** amux creates the orchestrator session
- **THEN** tmux binds `Prefix + A` to toggle the sidebar

#### Scenario: Custom toggle key is respected
- **GIVEN** the user has set `sidebar_toggle_key: "B"` in config.yaml
- **WHEN** amux creates the orchestrator session
- **THEN** tmux binds `Prefix + B` to toggle the sidebar
- **AND** the default `Prefix + A` is NOT bound

### Requirement: Sidebar can be hidden with 'q' or 'Esc' key
The TUI sidebar SHALL support hiding the overlay when the user presses 'q' or Escape key.

#### Scenario: User hides the sidebar
- **WHEN** the user presses 'q' or Escape
- **THEN** the sidebar clears its display
- **AND** the TUI enters pass-through mode

### Requirement: Hidden sidebar can be shown via tmux hotkey
The TUI sidebar SHALL support showing the overlay again when the user presses the tmux prefix hotkey (e.g., `Prefix + A`) while in pass-through mode.

#### Scenario: User shows the sidebar via hotkey
- **GIVEN** the sidebar is in pass-through mode (hidden)
- **WHEN** the user presses the configured tmux hotkey (e.g., `Ctrl+A A`)
- **THEN** tmux sends a toggle command to the sidebar pane
- **AND** the sidebar redraws its content
- **AND** the TUI exits pass-through mode

### Requirement: Pass-through mode allows workspace interaction
When in pass-through mode, the TUI SHALL NOT intercept keys intended for the workspace pane. All keystrokes except the tmux prefix hotkey shall pass through to the workspace.

#### Scenario: Keys pass through to workspace
- **GIVEN** the sidebar is in pass-through mode and the user is focused on the workspace pane
- **WHEN** the user types alphanumeric keys, commands, or text
- **THEN** all keystrokes go to the active agent session in the workspace
- **AND** the sidebar remains hidden and does not intercept any input

#### Scenario: Only tmux hotkey wakes sidebar
- **GIVEN** the sidebar is in pass-through mode
- **WHEN** the user presses the tmux prefix hotkey (e.g., `Ctrl+A A`)
- **THEN** the sidebar receives the toggle command and wakes up
- **AND** typing regular characters like 'a', 'b', '1' does NOT wake the sidebar

### Requirement: Toggle state persists during session
The TUI sidebar SHALL remember whether it is hidden or visible throughout the tmux session.

#### Scenario: Toggle state is maintained
- **GIVEN** the user hides the sidebar
- **WHEN** they switch to a different project
- **THEN** the sidebar remains hidden
- **AND** does not redraw until explicitly shown
