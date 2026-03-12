## Why

The current sidebar implementation sends content as keystrokes to a shell, which causes multiple problems: tmux formatting codes render as literal text, special shell characters execute as commands, and the UI is fragile and unresponsive. We need a proper sidebar that renders correctly and provides a reliable user experience.

## What Changes

- Replace shell-based sidebar with a dedicated TUI overlay program
- TUI draws over the left portion of the terminal window using ANSI escape codes
- Sidebar displays project list with colored status indicators (● running, ○ idle, ◐ needs review, ✗ stopped)
- Key bindings for project switching (1-9) and refresh (r) work via the TUI
- Implement overlay/pass-through mode switching (press `q` or `Esc` to hide/show sidebar)
- TUI communicates with tmux via command execution to switch windows and manage sessions

## Capabilities

### New Capabilities
- `tui-sidebar-rendering`: Rendering project list with status indicators using ANSI colors in a TUI overlay
- `overlay-mode-switching`: Toggle between sidebar overlay visible and hidden (pass-through) modes
- `tui-tmux-integration`: Communication between TUI sidebar and tmux for window switching and session management

### Modified Capabilities
- None (this is a pure implementation change with no external behavior changes)

## Impact

- New dependency: Bubble Tea (charm.sh) for TUI framework
- New dependency: Lipgloss for styling
- Replaces `internal/sidebar` package with new TUI-based implementation
- Adds new `cmd/amux-sidebar` binary for the sidebar TUI program
- Modifies session orchestration to launch TUI in sidebar pane instead of shell
- Configuration format extended with new optional field `sidebar_toggle_key` (default: "S")
- No breaking changes to existing configuration files
