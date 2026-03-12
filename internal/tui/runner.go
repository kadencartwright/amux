package tui

import (
	"os"
	"os/signal"
	"syscall"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/user/amux/internal/config"
)

// Run starts the TUI application with graceful shutdown
func Run(cfg *config.Config) error {
	model := NewModel(cfg)

	// Set up signal handling for graceful shutdown
	sigChan := make(chan os.Signal, 1)
	signal.Notify(sigChan, syscall.SIGINT, syscall.SIGTERM)

	p := tea.NewProgram(
		model,
		tea.WithAltScreen(),
		tea.WithMouseCellMotion(),
	)

	// Run the program in a goroutine
	errChan := make(chan error, 1)
	go func() {
		_, err := p.Run()
		errChan <- err
	}()

	// Wait for either program exit or signal
	select {
	case err := <-errChan:
		return err
	case <-sigChan:
		// Handle signal by quitting the program gracefully
		p.Quit()
		<-errChan // Wait for program to exit
		return nil
	}
}
