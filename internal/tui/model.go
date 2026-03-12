// Package tui provides the Bubble Tea TUI implementation
package tui

import (
	"fmt"
	"strings"
	"time"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
	"github.com/user/amux/internal/agents"
	"github.com/user/amux/internal/config"
	"github.com/user/amux/internal/tui/pty"
)

// Mode represents the TUI input mode
type Mode int

const (
	// ModeTerminal means keystrokes go to the active PTY
	ModeTerminal Mode = iota
	// ModeSidebar means keystrokes navigate the sidebar
	ModeSidebar
)

// Model is the Bubble Tea model for the TUI
type Model struct {
	// Configuration
	config       *config.Config
	styles       Styles
	sidebarWidth int

	// State
	mode          Mode
	activeIndex   int
	selectedIndex int
	projects      []ProjectState

	// Dimensions
	width  int
	height int

	// Status polling
	lastStatusUpdate time.Time

	// Error display
	errorMsg string
}

// ProjectState holds the state for a single project
type ProjectState struct {
	Project config.Project
	Status  agents.Status
	PTY     *pty.Session
	Output  OutputBuffer
}

// Msg types for Bubble Tea

// statusUpdateMsg is sent when project statuses need to be refreshed
type statusUpdateMsg time.Time

// ptyOutputMsg is sent when a PTY produces output
type ptyOutputMsg struct {
	ProjectName string
	Output      string
}

// ptyStartedMsg is sent when a PTY session is successfully created
type ptyStartedMsg struct {
	Index   int
	Session *pty.Session
}

// ptyErrorMsg is sent when PTY creation fails
type ptyErrorMsg struct {
	Err error
}

// NewModel creates a new TUI model
func NewModel(cfg *config.Config) Model {
	logf("NewModel called with %d projects", len(cfg.Projects))

	// Initialize project states
	projects := make([]ProjectState, len(cfg.Projects))
	for i, proj := range cfg.Projects {
		projects[i] = ProjectState{
			Project: proj,
			Status:  agents.Stopped,
			Output:  NewOutputBuffer(),
		}
		logf("Initialized project %d: %s (%s)", i, proj.Name, proj.Agent)
	}

	sidebarWidth := cfg.SidebarWidth
	if sidebarWidth == 0 {
		sidebarWidth = 25
	}

	return Model{
		config:        cfg,
		styles:        DefaultStyles(),
		sidebarWidth:  sidebarWidth,
		mode:          ModeTerminal,
		activeIndex:   0,
		selectedIndex: 0,
		projects:      projects,
	}
}

// Init implements tea.Model
func (m Model) Init() tea.Cmd {
	return tea.Batch(
		tea.EnterAltScreen,
		m.startStatusPolling(),
	)
}

// Cleanup closes all PTY sessions when the TUI exits
func (m *Model) Cleanup() {
	for i := range m.projects {
		if m.projects[i].PTY != nil {
			m.projects[i].PTY.Close()
			m.projects[i].PTY = nil
		}
	}
}

// startStatusPolling returns a command that polls for status updates
func (m Model) startStatusPolling() tea.Cmd {
	return tea.Tick(2*time.Second, func(t time.Time) tea.Msg {
		return statusUpdateMsg(t)
	})
}

// getStatusInfo gets the display info for a status
func getStatusInfo(status agents.Status) (string, lipgloss.Style) {
	switch status {
	case agents.Running:
		return SymbolRunning, lipgloss.NewStyle().Foreground(lipgloss.Color("46"))
	case agents.Idle:
		return SymbolIdle, lipgloss.NewStyle().Foreground(lipgloss.Color("244"))
	case agents.NeedsReview:
		return SymbolNeedsReview, lipgloss.NewStyle().Foreground(lipgloss.Color("226"))
	case agents.Stopped:
		return SymbolStopped, lipgloss.NewStyle().Foreground(lipgloss.Color("196"))
	default:
		return SymbolStopped, lipgloss.NewStyle().Foreground(lipgloss.Color("196"))
	}
}

// UpdateStatus updates the status of all projects
func (m *Model) UpdateStatus() {
	for i := range m.projects {
		m.projects[i].Status = agents.GetAgentStatus(m.projects[i].Project)
	}
	m.lastStatusUpdate = time.Now()
}

// renderSidebar renders the sidebar content
func (m Model) renderSidebar() string {
	var sb strings.Builder

	// Header
	sb.WriteString(m.styles.Header.Render("  AMUX"))
	sb.WriteString("\n")
	sb.WriteString(m.styles.Header.Render("  " + strings.Repeat("─", m.sidebarWidth-4)))
	sb.WriteString("\n\n")
	sb.WriteString(m.styles.Header.Render("  PROJECTS"))
	sb.WriteString("\n\n")

	// Project list
	for i, proj := range m.projects {
		symbol, style := getStatusInfo(proj.Status)

		// Build project line
		var line strings.Builder

		// Selection indicator
		if m.mode == ModeSidebar && i == m.selectedIndex {
			line.WriteString("> ")
		} else {
			line.WriteString("  ")
		}

		// Status symbol
		line.WriteString(style.Render(symbol))
		line.WriteString(" ")

		// Project name with active highlighting
		nameStyle := m.styles.ProjectName
		if i == m.activeIndex {
			nameStyle = m.styles.ActiveProject
		}
		line.WriteString(nameStyle.Render(proj.Project.Name))
		line.WriteString("\n")

		// Agent type
		line.WriteString(m.styles.ProjectAgent.Render(proj.Project.Agent))
		line.WriteString("\n\n")

		sb.WriteString(line.String())
	}

	// Legend
	sb.WriteString("\n")
	sb.WriteString(m.styles.Header.Render("  " + strings.Repeat("─", m.sidebarWidth-4)))
	sb.WriteString("\n")
	sb.WriteString(m.styles.Legend.Render("  STATUS LEGEND"))
	sb.WriteString("\n")

	runningInfo := agents.GetStatusInfo(agents.Running)
	idleInfo := agents.GetStatusInfo(agents.Idle)
	reviewInfo := agents.GetStatusInfo(agents.NeedsReview)
	stoppedInfo := agents.GetStatusInfo(agents.Stopped)

	sb.WriteString(fmt.Sprintf("  %s running\n",
		lipgloss.NewStyle().Foreground(lipgloss.Color(runningInfo.Color)).Render(runningInfo.Symbol)))
	sb.WriteString(fmt.Sprintf("  %s idle\n",
		lipgloss.NewStyle().Foreground(lipgloss.Color(idleInfo.Color)).Render(idleInfo.Symbol)))
	sb.WriteString(fmt.Sprintf("  %s needs review\n",
		lipgloss.NewStyle().Foreground(lipgloss.Color(reviewInfo.Color)).Render(reviewInfo.Symbol)))
	sb.WriteString(fmt.Sprintf("  %s stopped\n",
		lipgloss.NewStyle().Foreground(lipgloss.Color(stoppedInfo.Color)).Render(stoppedInfo.Symbol)))

	return sb.String()
}

// renderTerminalView renders the terminal view
func (m Model) renderTerminalView() string {
	if len(m.projects) == 0 {
		return m.styles.TerminalView.Render("No projects configured")
	}

	activeProject := m.projects[m.activeIndex]

	var content strings.Builder

	// Terminal header
	content.WriteString(fmt.Sprintf("Project: %s (%s)\n",
		activeProject.Project.Name,
		activeProject.Project.Agent))
	content.WriteString(strings.Repeat("─", m.width-m.sidebarWidth-2))
	content.WriteString("\n\n")

	// Show error message if present
	if m.errorMsg != "" {
		content.WriteString(fmt.Sprintf("Error: %s\n\n", m.errorMsg))
	}

	// PTY output
	if activeProject.PTY != nil {
		// Display buffered output
		output := activeProject.Output.Get()
		if output == "" {
			content.WriteString("[Terminal started - output will appear here]\n")
		} else {
			content.WriteString(output)
		}
		logf("Rendering %d bytes of output for project %s", len(output), activeProject.Project.Name)
	} else {
		content.WriteString("[Press Enter to start terminal session]\n")
	}

	// Mode indicator
	content.WriteString("\n")
	if m.mode == ModeSidebar {
		content.WriteString(m.styles.ModeIndicator.Render(" SIDEBAR MODE "))
	} else {
		content.WriteString(m.styles.ModeIndicator.Render(" TERMINAL MODE "))
	}
	content.WriteString(" Ctrl+A: switch mode | 1-9: switch project | q: quit")

	return m.styles.TerminalView.Render(content.String())
}
