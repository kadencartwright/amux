// Package tui provides the Bubble Tea TUI implementation for the amux sidebar
package tui

import (
	"fmt"
	"os/exec"
	"strings"
	"time"

	"github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
	"github.com/user/amux/internal/config"
	"github.com/user/amux/internal/status"
)

// Model represents the TUI state
type Model struct {
	config   *config.Config
	projects []config.Project
	statuses map[string]*status.ProjectStatus
	visible  bool
	width    int
	height   int
}

// Msg types for Bubble Tea
type (
	tickMsg    struct{}
	statusMsg  map[string]*status.ProjectStatus
	toggleMsg  struct{}
	refreshMsg struct{}
)

// NewModel creates a new TUI model
func NewModel(cfg *config.Config) Model {
	return Model{
		config:   cfg,
		projects: cfg.Projects,
		statuses: make(map[string]*status.ProjectStatus),
		visible:  true,
		width:    cfg.SidebarWidth,
		height:   24,
	}
}

// Init initializes the TUI
func (m Model) Init() tea.Cmd {
	return tea.Batch(
		readStatuses(m.config),
		tick(),
	)
}

// Update handles messages and updates the model
func (m Model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
	switch msg := msg.(type) {
	case tea.WindowSizeMsg:
		m.height = msg.Height
		return m, nil

	case tea.KeyMsg:
		switch msg.String() {
		case "f12":
			// Toggle visibility from tmux hotkey
			m.visible = !m.visible
			if !m.visible {
				return m, clearScreen()
			}
			return m, nil

		case "q", "esc":
			if m.visible {
				m.visible = false
				return m, clearScreen()
			}

		case "r":
			if m.visible {
				return m, tea.Batch(
					readStatuses(m.config),
					refreshTmux(),
				)
			}

		case "1", "2", "3", "4", "5", "6", "7", "8", "9":
			if m.visible {
				idx := int(msg.String()[0] - '1')
				if idx < len(m.projects) {
					return m, switchToProject(m.projects[idx])
				}
			}
		}

	case tickMsg:
		return m, tea.Batch(
			readStatuses(m.config),
			tick(),
		)

	case statusMsg:
		m.statuses = msg
		return m, nil

	case toggleMsg:
		m.visible = !m.visible
		if !m.visible {
			return m, clearScreen()
		}
		return m, nil

	case refreshMsg:
		// Triggered by 'r' key, already handled
		return m, nil
	}

	return m, nil
}

// View renders the TUI
func (m Model) View() string {
	if !m.visible {
		return ""
	}

	var sb strings.Builder

	// Header
	headerStyle := lipgloss.NewStyle().Bold(true).Foreground(lipgloss.Color("#ffffff"))
	sb.WriteString(headerStyle.Render("  AMUX"))
	sb.WriteString("\n")
	sb.WriteString("  ────────────────────\n\n")
	sb.WriteString("  PROJECTS\n\n")

	// Project list
	for _, proj := range m.projects {
		projStatus := m.statuses[proj.Name]
		if projStatus == nil {
			projStatus = &status.ProjectStatus{Project: proj.Name, Status: status.Stopped}
		}
		info := status.GetStatusInfo(projStatus.Status)

		// Status symbol with color
		symbolStyle := lipgloss.NewStyle().Foreground(lipgloss.Color(info.Color))
		sb.WriteString(fmt.Sprintf("  %s ", symbolStyle.Render(info.Symbol)))
		sb.WriteString(proj.Name)
		sb.WriteString("\n")

		// Agent type
		agentStyle := lipgloss.NewStyle().Foreground(lipgloss.Color("#808080"))
		sb.WriteString(fmt.Sprintf("    %s\n\n", agentStyle.Render(proj.Agent)))
	}

	// Legend
	sb.WriteString("\n  ────────────────────\n")
	sb.WriteString("  STATUS LEGEND\n")

	statuses := []string{status.Running, status.Idle, status.NeedsReview, status.Stopped}
	for _, s := range statuses {
		info := status.GetStatusInfo(s)
		symbolStyle := lipgloss.NewStyle().Foreground(lipgloss.Color(info.Color))
		sb.WriteString(fmt.Sprintf("  %s %s\n", symbolStyle.Render(info.Symbol), info.Name))
	}

	return sb.String()
}

// tick returns a command that sends a tickMsg after 2 seconds
func tick() tea.Cmd {
	return tea.Tick(time.Second*2, func(t time.Time) tea.Msg {
		return tickMsg{}
	})
}

// readStatuses reads all project statuses
func readStatuses(cfg *config.Config) tea.Cmd {
	return func() tea.Msg {
		statuses, _ := status.ReadAllStatuses(cfg)
		return statusMsg(statuses)
	}
}

// clearScreen returns a command that clears the screen
func clearScreen() tea.Cmd {
	return func() tea.Msg {
		// Send clear screen escape sequence
		fmt.Print("\033[2J\033[H")
		return nil
	}
}

// refreshTmux returns a command that refreshes tmux
func refreshTmux() tea.Cmd {
	return func() tea.Msg {
		cmd := exec.Command("tmux", "refresh-client")
		cmd.Run()
		return refreshMsg{}
	}
}

// switchToProject switches to a project window
func switchToProject(proj config.Project) tea.Cmd {
	return func() tea.Msg {
		windowName := "amux-orchestrator:" + proj.Name
		cmd := exec.Command("tmux", "select-window", "-t", windowName)
		if err := cmd.Run(); err != nil {
			// Silently fail - window might not exist yet
		}
		return nil
	}
}

// ToggleVisibility toggles the sidebar visibility
func ToggleVisibility() tea.Msg {
	return toggleMsg{}
}
