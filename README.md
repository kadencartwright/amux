# amux - Agent Multiplexer

A Bubble Tea TUI-based orchestrator for managing multiple AI agent sessions across different projects.

## Overview

amux provides a unified terminal user interface for managing multiple AI agents (opencode, claude, codex) running in different project directories. It uses Bubble Tea for the TUI and PTY-based terminal emulation for seamless agent interaction.

```
┌─────────────────────────────────────────────────────────────────┐
│                           AMUX TUI                              │
├─────────────────────────────────────────────────────────────────┤
│  ┌────────────────────┬─────────────────────────────────────┐  │
│  │      SIDEBAR       │          TERMINAL VIEW              │  │
│  │                    │                                     │  │
│  │  PROJECTS          │  Project: project-a (opencode)      │  │
│  │                    │  ─────────────────────────────────  │  │
│  │  > ● project-a     │                                     │  │
│  │    opencode        │  $ opencode                         │  │
│  │                    │  > Hello! How can I help?           │  │
│  │    ○ project-b     │                                     │  │
│  │    claude          │                                     │  │
│  │                    │                                     │  │
│  │    ◐ project-c     │                                     │  │
│  │    codex           │  SIDEBAR MODE                       │  │
│  │                    │  Ctrl+A: switch mode | 1-9: switch  │  │
│  │  ──────────────────┴─────────────────────────────────────┘  │
│  │  STATUS LEGEND                                              │
│  │  ● running                                                  │
│  │  ○ idle                                                     │
│  │  ◐ needs review                                             │
│  │  ✗ stopped                                                  │
│  └─────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

## Features

- **Bubble Tea TUI**: Rich terminal UI with sidebar and terminal view
- **PTY Terminal Emulation**: Full terminal support for agents with ANSI colors and readline
- **Real-time Status**: Visual indicators for agent states (running, idle, needs review, stopped)
- **Keyboard Navigation**: Quick project switching with number keys (1-9)
- **Session Persistence**: Uses tmux in the background for process lifecycle management
- **Multi-Agent Support**: Works with opencode, claude, codex, and custom agents

## Installation

### Using Make (recommended)

```bash
# Clone or download the repository
cd cmd/amux

# Build and install to ~/bin
make install

# Create initial config
amux init
```

### Manual Build

```bash
# Clone or download the repository
cd cmd/amux

# Build the binary
go build -o amux

# Move to your PATH
mv amux ~/bin/  # or /usr/local/bin/

# Create initial config
amux init
```

## Configuration

Edit `~/.config/amux/config.yaml`:

```yaml
sidebar_width: 25

projects:
  - name: myproject
    path: ~/projects/myproject
    agent: opencode
  - name: dotfiles
    path: ~/dotfiles
    agent: claude
```

## Usage

### Start the orchestrator

```bash
amux start
```

This will:
1. Launch the Bubble Tea TUI
2. Display a sidebar with all configured projects
3. Show real-time status indicators
4. Allow you to switch between projects
5. Create tmux sessions in the background for persistence

### Key Bindings

**Global:**
- `Ctrl+A` - Toggle between sidebar mode and terminal mode
- `Ctrl+C` - Quit amux
- `1-9` - Switch to project by number

**Sidebar Mode:**
- `↑/k` - Navigate up
- `↓/j` - Navigate down
- `Enter` - Activate selected project
- `q` - Quit amux

**Terminal Mode:**
- All keystrokes are forwarded to the active agent's PTY
- Use `Ctrl+A` to return to sidebar mode

### Commands

```bash
amux init    # Create sample configuration
amux start   # Start TUI and attach
amux stop    # Stop all amux sessions
```

## Status Indicators

| Symbol | Color  | Meaning       |
|--------|--------|---------------|
| ●      | Green  | Running       |
| ○      | Gray   | Idle          |
| ◐      | Yellow | Needs Review  |
| ✗      | Red    | Stopped       |

## Opencode Integration

For accurate status detection with opencode, use the provided plugin:

```go
import "github.com/yourusername/amux/plugins/opencode"

func main() {
    client, err := amux.NewClient("myproject")
    if err != nil {
        log.Fatal(err)
    }
    defer client.Close()

    // When starting work
    client.Running()

    // When waiting for user input
    client.NeedsReview()

    // When returning to idle
    client.Idle()
}
```

Status files are stored in `~/.local/share/amux/status/<project>.json`.

## Architecture

- **TUI Framework**: Bubble Tea for the main interface
- **Terminal Emulation**: PTY pairs for each agent session
- **Status Monitoring**: File-based status for opencode, process detection for others
- **Session Management**: Tmux in background for persistence
- **Input Handling**: Mode-based (sidebar vs terminal) with Ctrl+A switching

## Breaking Changes (v2.0.0)

The TUI-based version represents a complete architectural change from the previous tmux-based UI:

- **New**: Bubble Tea TUI with PTY terminal emulation
- **New**: Single executable that manages sessions internally
- **Changed**: `amux start` now launches a TUI instead of attaching to tmux
- **Removed**: Shell-based sidebar implementation
- **Migration**: You can still access tmux sessions directly with `tmux attach -t amux-agent-<project>`

## Development

### Available Make Targets

```bash
make build    # Build the binary
make test     # Run tests
make install  # Build and install to ~/bin
make clean    # Remove built binary
make dev      # Build and start amux
make init     # Build and run init
```

## Requirements

- Go 1.21+ (for building)
- tmux 3.0+ (for session persistence)
- Terminal with 256-color support (recommended)
- opencode, claude, or codex (optional)

## Troubleshooting

### "tmux: command not found"

Install tmux:
- macOS: `brew install tmux`
- Ubuntu/Debian: `sudo apt install tmux`
- Fedora: `sudo dnf install tmux`

### "config file not found"

Run `amux init` to create a sample configuration.

### TUI rendering issues

Ensure your terminal supports 256 colors:
```bash
echo $TERM
# Should show something like: xterm-256color or screen-256color
```

### Agent not starting

Verify the agent command is in your PATH:
```bash
which opencode
which claude
which codex
```

### Direct tmux access

If you need to access a session directly:
```bash
tmux attach -t amux-agent-<project-name>
```

## License

MIT
