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
1. Create the tmux session `amux`
2. Create a sidebar pane showing all projects
3. Create agent sessions for each project
4. Attach you to the orchestrator

### Key Bindings

When inside the amux session:

- `1-9` - Switch to project N
- `r` - Refresh sidebar

### Commands

```bash
amux init    # Create sample configuration
amux start   # Start orchestrator and attach
amux stop    # Detach from orchestrator
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
- **File-based status**: Simple JSON files for opencode integration
- **Process monitoring**: Fallback detection for non-opencode agents

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

### Sidebar not updating

Check that the sidebar pane exists:
```bash
tmux list-panes -t amux
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
