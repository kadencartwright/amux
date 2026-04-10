## Context

Amux is a tmux-based orchestrator for managing multiple AI agent sessions. The current implementation uses tmux's `send-keys` command to display sidebar content in a dedicated pane. This approach is fundamentally broken because:

1. Content is sent as keystrokes to a shell, causing shell interpretation issues
2. Tmux formatting codes (`#[fg=green]`) don't render in shell output
3. Special characters in project names can execute as shell commands
4. No way to handle interactive input in the sidebar

We need a proper TUI that renders directly to the terminal and communicates with tmux through its command interface.

## Goals / Non-Goals

**Goals:**
- Create a reliable sidebar that displays project list with real-time status updates
- Use ANSI escape codes for colors and styling (not tmux formatting)
- Support keyboard navigation (1-9 for project switching, r for refresh, q/Esc to toggle visibility)
- Maintain compatibility with existing tmux session management
- Allow users to hide the sidebar to maximize workspace area

**Non-Goals:**
- Replacing tmux entirely with a custom window manager
- Supporting mouse interactions (keyboard-only for now)
- Real-time collaboration or multi-user features
- Changing the configuration file format or project structure

## Decisions

### Decision 1: Use Bubble Tea as TUI Framework

**Choice:** Use Charm's Bubble Tea (github.com/charmbracelet/bubbletea) as the TUI framework.

**Rationale:**
- Mature, well-documented Go TUI framework
- Built-in support for ANSI styling via Lipgloss
- Event-driven architecture fits well with keyboard handling
- Active community and good examples

**Alternatives considered:**
- tview: Good but less idiomatic Go, more complex API
- termui: Lower-level, requires more boilerplate
- Custom ANSI handling: Too error-prone for this use case

### Decision 2: Sidebar as Separate Binary

**Choice:** Create a dedicated `amux-sidebar` binary that runs in the sidebar pane.

**Rationale:**
- Clean separation of concerns
- Can be started/stopped independently
- Easier to test and debug
- Doesn't block the main orchestrator

**Architecture:**
```
┌─────────────────────────────────────────────────────────┐
│              amux-orchestrator session                  │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  ┌──────────────────┬───────────────────────────────┐  │
│  │                  │                               │  │
│  │  amux-sidebar    │   amux-agent-<project>        │  │
│  │  (TUI program)   │   (opencode/claude/codex)     │  │
│  │                  │                               │  │
│  │  ┌────────────┐  │   ┌───────────────────────┐   │  │
│  │  │ Projects   │  │   │   $ opencode          │   │  │
│  │  │ ─────────  │  │   │   > working...        │   │  │
│  │  │ ● proj-a   │  │   │                       │   │  │
│  │  │ ○ proj-b   │  │   │   (real terminal)     │   │  │
│  │  │ ◐ proj-c   │  │   │                       │   │  │
│  │  │            │  │   └───────────────────────┘   │  │
│  │  │ 1-9 switch │  │                               │  │
│  │  │ q hide     │  │                               │  │
│  │  └────────────┘  │                               │  │
│  │                  │                               │  │
│  └──────────────────┴───────────────────────────────┘  │
│                                                         │
│  Sidebar renders ANSI directly to terminal             │
│  TUI handles keys locally                              │
│  On switch: execs 'tmux select-window'                 │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

### Decision 3: Overlay Mode with Configurable Toggle Hotkey

**Choice:** Implement a toggle to hide/show the sidebar overlay using a **configurable** tmux prefix hotkey.

**Rationale:**
- Users need maximum screen real estate sometimes
- Overlay approach is simpler than resizing tmux panes dynamically
- `q` or `Esc` hides the sidebar
- **Important:** We cannot listen for "any key" to wake because the user may be typing in the workspace pane full-screened - we would steal their keystrokes
- Instead, use a dedicated tmux prefix hotkey that is configurable and defaults to `Prefix + A`

**Configuration:**
```yaml
# ~/.config/amux/config.yaml
sidebar_width: 25
sidebar_toggle_key: "A"  # Single character, uppercase A by default

projects:
  - name: project1
    path: ~/projects/project1
    agent: opencode
```

**Implementation:**
- **Visible mode:** TUI renders project list and handles 1-9, r, q/Esc keys locally
- **Hidden mode:** TUI clears screen, stops rendering, and goes idle
- **Toggle mechanism:**
  - Tmux binds `Prefix + {sidebar_toggle_key}` to send an ANSI escape sequence to the sidebar pane
  - Sidebar TUI listens for this escape sequence via stdin and toggles state
  - This ensures workspace keystrokes are never intercepted

**Default Key Choice:**
- **Default: `A`** (uppercase A)
- **Why:** Mnemonic for "AMUX", not used by default tmux
- **Alternatives user might configure:** `B` (bar), `N` (navigator), `O` (overlay), `T` (toggle)

**Example user flow:**
```
User is working full-screen in workspace
  ↓
User presses Ctrl+A A (tmux prefix + A, or configured key)
  ↓
Tmux sends F12 key to sidebar pane
  ↓
Sidebar TUI detects F12 and toggles visibility
  ↓
Sidebar redraws (if was hidden) or clears (if was visible)
  ↓
User presses q to hide again
```

### Decision 4: Status via File Watching

**Choice:** Sidebar reads project status from JSON files in `~/.local/share/amux/status/`.

**Rationale:**
- Simple, file-based communication (no sockets/pipes needed)
- Works with existing opencode plugin architecture
- TUI can poll for changes or use fsnotify for efficiency
- No direct coupling between agent and sidebar processes

**Status file format:**
```json
{
  "project": "myproject",
  "status": "running",
  "timestamp": "2024-01-15T10:30:00Z"
}
```

**Status values:**
- `running` (● green) - agent is actively working
- `idle` (○ gray) - agent waiting for input
- `needs_review` (◐ yellow) - agent needs user approval
- `stopped` (✗ red) - agent session not running

## Risks / Trade-offs

**[Risk]** TUI overlay might conflict with tmux's own key handling
→ **Mitigation:** Use tmux's `bind-key` only for initial window setup, let TUI handle all sidebar keys. TUI runs in its own pane with focus.

**[Risk]** ANSI codes might not render correctly in all terminals
→ **Mitigation:** Bubble Tea handles terminal compatibility. We accept this risk as most modern terminals support ANSI.

**[Risk]** Additional dependency increases binary size
→ **Mitigation:** Bubble Tea adds ~2-3MB. Acceptable for CLI tool. Consider static linking if needed.

**[Risk]** File watching for status might have latency
→ **Mitigation:** Poll every 2 seconds as fallback, use fsnotify for immediate updates. Trade-off acceptable for this use case.

**[Trade-off]** Sidebar takes up horizontal space
→ **Acceptance:** This is by design. User can hide it with `q`. In future, could make width configurable.

## Migration Plan

1. New code is additive - doesn't break existing sessions
2. On `amux start`, new sessions use TUI sidebar
3. Existing sessions continue using old sidebar until restarted
4. No data migration needed (config format unchanged)

**Rollback:**
- Revert to previous commit
- Old `send-keys` sidebar still works (though broken)

## Open Questions

1. Should we support mouse clicks on sidebar items? (Deferred to v2)
2. What's the minimum terminal width we should support? (Start with 80 cols)
3. Should we show git branch or other project metadata in sidebar? (Future enhancement)
