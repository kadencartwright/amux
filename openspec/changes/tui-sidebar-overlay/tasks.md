## 1. Setup and Dependencies

- [x] 1.1 Add Bubble Tea and Lipgloss dependencies to go.mod
- [x] 1.2 Create new `cmd/amux-sidebar` directory for the TUI binary
- [x] 1.3 Create new `internal/tui` package directory for TUI components

## 2. TUI Core Implementation

- [x] 2.1 Create TUI model structure with Bubble Tea framework
- [x] 2.2 Implement initialization message and config loading
- [x] 2.3 Create view function to render project list with status symbols
- [x] 2.4 Implement ANSI color styling using Lipgloss
- [x] 2.5 Add header and legend rendering to the view

## 3. Status Management

- [x] 3.1 Implement status file reader for `~/.local/share/amux/status/*.json`
- [x] 3.2 Create status polling mechanism (2-second interval)
- [x] 3.3 Map status strings to symbols and colors
- [x] 3.4 Handle missing or malformed status files gracefully

## 4. Keyboard Handling

- [x] 4.1 Implement keymap for number keys 1-9 (project switching)
- [x] 4.2 Implement 'r' key for refresh functionality
- [x] 4.3 Implement 'q' and Escape keys for hiding sidebar
- [x] 4.4 Implement F12 key detection for toggle signal from tmux
- [x] 4.5 Test all keyboard interactions work correctly

## 5. Tmux Integration and Configuration

- [x] 5.1 Add `sidebar_toggle_key` field to Config struct with default value "S"
- [x] 5.2 Update config validation to accept single character toggle keys
- [x] 5.3 Implement tmux key binding setup for toggle key during session creation
- [x] 5.4 Implement tmux command executor for window switching
- [x] 5.5 Add error handling for failed tmux commands
- [x] 5.6 Create function to get current project list from config
- [x] 5.7 Test project switching works across different sessions

## 5. Tmux Integration

- [x] 5.1 Implement tmux command executor for window switching
- [x] 5.2 Add error handling for failed tmux commands
- [x] 5.3 Create function to get current project list from config
- [x] 5.4 Test project switching works across different sessions

## 6. Overlay and Pass-Through Mode

- [x] 6.1 Implement hidden/visible state in TUI model
- [x] 6.2 Create clear-screen function for hiding overlay
- [x] 6.3 Implement pass-through mode (minimal key handling)
- [x] 6.4 Ensure state persists when switching projects
- [x] 6.5 Test toggle behavior with configured hotkey works correctly

## 7. Session Integration

- [x] 7.1 Modify `internal/session` to launch TUI instead of shell in sidebar pane
- [x] 7.2 Remove old `send-keys` based sidebar implementation
- [x] 7.3 Ensure orchestrator session creates tmux binding for toggle key
- [x] 7.4 Test full integration: amux start → TUI sidebar appears

## 8. Testing and Validation

- [x] 8.1 Test sidebar displays correctly with multiple projects
- [x] 8.2 Verify status colors render properly (ANSI codes)
- [x] 8.3 Test keyboard shortcuts work (1-9, r, q, Esc)
- [x] 8.4 Test hide/show toggle with default key (Prefix+S)
- [x] 8.5 Test hide/show toggle with custom key from config
- [x] 8.6 Verify project switching works via tmux commands
- [x] 8.7 Test error handling when tmux commands fail
- [x] 8.8 Run full integration test: start, switch projects, stop

## 9. Cleanup and Documentation

- [x] 9.1 Remove old sidebar package if no longer needed
- [x] 9.2 Update README.md with new TUI sidebar information
- [x] 9.3 Document the configurable toggle key in README
- [x] 9.4 Add code comments for complex TUI logic
- [x] 9.5 Update go.mod and run go mod tidy
