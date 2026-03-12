// Package session handles orchestrator and agent session management
package session

import (
	"fmt"
	"os"
	"os/exec"
	"path/filepath"

	"github.com/user/amux/internal/config"
	"github.com/user/amux/internal/sidebar"
	"github.com/user/amux/internal/tmux"
)

const orchestratorName = "amux-orchestrator"
const agentPrefix = "amux-agent-"

// EnsureOrchestrator creates the orchestrator session if it doesn't exist
func EnsureOrchestrator(cfg *config.Config) error {
	if tmux.HasSession(orchestratorName) {
		return nil
	}

	// Create new session
	if err := tmux.NewSession(orchestratorName, "."); err != nil {
		return fmt.Errorf("creating orchestrator session: %w", err)
	}

	// Split window to create sidebar
	if err := tmux.SplitWindow(orchestratorName+":0", cfg.SidebarWidth); err != nil {
		return fmt.Errorf("creating sidebar: %w", err)
	}

	// Name the panes
	tmux.SelectPane(orchestratorName + ":0.0")
	tmux.SetOption(orchestratorName, "pane-border-status", "top")

	// Select workspace pane by default
	tmux.SelectPane(orchestratorName + ":0.1")

	// Set up key bindings for project switching
	for i, proj := range cfg.Projects {
		if i >= 9 {
			break
		}
		key := fmt.Sprintf("%d", i+1)
		cmd := fmt.Sprintf("run-shell 'amux switch %s'", proj.Name)
		tmux.BindKey(orchestratorName, key, cmd)
	}

	// Bind 'r' to refresh
	tmux.BindKey(orchestratorName, "r", "run-shell 'amux refresh'")

	return nil
}

// EnsureAgent creates an agent session for a project if it doesn't exist
func EnsureAgent(proj config.Project) error {
	sessionName := agentPrefix + proj.Name

	if tmux.HasSession(sessionName) {
		return nil
	}

	// Expand path
	path := proj.Path
	if path[0] == '~' {
		home, _ := os.UserHomeDir()
		path = filepath.Join(home, path[1:])
	}

	// Create agent session
	if err := tmux.NewSession(sessionName, path); err != nil {
		return fmt.Errorf("creating agent session for %s: %w", proj.Name, err)
	}

	// Set session options
	tmux.SetOption(sessionName, "@agent_type", proj.Agent)
	tmux.SetOption(sessionName, "@project_path", path)
	tmux.SetOption(sessionName, "@project_name", proj.Name)

	// Start the agent
	var agentCmd string
	switch proj.Agent {
	case "opencode":
		agentCmd = "opencode"
	case "claude":
		agentCmd = "claude"
	case "codex":
		agentCmd = "codex"
	default:
		agentCmd = proj.Agent
	}

	if err := tmux.SendKeys(sessionName+":0.0", agentCmd); err != nil {
		return fmt.Errorf("starting agent for %s: %w", proj.Name, err)
	}

	return nil
}

// SwitchTo switches the orchestrator to a project
func SwitchTo(proj config.Project) error {
	sessionName := agentPrefix + proj.Name
	targetWindow := orchestratorName + ":" + proj.Name

	// Check if window exists
	cmd := exec.Command("tmux", "list-windows", "-t", orchestratorName)
	output, _ := cmd.CombinedOutput()
	windowExists := false
	for _, line := range splitLines(string(output)) {
		if contains(line, proj.Name) {
			windowExists = true
			break
		}
	}

	// Link window if it doesn't exist
	if !windowExists {
		if err := tmux.LinkWindow(sessionName+":0", targetWindow); err != nil {
			return fmt.Errorf("linking window for %s: %w", proj.Name, err)
		}
	}

	// Select the window
	if err := tmux.SelectWindow(targetWindow); err != nil {
		return fmt.Errorf("selecting window for %s: %w", proj.Name, err)
	}

	// Select workspace pane (not sidebar)
	tmux.SelectPane(targetWindow + ".1")

	return nil
}

// Start initializes and starts the orchestrator
func Start() error {
	cfg, err := config.LoadConfig()
	if err != nil {
		return fmt.Errorf("loading config: %w", err)
	}

	// Ensure status directory exists
	if err := os.MkdirAll(config.GetStatusDir(), 0700); err != nil {
		return fmt.Errorf("creating status directory: %w", err)
	}

	// Create orchestrator session
	if err := EnsureOrchestrator(cfg); err != nil {
		return fmt.Errorf("ensuring orchestrator: %w", err)
	}

	// Create agent sessions
	for _, proj := range cfg.Projects {
		if err := EnsureAgent(proj); err != nil {
			fmt.Fprintf(os.Stderr, "Warning: %v\n", err)
		}
	}

	// Initial sidebar render
	if err := sidebar.Update(cfg.Projects); err != nil {
		fmt.Fprintf(os.Stderr, "Warning: initial sidebar render failed: %v\n", err)
	}

	// Attach to orchestrator
	if err := tmux.Attach(orchestratorName); err != nil {
		// Session is ready but we can't attach from this context
		fmt.Printf("amux orchestrator started: %s\n", orchestratorName)
		fmt.Printf("To attach, run: tmux attach -t %s\n", orchestratorName)
		return nil
	}
	return nil
}

// Stop detaches from the orchestrator
func Stop() error {
	return tmux.Detach()
}

func splitLines(s string) []string {
	var lines []string
	start := 0
	for i := 0; i < len(s); i++ {
		if s[i] == '\n' {
			lines = append(lines, s[start:i])
			start = i + 1
		}
	}
	if start < len(s) {
		lines = append(lines, s[start:])
	}
	return lines
}

func contains(s, substr string) bool {
	for i := 0; i <= len(s)-len(substr); i++ {
		if s[i:i+len(substr)] == substr {
			return true
		}
	}
	return false
}
