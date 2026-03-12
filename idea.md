
This is a really cool project idea! I can help you think through the architecture. Let me search for some current best practices around tmux automation and status monitoring since tooling has evolved.This is an excellent architecture for an agent orchestrator! Let me break down a practical implementation approach:

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                    Orchestrator Session                      │
│  ┌──────────────────┬────────────────────────────────────┐  │
│  │     SIDEBAR      │         MAIN WORK AREA             │  │
│  │  (Project List)  │   (Link agent windows here)        │  │
│  │                  │                                    │  │
│  │  project-a ●     │  ┌──────────────────────────────┐  │  │
│  │  project-b ○     │  │ Agent Window (linked view)   │  │  │
│  │  project-c ●     │  │                              │  │  │
│  │                  │  │  [agent output here]         │  │  │
│  │  Status:         │  │                              │  │  │
│  │  ● running       │  └──────────────────────────────┘  │  │
│  │  ○ idle          │                                    │  │
│  │  ◐ needs review  │                                    │  │
│  └──────────────────┴────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘

┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐
│  Agent Session  │  │  Agent Session  │  │  Agent Session  │
│   (project-a)   │  │   (project-b)   │  │   (project-c)   │
│                 │  │                 │  │                 │
│  opencode/      │  │  claude code    │  │  codex          │
│  claude/codex   │  │                 │  │                 │
└─────────────────┘  └─────────────────┘  └─────────────────┘
```

## Implementation Components

### 1. Project Configuration

```bash
# ~/.agent-orchestrator/config.sh
declare -A PROJECTS=(
  ["project-a"]="/home/user/projects/project-a:opencode"
  ["project-b"]="/home/user/projects/project-b:claude"
  ["project-c"]="/home/user/projects/project-c:codex"
)

# Status colors
IDLE_COLOR="colour244"
RUNNING_COLOR="colour46"
NEEDS_REVIEW_COLOR="colour226"
```

### 2. Core Orchestrator Script

```bash
#!/usr/bin/env bash
# orchestrator.sh

set -euo pipefail

# =============================================================================
# CONFIGURATION
# =============================================================================

ORCH_SESSION="agent-orchestrator"
SIDEBAR_WIDTH=25
SIDEBAR_PANE="${ORCH_SESSION}:sidebar.0"

# Load config
source ~/.agent-orchestrator/config.sh

# =============================================================================
# SESSION MANAGEMENT
# =============================================================================

create_agent_session() {
  local project="$1"
  local path="$2"
  local agent="$3"
  
  local session_name="agent-${project}"
  
  # Skip if session already exists
  if tmux has-session -t "=$session_name" 2>/dev/null; then
    echo "Session $session_name already exists"
    return 0
  fi
  
  # Create new session in detached mode
  tmux new-session -d -s "$session_name" -c "$path"
  
  # Set session option for agent type (useful for status detection)
  tmux set-option -t "$session_name" "@agent_type" "$agent"
  tmux set-option -t "$session_name" "@project_path" "$path"
  tmux set-option -t "$session_name" "@project_name" "$project"
  
  # Start the agent in the session
  case "$agent" in
    opencode)
      tmux send-keys -t "$session_name" "opencode" Enter
      ;;
    claude)
      tmux send-keys -t "$session_name" "claude" Enter
      ;;
    codex)
      tmux send-keys -t "$session_name" "codex" Enter
      ;;
  esac
  
  echo "Created session $session_name with $agent in $path"
}

# =============================================================================
# SIDEBAR RENDERING
# =============================================================================

get_agent_status() {
  local session_name="$1"
  
  # Check if session exists
  if ! tmux has-session -t "=$session_name" 2>/dev/null; then
    echo "stopped"
    return
  fi
  
  # Check for running process (customize per agent)
  local pane_pid
  pane_pid=$(tmux list-panes -t "$session_name" -F "#{pane_pid}" | head -1)
  
  # Check if agent process is active (this needs customization per agent)
  # For example, check if the agent CLI is in the process list
  if ps -p "$pane_pid" -o command= 2>/dev/null | grep -qE "(opencode|claude|codex)"; then
    echo "running"
    return
  fi
  
  # Check for "needs review" markers in pane output
  local output
  output=$(tmux capture-pane -t "$session_name:0.0" -p 2>/dev/null | tail -20)
  
  if echo "$output" | grep -qiE "(needs review|waiting for input|please verify)"; then
    echo "needs-review"
    return
  fi
  
  echo "idle"
}

status_to_symbol() {
  local status="$1"
  case "$status" in
    running)      echo "●" ;;
    idle)         echo "○" ;;
    needs-review) echo "◐" ;;
    stopped)      echo "✗" ;;
    *)            echo "?" ;;
  esac
}

status_to_color() {
  local status="$1"
  case "$status" in
    running)      echo "colour46" ;;      # Green
    idle)         echo "colour244" ;;     # Gray
    needs-review) echo "colour226" ;;     # Yellow
    stopped)      echo "colour196" ;;     # Red
    *)            echo "colour255" ;;     # White
  esac
}

render_sidebar() {
  local sidebar_content=""
  sidebar_content+="  AGENT ORCHESTRATOR\n"
  sidebar_content+="  ────────────────────\n\n"
  sidebar_content+="  PROJECTS\n\n"
  
  for project in "${!PROJECTS[@]}"; do
    IFS=':' read -r path agent <<< "${PROJECTS[$project]}"
    local session_name="agent-${project}"
    local status
    status=$(get_agent_status "$session_name")
    local symbol
    symbol=$(status_to_symbol "$status")
    local color
    color=$(status_to_color "$status")
    
    # Format: "● project-a (opencode)"
    sidebar_content+="  #[fg=$color]$symbol#[default] $project\n"
    sidebar_content+="    #[fg=colour244]$agent#[default]\n\n"
  done
  
  sidebar_content+="\n  ────────────────────\n"
  sidebar_content+="  STATUS LEGEND\n"
  sidebar_content+="  #[fg=colour46]●#[default] running\n"
  sidebar_content+="  #[fg=colour244]○#[default] idle\n"
  sidebar_content+="  #[fg=colour226]◐#[default] needs review\n"
  sidebar_content+="  #[fg=colour196]✗#[default] stopped\n"
  
  # Write to sidebar pane
  tmux send-keys -t "$SIDEBAR_PANE" "clear" Enter
  tmux send-keys -t "$SIDEBAR_PANE" "echo -e '$sidebar_content'" Enter
}

# =============================================================================
# ORCHESTRATOR SESSION SETUP
# =============================================================================

setup_orchestrator() {
  # Create orchestrator session if needed
  if ! tmux has-session -t "=$ORCH_SESSION" 2>/dev/null; then
    tmux new-session -d -s "$ORCH_SESSION" -n "main"
    
    # Create sidebar pane (vertical split, left side)
    tmux split-window -h -t "$ORCH_SESSION:main" -l "$SIDEBAR_WIDTH"
    
    # Rename the panes for clarity
    tmux select-pane -t "$ORCH_SESSION:main.0" -T "sidebar"
    tmux select-pane -t "$ORCH_SESSION:main.1" -T "workspace"
    
    # Store pane IDs for later reference
    tmux set-option -t "$ORCH_SESSION" "@sidebar_pane" "$ORCH_SESSION:main.0"
    tmux set-option -t "$ORCH_SESSION" "@workspace_pane" "$ORCH_SESSION:main.1"
    
    # Style sidebar
    tmux set-option -t "$ORCH_SESSION:main.0" pane-border-style fg=colour238
    tmux set-option -t "$ORCH_SESSION:main.0" pane-active-border-style fg=colour238
    
    # Disable input in sidebar (optional)
    # tmux set-option -t "$ORCH_SESSION:main.0" @pane-readonly on
    
    # Select workspace pane as default
    tmux select-pane -t "$ORCH_SESSION:main.1"
  fi
  
  # Create all agent sessions
  for project in "${!PROJECTS[@]}"; do
    IFS=':' read -r path agent <<< "${PROJECTS[$project]}"
    create_agent_session "$project" "$path" "$agent"
  done
  
  # Initial sidebar render
  render_sidebar
}

# =============================================================================
# WINDOW LINKING (THE KEY FEATURE)
# =============================================================================

link_agent_window() {
  local project="$1"
  local session_name="agent-${project}"
  
  # Get workspace pane
  local workspace_pane
  workspace_pane=$(tmux show-option -t "$ORCH_SESSION" "@workspace_pane" | cut -d' ' -f2)
  
  # Create a new window in orchestrator session that links to agent session
  # This creates a VIEW into the agent session
  tmux link-window -s "${session_name}:0" -t "${ORCH_SESSION}:${project}"
  
  echo "Linked $session_name to orchestrator"
}

switch_to_project() {
  local project="$1"
  local target_window="${ORCH_SESSION}:${project}"
  
  # Check if window exists, if not create it
  if ! tmux list-windows -t "$ORCH_SESSION" | grep -q "^.*:.*${project}"; then
    link_agent_window "$project"
  fi
  
  # Switch to the window
  tmux select-window -t "$target_window"
  tmux select-pane -t "$target_window.1"  # Select workspace, not sidebar
}

# =============================================================================
# STATUS MONITORING
# =============================================================================

# Background process to update sidebar periodically
start_status_monitor() {
  while true; do
    render_sidebar
    sleep 2
  done &
  echo $! > ~/.agent-orchestrator/monitor.pid
}

stop_status_monitor() {
  if [ -f ~/.agent-orchestrator/monitor.pid ]; then
    kill "$(cat ~/.agent-orchestrator/monitor.pid)" 2>/dev/null || true
    rm ~/.agent-orchestrator/monitor.pid
  fi
}

# =============================================================================
# KEY BINDINGS
# =============================================================================

setup_key_bindings() {
  # Switch between projects
  local index=1
  for project in "${!PROJECTS[@]}"; do
    tmux bind-key "$index" run-shell "orchestrator.sh switch $project"
    ((index++))
  done
  
  # Refresh sidebar
  tmux bind-key R run-shell "orchestrator.sh render"
  
  # Kill current agent session
  tmux bind-key K confirm-before -p "Kill agent session? (y/n)" \
    run-shell "orchestrator.sh kill-current"
}

# =============================================================================
# CLI INTERFACE
# =============================================================================

case "${1:-}" in
  init)
    setup_orchestrator
    setup_key_bindings
    start_status_monitor
    ;;
  
  render)
    render_sidebar
    ;;
  
  switch)
    switch_to_project "$2"
    ;;
  
  kill-current)
    # Kill the currently viewed agent session
    current_window=$(tmux display-message -p "#{window_name}")
    if [[ "$current_window" =~ ^agent- ]]; then
      tmux kill-session -t "$current_window"
    fi
    ;;
  
  monitor-start)
    start_status_monitor
    ;;
  
  monitor-stop)
    stop_status_monitor
    ;;
  
  *)
    echo "Usage: $0 {init|render|switch <project>|kill-current|monitor-start|monitor-stop}"
    exit 1
    ;;
esac
```

### 3. Key Technical Decisions

#### Window Linking vs. Pane Joining

**Use `link-window`** for your use case because:
- Both sessions see the **same content** (true shared view)
- Agent session remains independent
- Changes in either session are reflected in both
- You can unlink without affecting the agent session

**Don't use `join-pane`** because:
- It **moves** the pane (agent loses its window)
- Not a "view" but a relocation

#### Status Detection Strategies

Based on the search results, you have several options for detecting agent status:

**Option A: Process Monitoring** (Recommended for MVP)
```bash
# Check if agent process is running in the pane
pane_pid=$(tmux list-panes -t "$session" -F "#{pane_pid}")
if ps -p "$pane_pid" -o command= | grep -q "opencode\|claude\|codex"; then
  echo "running"
fi
```

**Option B: Output Pattern Matching**
```bash
# Capture last N lines and look for status markers
output=$(tmux capture-pane -t "$session:0.0" -p | tail -20)
if echo "$output" | grep -qi "needs review"; then
  echo "needs-review"
fi
```

**Option C: Agent-Specific Markers** (Most Accurate)
Have each agent output specific markers:
```bash
# In agent session, when agent starts:
echo "___AGENT_STATUS_RUNNING___"

# When agent needs review:
echo "___AGENT_STATUS_NEEDS_REVIEW___"

# Orchestrator checks for these markers
```

### 4. Advanced Features

#### tmux Hooks for Real-time Updates

```bash
# Auto-refresh sidebar when switching windows
tmux set-hook -g window-switched 'run-shell "orchestrator.sh render"'

# Detect when agent process exits
tmux set-hook -g pane-exited 'run-shell "orchestrator.sh check-status #{session_name}"'
```

#### Custom Status Line for Orchestrator

```bash
# Add to orchestrator session setup
tmux set-option -t "$ORCH_SESSION" status-left-length 50
tmux set-option -t "$ORCH_SESSION" status-left \
  '#[fg=colour46,bold] ORCHESTRATOR #[default] | #{window_name}'
```

### 5. Integration with libtmux (Python)

For a more robust orchestrator, consider using **libtmux**:

```python
# orchestrator.py
import libtmux
import time

server = libtmux.Server()
orch_session = server.new_session('agent-orchestrator')

# Create sidebar
main_window = orch_session.windows[0]
main_window.rename_window('main')
sidebar = main_window.split_window(vertical=False, percentage=20)

# Create agent sessions
for project, (path, agent) in PROJECTS.items():
    agent_session = server.new_session(f'agent-{project}', 
                                       attach=False,
                                       window_name='agent')
    agent_window = agent_session.windows[0]
    
    # Link to orchestrator
    orch_session.link_window(
        source_window=agent_window,
        dst_index=project
    )

# Monitor loop
while True:
    for project in PROJECTS:
        session_name = f'agent-{project}'
        # Check status and update sidebar
        update_sidebar(project, session_name)
    time.sleep(2)
```

### 6. Quick Start

```bash
# 1. Initialize the orchestrator
./orchestrator.sh init

# 2. Attach to orchestrator session
tmux attach -t agent-orchestrator

# 3. Use key bindings:
#    1-9: Switch between projects
#    R:   Refresh sidebar
#    K:   Kill current agent session
```

This architecture gives you:
- **True sidebar** with real-time status
- **Linked views** into agent sessions (not copies)
- **Independent agent sessions** that can run detached
- **Scalable design** for multiple projects
- **Status monitoring** with visual indicators

Would you like me to elaborate on any specific component or help you implement a particular part?
