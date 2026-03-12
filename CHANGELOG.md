# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [2.0.0] - 2026-03-12

### Breaking Changes

#### Complete architectural change from tmux-based UI to Bubble Tea TUI

**Changed:**
- `amux start` now launches a Bubble Tea TUI instead of attaching to a tmux session
- UI is rendered via Bubble Tea framework rather than tmux panes and shell commands
- Terminal interaction uses PTY-based emulation instead of tmux pane linking

**Removed:**
- Shell-based sidebar implementation (`internal/sidebar/`)
- Tmux window linking for UI rendering
- `send-keys` based sidebar updates
- `r` key binding for sidebar refresh (now automatic)
- `refresh` command (no longer needed with TUI)

**Migration Guide:**
- Update to Go 1.21+ (new dependencies require it)
- Run `amux start` as before - now launches TUI instead of tmux
- Key bindings remain similar: `1-9` to switch, `q` to quit
- New: `Ctrl+A` toggles between sidebar and terminal input modes
- Direct tmux access still available: `tmux attach -t amux-agent-<project>`

### Added

- **Bubble Tea TUI framework** (`internal/tui/`)
  - Sidebar component with real-time status indicators
  - Terminal view with PTY-based output rendering
  - Mode switching between sidebar navigation and terminal input
  - Visual mode indicator at bottom of screen

- **PTY Terminal Emulation** (`internal/tui/pty/`)
  - PTY creation for each project session
  - Stdout capture and display
  - Stdin forwarding from TUI to PTY
  - Terminal resize handling (SIGWINCH propagation)
  - Proper cleanup on exit

- **Event Handling**
  - Number keys (1-9) for quick project switching
  - Arrow keys for sidebar navigation
  - Enter key to activate selected project
  - `q` key to quit (in sidebar mode)
  - `Ctrl+A` to toggle between sidebar and terminal modes

- **Styling**
  - Lipgloss-based style definitions
  - Color-coded status indicators
  - Configurable sidebar width
  - Visual border between sidebar and terminal view

### Dependencies

- Added `github.com/charmbracelet/bubbletea` v1.3.10
- Added `github.com/charmbracelet/lipgloss` v1.1.0
- Added `github.com/charmbracelet/bubbles` v1.0.0
- Added `github.com/creack/pty` v1.1.24

### Technical Details

The new architecture uses:
- **TUI Layer**: Bubble Tea Model-Update-View pattern
- **PTY Layer**: One PTY pair per project for terminal emulation
- **Session Layer**: Tmux sessions in background for persistence
- **Status Layer**: Existing file-based status monitoring

This provides:
- Better terminal compatibility (agents work normally)
- Smoother rendering (no shell interpretation issues)
- Richer UI capabilities (future extensibility)
- Same session persistence (tmux in background)

## [1.0.0] - 2026-03-01

### Initial Release

- Tmux-based session orchestration
- Shell-based sidebar with status indicators
- Support for opencode, claude, and codex agents
- File-based status reporting for opencode
- Project configuration via YAML

[Unreleased]: https://github.com/user/amux/compare/v2.0.0...HEAD
[2.0.0]: https://github.com/user/amux/compare/v1.0.0...v2.0.0
[1.0.0]: https://github.com/user/amux/releases/tag/v1.0.0
