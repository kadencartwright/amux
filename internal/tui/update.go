package tui

import (
	"fmt"
	"os"
	"path/filepath"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
	"github.com/user/amux/internal/config"
	"github.com/user/amux/internal/tui/pty"
)

// Update implements tea.Model
func (m Model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
	logf("Update called with msg type: %T", msg)

	switch msg := msg.(type) {

	case tea.WindowSizeMsg:
		logf("Window size: %dx%d", msg.Width, msg.Height)
		m.width = msg.Width
		m.height = msg.Height

	case tea.KeyMsg:
		logf("Key pressed: %s (type: %v)", msg.String(), msg.Type)
		return m.handleKeyPress(msg)

	case statusUpdateMsg:
		logMsg("Status update tick")
		m.UpdateStatus()
		return m, m.startStatusPolling()

	case ptyOutputMsg:
		logf("PTY output received for project: %s, length: %d", msg.ProjectName, len(msg.Output))
		// Update terminal output for the project
		for i := range m.projects {
			if m.projects[i].Project.Name == msg.ProjectName {
				// Store the output in the buffer
				m.projects[i].Output.Add(msg.Output)
				logf("Stored output in buffer for project %s, buffer now has %d lines", msg.ProjectName, len(m.projects[i].Output.Lines))
				// Continue reading
				return m, m.readPTYOutput(i)
			}
		}

	case ptyStartedMsg:
		logf("PTY started successfully for project index: %d", msg.Index)
		// PTY session started successfully
		if msg.Index < len(m.projects) {
			// Important: reassign to slice to ensure the change persists
			project := m.projects[msg.Index]
			project.PTY = msg.Session
			// Initialize output buffer if needed
			if len(project.Output.Lines) == 0 && cap(project.Output.Lines) == 0 {
				project.Output = NewOutputBuffer()
			}
			m.projects[msg.Index] = project
			m.errorMsg = "" // Clear the "Starting..." message
			logf("PTY session stored for project: %s", m.projects[msg.Index].Project.Name)

			// Start reading PTY output in background
			return m, m.readPTYOutput(msg.Index)
		}
		return m, nil

	case ptyErrorMsg:
		logf("PTY error: %v", msg.Err)
		// Display the error message
		m.errorMsg = msg.Err.Error()
		return m, nil
	}

	return m, nil
}

// handleKeyPress handles keyboard input
func (m Model) handleKeyPress(msg tea.KeyMsg) (tea.Model, tea.Cmd) {

	// Global keys (work in any mode)
	switch msg.String() {
	case "ctrl+c":
		logMsg("Ctrl+C pressed - quitting")
		return m, tea.Quit

	case "ctrl+a":
		// Toggle between sidebar and terminal mode
		if m.mode == ModeTerminal {
			m.mode = ModeSidebar
			logMsg("Switched to SIDEBAR mode")
		} else {
			m.mode = ModeTerminal
			logMsg("Switched to TERMINAL mode")
		}
		return m, nil
	}

	// Mode-specific handling
	switch m.mode {
	case ModeSidebar:
		return m.handleSidebarKeys(msg)
	case ModeTerminal:
		return m.handleTerminalKeys(msg)
	}

	return m, nil
}

// handleSidebarKeys handles keys when in sidebar mode
func (m Model) handleSidebarKeys(msg tea.KeyMsg) (tea.Model, tea.Cmd) {
	switch msg.String() {
	case "q":
		logMsg("'q' pressed in sidebar - quitting")
		return m, tea.Quit

	case "up", "k":
		if m.selectedIndex > 0 {
			m.selectedIndex--
			logf("Selected index: %d", m.selectedIndex)
		}
		return m, nil

	case "down", "j":
		if m.selectedIndex < len(m.projects)-1 {
			m.selectedIndex++
			logf("Selected index: %d", m.selectedIndex)
		}
		return m, nil

	case "enter":
		// Activate selected project
		if m.selectedIndex < len(m.projects) {
			logf("Activating project at index: %d", m.selectedIndex)
			m.activeIndex = m.selectedIndex
			m.mode = ModeTerminal
			return m.startActiveProjectPTY()
		}
		return m, nil

	case "1", "2", "3", "4", "5", "6", "7", "8", "9":
		// Switch to project by number
		idx := int(msg.String()[0] - '1')
		logf("Number key pressed: %d", idx+1)
		if idx < len(m.projects) {
			m.activeIndex = idx
			m.selectedIndex = idx
			m.mode = ModeTerminal
			return m.startActiveProjectPTY()
		}
		return m, nil
	}

	return m, nil
}

// handleTerminalKeys handles keys when in terminal mode
func (m Model) handleTerminalKeys(msg tea.KeyMsg) (tea.Model, tea.Cmd) {
	switch msg.String() {
	case "1", "2", "3", "4", "5", "6", "7", "8", "9":
		// Switch to project by number
		idx := int(msg.String()[0] - '1')
		if idx < len(m.projects) && idx != m.activeIndex {
			m.activeIndex = idx
			logf("Switching to project index: %d", idx)
			return m.startActiveProjectPTY()
		}
		return m, nil

	case "enter":
		// Check if we need to start a PTY session
		if m.activeIndex < len(m.projects) && m.projects[m.activeIndex].PTY == nil {
			logf("Enter pressed, starting PTY for project: %s", m.projects[m.activeIndex].Project.Name)
			m.errorMsg = "Starting terminal session..."
			return m.startActiveProjectPTY()
		}
		// Otherwise forward to PTY
		return m.forwardToPTY(msg)

	case "q":
		// In terminal mode, forward 'q' to PTY unless it's a special case
		return m.forwardToPTY(msg)

	default:
		// Forward all other keys to the active PTY
		return m.forwardToPTY(msg)
	}
}

// forwardToPTY forwards a keypress to the active PTY
func (m Model) forwardToPTY(msg tea.KeyMsg) (tea.Model, tea.Cmd) {
	if m.activeIndex >= len(m.projects) {
		return m, nil
	}

	project := &m.projects[m.activeIndex]
	if project.PTY != nil {
		// Convert key to bytes and write to PTY
		input := keyToBytes(msg)
		if len(input) > 0 {
			logf("Forwarding %d bytes to PTY", len(input))
			project.PTY.Write(input)
		}
	} else {
		logf("Cannot forward key - PTY is nil for project: %s", project.Project.Name)
	}

	return m, nil
}

// keyToBytes converts a tea.KeyMsg to bytes for PTY input
func keyToBytes(msg tea.KeyMsg) []byte {
	switch msg.Type {
	case tea.KeyRunes:
		return []byte(string(msg.Runes))
	case tea.KeySpace:
		return []byte{' '}
	case tea.KeyEnter:
		return []byte{'\n'}
	case tea.KeyBackspace:
		return []byte{0x7f} // DEL character
	case tea.KeyTab:
		return []byte{'\t'}
	case tea.KeyEsc:
		return []byte{0x1b}
	case tea.KeyUp:
		return []byte{0x1b, '[', 'A'}
	case tea.KeyDown:
		return []byte{0x1b, '[', 'B'}
	case tea.KeyRight:
		return []byte{0x1b, '[', 'C'}
	case tea.KeyLeft:
		return []byte{0x1b, '[', 'D'}
	case tea.KeyHome:
		return []byte{0x1b, '[', 'H'}
	case tea.KeyEnd:
		return []byte{0x1b, '[', 'F'}
	case tea.KeyDelete:
		return []byte{0x1b, '[', '3', '~'}
	case tea.KeyPgUp:
		return []byte{0x1b, '[', '5', '~'}
	case tea.KeyPgDown:
		return []byte{0x1b, '[', '6', '~'}
	default:
		if msg.Alt {
			return append([]byte{0x1b}, []byte(msg.String())...)
		}
		return []byte(msg.String())
	}
}

// startActiveProjectPTY starts a PTY session for the active project
func (m Model) startActiveProjectPTY() (tea.Model, tea.Cmd) {
	if m.activeIndex >= len(m.projects) {
		logf("Cannot start PTY - invalid index: %d", m.activeIndex)
		return m, nil
	}

	project := m.projects[m.activeIndex]

	// If PTY already exists, just activate it
	if project.PTY != nil {
		logf("PTY already exists for project: %s", project.Project.Name)
		return m, nil
	}

	// Capture the index for the closure
	index := m.activeIndex

	// Start a new PTY session
	cmd := func() tea.Msg {
		logf("Creating PTY for project: %s (agent: %s, path: %s)",
			project.Project.Name, project.Project.Agent, project.Project.Path)
		session, err := m.createPTYForProject(project.Project)
		if err != nil {
			logf("Failed to create PTY: %v", err)
			return ptyErrorMsg{Err: err}
		}
		logf("PTY created successfully for project: %s", project.Project.Name)
		return ptyStartedMsg{Index: index, Session: session}
	}

	return m, cmd
}

// readPTYOutput creates a command that reads from the PTY
func (m Model) readPTYOutput(index int) tea.Cmd {
	return func() tea.Msg {
		if index >= len(m.projects) || m.projects[index].PTY == nil {
			logf("Cannot read PTY - index: %d, has PTY: %v", index, index < len(m.projects) && m.projects[index].PTY != nil)
			return nil
		}

		// Read a chunk of output
		buf := make([]byte, 1024)
		logf("Reading from PTY for project: %s", m.projects[index].Project.Name)
		n, err := m.projects[index].PTY.Read(buf)
		if err != nil {
			logf("PTY read error: %v", err)
			return nil // PTY closed or error
		}

		logf("Read %d bytes from PTY", n)
		return ptyOutputMsg{
			ProjectName: m.projects[index].Project.Name,
			Output:      string(buf[:n]),
		}
	}
}

// createPTYForProject creates a PTY session for a project
func (m Model) createPTYForProject(project config.Project) (*pty.Session, error) {
	logf("createPTYForProject called for: %s", project.Name)

	// Determine agent command
	var agentCmd string
	switch project.Agent {
	case "opencode":
		agentCmd = "opencode"
	case "claude":
		agentCmd = "claude"
	case "codex":
		agentCmd = "codex"
	default:
		agentCmd = project.Agent
	}
	logf("Agent command: %s", agentCmd)

	// Expand path (handle ~)
	path := project.Path
	if len(path) > 0 && path[0] == '~' {
		home, err := os.UserHomeDir()
		if err != nil {
			return nil, fmt.Errorf("expanding home directory: %w", err)
		}
		path = filepath.Join(home, path[1:])
	}
	logf("Expanded path: %s", path)

	// Check if path exists
	if _, err := os.Stat(path); err != nil {
		return nil, fmt.Errorf("project path not found: %s", path)
	}
	logf("Path exists: %s", path)

	// Create PTY session
	logf("Calling pty.CreateWithShell with command: %s, dir: %s", agentCmd, path)
	session, err := pty.CreateWithShell(agentCmd, path)
	if err != nil {
		return nil, fmt.Errorf("creating PTY: %w", err)
	}
	logf("PTY session created successfully")
	return session, nil
}

// View implements tea.Model
func (m Model) View() string {
	if m.width == 0 || m.height == 0 {
		return "Loading..."
	}

	// Render sidebar
	sidebarContent := m.renderSidebar()
	sidebar := m.styles.SidebarBorder.
		Width(m.sidebarWidth).
		Height(m.height).
		Render(sidebarContent)

	// Render terminal view
	terminalContent := m.renderTerminalView()
	terminalWidth := m.width - m.sidebarWidth - 1
	if terminalWidth < 0 {
		terminalWidth = 0
	}
	terminal := lipgloss.NewStyle().
		Width(terminalWidth).
		Height(m.height).
		Render(terminalContent)

	// Join sidebar and terminal horizontally
	return lipgloss.JoinHorizontal(lipgloss.Top, sidebar, terminal)
}
