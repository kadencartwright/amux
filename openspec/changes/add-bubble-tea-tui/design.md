## Context

Currently, amux uses tmux panes and `send-keys` to render the sidebar. This approach has fundamental limitations:
- Content is typed into a shell, causing interpretation issues
- Tmux formatting codes (`#[fg=green]`) render as literal text
- Project names with special characters execute as shell commands
- No way to handle interactive events cleanly

The system manages agent sessions via tmux (which works well) but needs a proper UI layer.

## Goals / Non-Goals

**Goals:**
- Create a Bubble Tea TUI that displays projects in a sidebar with real-time status
- Embed agent terminals using PTY for full terminal support (readline, colors, etc.)
- Support keyboard navigation (arrow keys, number keys) for project switching
- Maintain status monitoring from existing specs
- Single binary that manages sessions internally

**Non-Goals:**
- Remove tmux entirely (still useful for session persistence)
- Support mouse interactions (keyboard-first for now)
- Handle agent output scrollback (simpler: just show current session)
- Complex window management (single active project view)

## Decisions

### Decision: Architecture - TUI + PTY per Session

**Approach:** Run Bubble Tea TUI as the main process. For each agent session, create a PTY pair. The TUI captures PTY output and renders it in the terminal view.

```
┌─────────────────────────────────────────────────────┐
│                    amux TUI                         │
├─────────────────────────────────────────────────────┤
│                                                     │
│  ┌──────────────┐  ┌─────────────────────────────┐ │
│  │   Sidebar    │  │      Terminal View          │ │
│  │              │  │                             │ │
│  │ ● project-a  │  │  ┌─────────────────────┐   │ │
│  │ ○ project-b  │  │  │ PTY stdout          │   │ │
│  │ ◐ project-c  │  │  │ $ opencode          │   │ │
│  │              │  │  │ > hello world       │   │ │
│  │ Legend       │  │  │                     │   │ │
│  └──────────────┘  │  └─────────────────────┘   │ │
│                    │       ↑ stdin              │ │
│                    └─────────────────────────────┘ │
│                                                     │
│   PTY Pairs (one per project):                     │
│   ┌──────────┐    ┌──────────┐    ┌──────────┐    │
│   │ pty-p1   │    │ pty-p2   │    │ pty-p3   │    │
│   │ ↓ stdout │    │ ↓ stdout │    │ ↓ stdout │    │
│   │ ↑ stdin  │    │ ↑ stdin  │    │ ↑ stdin  │    │
│   └────┬─────┘    └────┬─────┘    └────┬─────┘    │
│        │               │               │          │
│        ▼               ▼               ▼          │
│   ┌──────────┐    ┌──────────┐    ┌──────────┐    │
│   │opencode  │    │ claude   │    │  codex   │    │
│   │ process  │    │ process  │    │ process  │    │
│   └──────────┘    └──────────┘    └──────────┘    │
│                                                     │
└─────────────────────────────────────────────────────┘
```

**Rationale:**
- PTY gives us real terminal support (agents work normally)
- Bubble Tea handles the UI loop, rendering, and events
- Each project has its own isolated terminal session
- Only the active project's PTY output is displayed

**Alternative Considered:** Use tmux for terminal management, TUI just for sidebar overlay
- Rejected: Adds complexity of coordinating two window systems
- Rejected: Hard to manage focus between TUI and tmux

### Decision: Session Persistence Strategy

**Approach:** Use tmux in the background for session persistence, but the TUI manages the foreground interaction.

```
Background (tmux sessions):
  amux-p1: opencode process
  amux-p2: claude process  
  amux-p3: codex process

Foreground (TUI):
  - Renders sidebar
  - Renders active PTY
  - Forwards input to active PTY
```

**Rationale:**
- Tmux handles process lifecycle (if TUI crashes, agents keep running)
- User can `tmux attach` directly to a session if needed
- Gradual migration path: can use tmux or TUI

**Alternative Considered:** Pure Go process management
- Rejected: Would need to reimplement tmux's session persistence
- Rejected: Users expect tmux integration

### Decision: Status Monitoring Integration

**Approach:** Keep existing status monitoring from specs. The TUI sidebar polls status files and renders indicators.

**Rationale:**
- Reuse existing well-designed status system
- TUI just changes how status is displayed, not how it's determined

### Decision: Input Handling

**Approach:** When user types in the terminal view, forward keystrokes to the active PTY. When user uses special keys (arrow keys to navigate sidebar, number keys to switch), handle in TUI.

**Mode Switching:**
- Default: Focus is on terminal (keystrokes go to agent)
- Special keys (Ctrl+A, arrow keys): Switch focus to sidebar navigation
- Enter/Space on project: Switch to that project

**Rationale:**
- Agents expect full terminal input
- Need way to navigate without interfering with agent

## Risks / Trade-offs

**Risk:** PTY output rendering performance
- Mitigation: Use Bubble Tea's viewport component with efficient buffer management
- Mitigation: Limit scrollback buffer size (e.g., 1000 lines)

**Risk:** Terminal compatibility (different terminals render differently)
- Mitigation: Test with common terminals (xterm-256color, screen-256color, tmux)
- Trade-off: Some advanced terminal features may not work perfectly

**Risk:** Complexity of PTY management on different OS
- Mitigation: Use `creack/pty` which handles cross-platform differences
- Trade-off: Windows support may be limited (pty works on Windows 10+ with ConPTY)

**Risk:** Losing tmux features (copy mode, search, etc.)
- Mitigation: User can still `tmux attach` to sessions directly if needed
- Trade-off: TUI won't have all tmux features initially

**Risk:** Breaking existing workflows
- **BREAKING:** `amux start` behavior changes from "attach to tmux" to "launch TUI"
- Mitigation: Add `amux tmux` command for legacy tmux-based access
- Migration: Document the change, users can still use tmux directly

## Migration Plan

1. **Phase 1: Development** - Build TUI alongside existing tmux code
2. **Phase 2: Testing** - Ensure PTY handling works with opencode, claude, codex
3. **Phase 3: Release** - Replace `amux start` with TUI version
   - Add `--tmux` flag or `amux start --legacy` for old behavior
   - Update README with new workflow
4. **Phase 4: Deprecation** - Eventually remove shell-based sidebar

**Rollback:** Users can always `tmux attach -t amux-agent-<project>` directly

## Open Questions

1. **Scrollback:** How much output history should we keep per session? 1000 lines? 10,000?
2. **Resize:** How to handle terminal resize events propagating to PTY?
3. **Copy/Paste:** Should we support copy mode in the TUI, or rely on terminal's selection?
