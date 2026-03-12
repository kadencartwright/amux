## Why

The current tmux-based sidebar implementation sends text as keystrokes to a shell, which causes multiple problems: tmux formatting codes render as literal text, special characters in project names execute as shell commands (security risk), and the UI is limited by shell interpretation. A proper TUI framework will provide reliable rendering, rich interactions, and a foundation for future enhancements.

## What Changes

**BREAKING**: Complete architectural change from tmux-based UI to Bubble Tea TUI

- Replace shell-based sidebar with Bubble Tea TUI application
- Remove dependency on tmux for UI rendering (still used for session management)
- Add terminal emulation via PTY for running agent processes
- Implement unified interface with sidebar and terminal view in single application
- Support mouse interactions and rich keyboard shortcuts
- Add smooth status updates without flickering

## Capabilities

### New Capabilities
- `tui-framework`: Bubble Tea-based terminal user interface with sidebar and terminal view
- `terminal-emulation`: PTY-based terminal emulation for running agents within TUI
- `event-handling`: Keyboard and mouse event processing for project switching and interactions

### Modified Capabilities
- `session-orchestration`: Changes from tmux-pane-based UI to TUI-managed sessions. Requirements change: instead of "sidebar displays project list" via tmux panes, it's "sidebar renders via TUI". Instead of "project switching via keystrokes" in tmux, it's "project switching via TUI event loop".

## Impact

- **Dependencies**: Add `github.com/charmbracelet/bubbletea`, `github.com/charmbracelet/lipgloss`, `github.com/charmbracelet/bubbles`, `github.com/creack/pty`
- **Architecture**: Replace tmux-centric UI code with TUI components
- **User Experience**: Single executable instead of tmux session management, direct attachment without terminal issues
- **Code Structure**: New `internal/tui/` package, removal of `internal/sidebar/` shell-based implementation
