// Package tui provides the Bubble Tea TUI implementation
package tui

import (
	"github.com/charmbracelet/lipgloss"
)

// Styles holds all lipgloss styles used in the TUI
type Styles struct {
	Sidebar       lipgloss.Style
	SidebarBorder lipgloss.Style
	TerminalView  lipgloss.Style
	Header        lipgloss.Style
	ProjectName   lipgloss.Style
	ProjectAgent  lipgloss.Style
	ActiveProject lipgloss.Style
	Legend        lipgloss.Style
	ModeIndicator lipgloss.Style
	StatusRunning lipgloss.Style
	StatusIdle    lipgloss.Style
	StatusReview  lipgloss.Style
	StatusStopped lipgloss.Style
}

// DefaultStyles creates the default style configuration
func DefaultStyles() Styles {
	return Styles{
		Sidebar: lipgloss.NewStyle().
			Padding(0, 1),

		SidebarBorder: lipgloss.NewStyle().
			BorderStyle(lipgloss.NormalBorder()).
			BorderRight(true).
			BorderForeground(lipgloss.Color("240")),

		TerminalView: lipgloss.NewStyle().
			Padding(0, 1),

		Header: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("39")).
			PaddingBottom(1),

		ProjectName: lipgloss.NewStyle().
			Foreground(lipgloss.Color("252")),

		ProjectAgent: lipgloss.NewStyle().
			Foreground(lipgloss.Color("244")).
			PaddingLeft(2),

		ActiveProject: lipgloss.NewStyle().
			Background(lipgloss.Color("238")).
			Bold(true),

		Legend: lipgloss.NewStyle().
			Foreground(lipgloss.Color("248")).
			MarginTop(1),

		ModeIndicator: lipgloss.NewStyle().
			Foreground(lipgloss.Color("39")).
			Background(lipgloss.Color("236")).
			Padding(0, 1),

		StatusRunning: lipgloss.NewStyle().
			Foreground(lipgloss.Color("46")),

		StatusIdle: lipgloss.NewStyle().
			Foreground(lipgloss.Color("244")),

		StatusReview: lipgloss.NewStyle().
			Foreground(lipgloss.Color("226")),

		StatusStopped: lipgloss.NewStyle().
			Foreground(lipgloss.Color("196")),
	}
}

// Status symbols
const (
	SymbolRunning     = "●"
	SymbolIdle        = "○"
	SymbolNeedsReview = "◐"
	SymbolStopped     = "✗"
)
