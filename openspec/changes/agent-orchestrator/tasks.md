## 1. Project Setup

- [x] 1.1 Initialize Go module (`go mod init orchestrator`)
- [x] 1.2 Create directory structure (main.go, config.go, tmux.go, sidebar.go, agents.go)
- [x] 1.3 Add necessary imports (yaml.v3, etc.)

## 2. Configuration

- [x] 2.1 Define Config struct with Projects array
- [x] 2.2 Implement loadConfig() to read YAML from ~/.config/amux/config.yaml
- [x] 2.3 Add validation for required fields (name, path, agent)
- [x] 2.4 Create sample config file for testing

## 3. Tmux Client

- [x] 3.1 Implement tmuxHasSession(session string) bool
- [x] 3.2 Implement tmuxNewSession(name, dir string) error
- [x] 3.3 Implement tmuxSplitWindow(target string, width int) error
- [x] 3.4 Implement tmuxLinkWindow(src, dst string) error
- [x] 3.5 Implement tmuxSelectWindow(target string) error
- [x] 3.6 Implement tmuxSendKeys(target, keys string) error
- [x] 3.7 Implement tmuxCapturePane(target string, lines int) (string, error)
- [x] 3.8 Implement tmuxListPanes(session string) to get pane PID

## 4. Session Orchestration

- [x] 4.1 Implement ensureSession() to create "amux" session with sidebar
- [x] 4.2 Implement ensureAgentSession() to create detached agent sessions
- [x] 4.3 Implement switchToProject() with number key binding
- [x] 4.4 Create initial sidebar rendering function
- [x] 4.5 Implement attachToOrchestrator() function

## 5. Status Detection

- [x] 5.1 Implement isAgentRunning(pid int, agentType string) bool
- [x] 5.2 Implement getProcessStatus() to check if tmux session exists
- [x] 5.3 Implement pattern matching for "needs review" detection
- [x] 5.4 Implement Status type with constants (Stopped, Idle, Running, NeedsReview)
- [x] 5.5 Create statusToSymbol() and statusToColor() helper functions

## 6. Opencode Integration

- [x] 6.1 Create ~/.local/share/amux/status/ directory on startup
- [x] 6.2 Implement readStatusFile(project string) (Status, error)
- [x] 6.3 Implement writeStatusFile() for opencode plugin
- [x] 6.4 Add staleness detection (ignore files older than 30s)
- [x] 6.5 Create opencode plugin that hooks into agent lifecycle

## 7. Sidebar

- [x] 7.1 Implement renderSidebar() with header and project list
- [x] 7.2 Add status symbols with tmux color formatting
- [x] 7.3 Implement updateSidebar() polling loop (2s interval)
- [x] 7.4 Add legend at bottom of sidebar
- [x] 7.5 Handle sidebar refresh on 'r' key binding

## 8. CLI Interface

- [x] 8.1 Implement `amux init` command to create sample config
- [x] 8.2 Implement `amux start` command (main entry point)
- [x] 8.3 Implement `amux stop` command (detach orchestrator)
- [x] 8.4 Add help text and usage information

## 9. Integration & Testing

- [x] 9.1 Test with single opencode project
- [x] 9.2 Test with multiple projects
- [x] 9.3 Test project switching (1, 2, 3 keys)
- [x] 9.4 Test status detection accuracy
- [x] 9.5 Test opencode plugin status updates
- [x] 9.6 Test error handling (missing config, tmux not running)

## 10. Documentation

- [x] 10.1 Write README with installation instructions
- [x] 10.2 Document config file format
- [x] 10.3 Document key bindings
- [x] 10.4 Add troubleshooting section
