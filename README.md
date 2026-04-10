# amux - Agent Multiplexer

A tmux-based orchestrator for managing multiple AI agent sessions across different projects.

## Overview

amux provides a single-pane-of-glass view for managing multiple AI agents (opencode, claude, codex) running in different project directories. It uses tmux sessions and window linking to create a seamless workflow.

```
┌─────────────────────────────────────────────────────────────┐
│                    amux Session                              │
│  ┌──────────────────┬────────────────────────────────────┐  │
│  │     SIDEBAR      │         MAIN WORK AREA             │  │
│  │  (Project List)  │   (Linked agent window)            │  │
│  │                  │                                    │  │
│  │  project-a ●     │  ┌──────────────────────────────┐  │  │
│  │  project-b ○     │  │ Agent output here            │  │  │
│  │  project-c ◐     │  │                              │  │  │
│  │                  │  └──────────────────────────────┘  │  │
│  │  Status:         │                                    │  │
│  │  ● running       │                                    │  │
│  │  ○ idle          │                                    │  │
│  │  ◐ needs review  │                                    │  │
│  └──────────────────┴────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

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
sidebar_toggle_key: A  # Key to toggle sidebar visibility (Prefix + A)

projects:
  - name: myproject
    path: ~/projects/myproject
    agent: opencode
  - name: dotfiles
    path: ~/dotfiles
    agent: claude
```

### Configuration Options

- `sidebar_width`: Width of the sidebar pane (default: 25)
- `sidebar_toggle_key`: Single character key to toggle sidebar visibility with tmux prefix (default: "A")

## Usage

### Start the orchestrator

```bash
amux start
```

This will:
1. Create the tmux session `amux`
2. Create a sidebar pane showing all projects
3. Create agent sessions for each project
4. Attach you to the orchestrator

### Key Bindings

When inside the amux session:

**In the sidebar pane:**
- `1-9` - Switch to project N
- `r` - Refresh sidebar
- `q` or `Esc` - Hide sidebar

**Global tmux bindings:**
- `Prefix + A` (or your configured toggle key) - Toggle sidebar visibility

The sidebar runs as a TUI application and can be hidden to maximize workspace area. When hidden, press your configured toggle key (default: Prefix + A) to show it again.

### Commands

```bash
amux init     # Create sample configuration
amux start    # Start orchestrator and attach
amux stop     # Detach from orchestrator
amux switch   # Switch to a project
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

- **Single-focus model**: One project active at a time for deep context
- **Tmux-native**: Uses tmux sessions, panes, and window linking
- **TUI Sidebar**: Bubble Tea-based sidebar with ANSI colors and keyboard navigation
- **File-based status**: Simple JSON files for opencode integration
- **Configurable toggle**: Hide/show sidebar with configurable tmux hotkey

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

- tmux 3.0+
- Go 1.21+ (for building)
- opencode, claude, or codex (optional)

## Troubleshooting

### "tmux: command not found"

Install tmux:
- macOS: `brew install tmux`
- Ubuntu/Debian: `sudo apt install tmux`
- Fedora: `sudo dnf install tmux`

### "config file not found"

Run `amux init` to create a sample configuration.

### Sidebar not showing or toggling

Check that the sidebar TUI is running:
```bash
tmux list-panes -t amux-orchestrator
```

To manually toggle the sidebar visibility:
```bash
tmux send-keys -t amux-orchestrator:0.0 F12
```

### Agent not starting

Verify the agent command is in your PATH:
```bash
which opencode
which claude
which codex
```

## License

MIT
