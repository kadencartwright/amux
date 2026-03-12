## Context

This is a new tool, not a modification to existing systems. The agent orchestrator will be a standalone Go binary that leverages tmux for session management.

**Current State**: Users manually manage multiple tmux sessions for different AI agents (opencode, claude, codex). Switching contexts requires detaching, navigating, and re-attaching - high friction.

**Constraints**:
- Must work within tmux (no custom terminal UI)
- Must support existing agents without modification (except opencode plugin)
- Personal tool first - simplicity over enterprise features
- Single-focus model (one project active at a time)

## Goals / Non-Goals

**Goals:**
- Single-pane view of all projects with status indicators
- One-keystroke switching between projects (1, 2, 3...)
- Accurate status detection for opencode via native hooks
- Basic status detection for claude/codex via heuristics
- Static YAML configuration

**Non-Goals:**
- Multi-view mode (showing multiple projects simultaneously)
- Web UI or TUI outside tmux
- Dynamic project management (add/remove at runtime)
- Non-tmux backends
- Enterprise features (multi-user, permissions, etc.)

## Decisions

### Use Go, not Python or Bash
**Rationale**: Go provides:
- Single binary deployment (no dependencies)
- Better error handling than bash
- Easier to extend later than bash
- Faster than Python for CLI tools

**Alternatives considered**: Bash (simpler but fragile), Python (libtmux exists but adds dependency)

### Single-focus model with sidebar
**Rationale**: Agent work requires deep context. Full-screen view of one project is more valuable than partial views of many. Sidebar provides sufficient awareness.

**Alternatives considered**: Multi-view with previews (complex, small text), tabs only (no status visibility)

### File-based status for opencode hooks
**Rationale**: Simple, debuggable, survives crashes. Socket would be faster but adds complexity.

**Format**: `~/.local/share/amux/status/<project>.json` with `{"status": "...", "timestamp": "..."}`

**Alternatives considered**: Unix socket (complex), tmux user options (limited data), stdout parsing (unreliable)

### Static YAML config
**Rationale**: Personal tool - editing a file is fine. No need for interactive management.

**Location**: `~/.config/amux/config.yaml`

### Tmux-native UI
**Rationale**: Users are already in tmux. Don't fight the environment.

**Implementation**: Sidebar pane + linked windows. No custom rendering libraries.

### Package Structure
Standard Go project layout:

```
amux/
├── go.mod                    # Module: github.com/user/amux
├── Makefile                  # Build, test, install targets
├── README.md                 # Documentation
├── cmd/amux/
│   └── main.go              # CLI entry point only
├── internal/
│   ├── config/              # Config loading (yaml)
│   ├── tmux/                # Tmux command wrappers
│   ├── agents/              # Status detection logic
│   ├── sidebar/             # Sidebar rendering
│   └── session/             # Session orchestration
└── pkg/plugins/opencode/    # Public plugin API
```

**Rationale**:
- `internal/` - Implementation details, not exposed
- `pkg/plugins/` - Public API for opencode integration
- `cmd/amux/` - Minimal main package (best practice)
- Clean separation of concerns for testing

## Risks / Trade-offs

| Risk | Mitigation |
|------|------------|
| Status detection for non-opencode agents is unreliable | Accept limitation; opencode is primary use case |
| Tmux version differences in link-window behavior | Test on common versions; document minimum version |
| Sidebar flickering from polling | Acceptable for MVP; optimize later if needed |
| Agent output parsing is fragile | Use multiple signals (time + patterns); allow manual override |

## Migration Plan

N/A - This is a new tool, not a change to existing systems.

**Installation**: 
1. `go build -o amux`
2. `mv amux ~/bin/`
3. Create `~/.config/amux/config.yaml`
4. Run `amux start`

## Open Questions

1. ~~Status file location~~: Decided - `~/.local/share/amux/status/`
2. **Poll interval**: Start with 2s, make configurable?
3. ~~Should we support "paused" state?~~: Decided - paused = idle
