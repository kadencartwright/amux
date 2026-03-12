## 1. Dependencies and Setup

- [ ] 1.1 Add Bubble Tea dependencies to go.mod (bubbletea, lipgloss, bubbles)
- [ ] 1.2 Add PTY dependency (creack/pty)
- [ ] 1.3 Run go mod tidy to download dependencies
- [ ] 1.4 Create internal/tui/ directory structure

## 2. Core TUI Framework

- [ ] 2.1 Create model.go with Bubble Tea Model struct
- [ ] 2.2 Implement Init() method for TUI initialization
- [ ] 2.3 Implement Update() method for event handling
- [ ] 2.4 Implement View() method for rendering sidebar and terminal view
- [ ] 2.5 Add sidebar width configuration support
- [ ] 2.6 Create styles.go with Lipgloss style definitions

## 3. Sidebar Component

- [ ] 3.1 Create sidebar component with project list
- [ ] 3.2 Implement status symbol rendering (● ○ ◐ ✗)
- [ ] 3.3 Add color-coded status indicators using Lipgloss
- [ ] 3.4 Display project name and agent type
- [ ] 3.5 Add visual border between sidebar and terminal view
- [ ] 3.6 Implement legend display at bottom of sidebar

## 4. Terminal View Component

- [ ] 4.1 Create terminal view component using viewport from bubbles
- [ ] 4.2 Implement scrollback buffer management
- [ ] 4.3 Add terminal output rendering with ANSI support
- [ ] 4.4 Handle terminal resize events
- [ ] 4.5 Implement active project highlight

## 5. PTY and Terminal Emulation

- [ ] 5.1 Create pty/ package with PTY management
- [ ] 5.2 Implement CreatePTY(project) function
- [ ] 5.3 Add stdout capture goroutine for each PTY
- [ ] 5.4 Implement stdin forwarding from TUI to PTY
- [ ] 5.5 Add SIGWINCH propagation on terminal resize
- [ ] 5.6 Implement PTY cleanup on TUI exit

## 6. Event Handling

- [ ] 6.1 Implement number key (1-9) project switching
- [ ] 6.2 Add arrow key navigation in sidebar
- [ ] 6.3 Implement Enter key to activate selected project
- [ ] 6.4 Add 'q' key to quit TUI
- [ ] 6.5 Implement Ctrl+A to switch between sidebar and terminal modes
- [ ] 6.6 Add mode indicator (sidebar vs terminal input)

## 7. Agent Integration

- [ ] 7.1 Integrate existing status monitoring from internal/agents
- [ ] 7.2 Connect agent process management to PTY creation
- [ ] 7.3 Handle agent startup for different agent types (opencode, claude, codex)
- [ ] 7.4 Integrate tmux session persistence for background sessions

## 8. Application Entry Point

- [ ] 8.1 Update cmd/amux/main.go to launch TUI for 'start' command
- [ ] 8.2 Add graceful shutdown handling (save state, cleanup PTYs)
- [ ] 8.3 Handle terminal restoration on exit
- [ ] 8.4 Add error handling for PTY creation failures

## 9. Testing and Validation

- [ ] 9.1 Test TUI rendering with multiple projects
- [ ] 9.2 Verify PTY output capture with opencode
- [ ] 9.3 Test keyboard navigation and project switching
- [ ] 9.4 Verify status indicator updates in real-time
- [ ] 9.5 Test terminal resize handling
- [ ] 9.6 Test cleanup on exit

## 10. Documentation and Cleanup

- [ ] 10.1 Update README.md with new TUI usage
- [ ] 10.2 Remove deprecated internal/sidebar/ shell-based implementation
- [ ] 10.3 Update internal/session/ to remove tmux UI dependencies
- [ ] 10.4 Add comments to new TUI code
- [ ] 10.5 Create CHANGELOG entry for breaking change
