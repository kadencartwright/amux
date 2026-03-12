## 1. Dependencies and Setup

- [x] 1.1 Add Bubble Tea dependencies to go.mod (bubbletea, lipgloss, bubbles)
- [x] 1.2 Add PTY dependency (creack/pty)
- [x] 1.3 Run go mod tidy to download dependencies
- [x] 1.4 Create internal/tui/ directory structure

## 2. Core TUI Framework

- [x] 2.1 Create model.go with Bubble Tea Model struct
- [x] 2.2 Implement Init() method for TUI initialization
- [x] 2.3 Implement Update() method for event handling
- [x] 2.4 Implement View() method for rendering sidebar and terminal view
- [x] 2.5 Add sidebar width configuration support
- [x] 2.6 Create styles.go with Lipgloss style definitions

## 3. Sidebar Component

- [x] 3.1 Create sidebar component with project list
- [x] 3.2 Implement status symbol rendering (● ○ ◐ ✗)
- [x] 3.3 Add color-coded status indicators using Lipgloss
- [x] 3.4 Display project name and agent type
- [x] 3.5 Add visual border between sidebar and terminal view
- [x] 3.6 Implement legend display at bottom of sidebar

## 4. Terminal View Component

- [x] 4.1 Create terminal view component using viewport from bubbles
- [x] 4.2 Implement scrollback buffer management
- [x] 4.3 Add terminal output rendering with ANSI support
- [x] 4.4 Handle terminal resize events
- [x] 4.5 Implement active project highlight

## 5. PTY and Terminal Emulation

- [x] 5.1 Create pty/ package with PTY management
- [x] 5.2 Implement CreatePTY(project) function
- [x] 5.3 Add stdout capture goroutine for each PTY
- [x] 5.4 Implement stdin forwarding from TUI to PTY
- [x] 5.5 Add SIGWINCH propagation on terminal resize
- [x] 5.6 Implement PTY cleanup on TUI exit

## 6. Event Handling

- [x] 6.1 Implement number key (1-9) project switching
- [x] 6.2 Add arrow key navigation in sidebar
- [x] 6.3 Implement Enter key to activate selected project
- [x] 6.4 Add 'q' key to quit TUI
- [x] 6.5 Implement Ctrl+A to switch between sidebar and terminal modes
- [x] 6.6 Add mode indicator (sidebar vs terminal input)

## 7. Agent Integration

- [x] 7.1 Integrate existing status monitoring from internal/agents
- [x] 7.2 Connect agent process management to PTY creation
- [x] 7.3 Handle agent startup for different agent types (opencode, claude, codex)
- [x] 7.4 Integrate tmux session persistence for background sessions

## 8. Application Entry Point

- [x] 8.1 Update cmd/amux/main.go to launch TUI for 'start' command
- [x] 8.2 Add graceful shutdown handling (save state, cleanup PTYs)
- [x] 8.3 Handle terminal restoration on exit
- [x] 8.4 Add error handling for PTY creation failures

## 9. Testing and Validation

- [x] 9.1 Test TUI rendering with multiple projects
- [x] 9.2 Verify PTY output capture with opencode
- [x] 9.3 Test keyboard navigation and project switching
- [x] 9.4 Verify status indicator updates in real-time
- [x] 9.5 Test terminal resize handling
- [x] 9.6 Test cleanup on exit

## 10. Documentation and Cleanup

- [x] 10.1 Update README.md with new TUI usage
- [x] 10.2 Remove deprecated internal/sidebar/ shell-based implementation
- [x] 10.3 Update internal/session/ to remove tmux UI dependencies
- [x] 10.4 Add comments to new TUI code
- [x] 10.5 Create CHANGELOG entry for breaking change
