## Why

Managing multiple AI agent sessions across different projects is painful. Currently, switching between projects requires detaching from one tmux session, navigating to another directory, and starting a new agent - losing context each time. There's no single-pane-of-glass view to see all active agents and their statuses at a glance.

## What Changes

- Create a tmux-based orchestrator that manages multiple agent sessions from a single interface
- Add a persistent sidebar showing all projects with real-time status indicators (running, idle, needs-review, stopped)
- Implement single-keystroke project switching (1, 2, 3...) that links agent windows into the main view
- Build status detection system: process monitoring for basic agents, native hooks for opencode
- Add opencode plugin for accurate "needs review" detection via file-based status communication
- Support screen scraping for claude and codex agents as fallback
- Create static YAML configuration for project definitions

## Capabilities

### New Capabilities
- `session-orchestration`: Manage multiple tmux sessions from a central orchestrator with sidebar navigation and window linking
- `agent-status-monitoring`: Detect and display agent states (running, idle, needs-review, stopped) via process monitoring and output analysis
- `opencode-integration`: Native plugin support for opencode agent with bidirectional status communication

### Modified Capabilities
- None. This is a new tool, not a modification to existing capabilities.

## Impact

- New Go binary (`amux`) added to the codebase
- New configuration directory `~/.config/amux/` for project definitions
- New tmux session naming convention (`amux` for orchestrator, `agent-<project>` for agents)
- Opencode plugin requires opencode to load and use the integration
- No breaking changes to existing systems - this is a standalone tool
