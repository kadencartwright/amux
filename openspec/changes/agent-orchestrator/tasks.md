## 1. Project Setup

- [ ] 1.1 Initialize Go module (`go mod init orchestrator`)
- [ ] 1.2 Create directory structure (main.go, config.go, tmux.go, sidebar.go, agents.go)
- [ ] 1.3 Add necessary imports (yaml.v3, etc.)

## 2. Configuration

- [ ] 2.1 Define Config struct with Projects array
- [ ] 2.2 Implement loadConfig() to read YAML from ~/.config/amux/config.yaml
- [ ] 2.3 Add validation for required fields (name, path, agent)
- [ ] 2.4 Create sample config file for testing

## 3. Tmux Client

- [ ] 3.1 Implement tmuxHasSession(session string) bool
- [ ] 3.2 Implement tmuxNewSession(name, dir string) error
- [ ] 3.3 Implement tmuxSplitWindow(target string, width int) error
- [ ] 3.4 Implement tmuxLinkWindow(src, dst string) error
- [ ] 3.5 Implement tmuxSelectWindow(target string) error
- [ ] 3.6 Implement tmuxSendKeys(target, keys string) error
- [ ] 3.7 Implement tmuxCapturePane(target string, lines int) (string, error)
- [ ] 3.8 Implement tmuxListPanes(session string) to get pane PID

## 4. Session Orchestration

- [ ] 4.1 Implement ensureSession() to create "amux" session with sidebar
- [ ] 4.2 Implement ensureAgentSession() to create detached agent sessions
- [ ] 4.3 Implement switchToProject() with number key binding
- [ ] 4.4 Create initial sidebar rendering function
- [ ] 4.5 Implement attachToOrchestrator() function

## 5. Status Detection

- [ ] 5.1 Implement isAgentRunning(pid int, agentType string) bool
- [ ] 5.2 Implement getProcessStatus() to check if tmux session exists
- [ ] 5.3 Implement pattern matching for "needs review" detection
- [ ] 5.4 Implement Status type with constants (Stopped, Idle, Running, NeedsReview)
- [ ] 5.5 Create statusToSymbol() and statusToColor() helper functions

## 6. Opencode Integration

- [ ] 6.1 Create ~/.local/share/amux/status/ directory on startup
- [ ] 6.2 Implement readStatusFile(project string) (Status, error)
- [ ] 6.3 Implement writeStatusFile() for opencode plugin
- [ ] 6.4 Add staleness detection (ignore files older than 30s)
- [ ] 6.5 Create opencode plugin that hooks into agent lifecycle

## 7. Sidebar

- [ ] 7.1 Implement renderSidebar() with header and project list
- [ ] 7.2 Add status symbols with tmux color formatting
- [ ] 7.3 Implement updateSidebar() polling loop (2s interval)
- [ ] 7.4 Add legend at bottom of sidebar
- [ ] 7.5 Handle sidebar refresh on 'r' key binding

## 8. CLI Interface

- [ ] 8.1 Implement `amux init` command to create sample config
- [ ] 8.2 Implement `amux start` command (main entry point)
- [ ] 8.3 Implement `amux stop` command (detach orchestrator)
- [ ] 8.4 Add help text and usage information

## 9. Integration & Testing

- [ ] 9.1 Test with single opencode project
- [ ] 9.2 Test with multiple projects
- [ ] 9.3 Test project switching (1, 2, 3 keys)
- [ ] 9.4 Test status detection accuracy
- [ ] 9.5 Test opencode plugin status updates
- [ ] 9.6 Test error handling (missing config, tmux not running)

## 10. Documentation

- [ ] 10.1 Write README with installation instructions
- [ ] 10.2 Document config file format
- [ ] 10.3 Document key bindings
- [ ] 10.4 Add troubleshooting section
