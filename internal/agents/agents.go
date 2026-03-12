// Package agents handles agent status detection
package agents

import (
	"encoding/json"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"time"

	"github.com/user/amux/internal/config"
	"github.com/user/amux/internal/tmux"
)

// Status represents the state of an agent
type Status int

const (
	Stopped Status = iota
	Idle
	Running
	NeedsReview
)

func (s Status) String() string {
	switch s {
	case Stopped:
		return "stopped"
	case Idle:
		return "idle"
	case Running:
		return "running"
	case NeedsReview:
		return "needs_review"
	default:
		return "unknown"
	}
}

// StatusInfo holds symbol and color for a status
type StatusInfo struct {
	Symbol string
	Color  string
}

var (
	// StatusSymbols maps status to display symbol
	StatusSymbols = map[Status]string{
		Stopped:     "✗",
		Idle:        "○",
		Running:     "●",
		NeedsReview: "◐",
	}

	// StatusColors maps status to tmux color code
	StatusColors = map[Status]string{
		Stopped:     "colour196",
		Idle:        "colour244",
		Running:     "colour46",
		NeedsReview: "colour226",
	}

	statusTimeout = 30 * time.Second
)

// GetStatusInfo returns the symbol and color for a status
func GetStatusInfo(status Status) StatusInfo {
	return StatusInfo{
		Symbol: StatusSymbols[status],
		Color:  StatusColors[status],
	}
}

// StatusFile represents the JSON structure of a status file
type StatusFile struct {
	Status    string `json:"status"`
	Timestamp string `json:"timestamp"`
}

// GetAgentStatus determines the status of a project
func GetAgentStatus(proj config.Project) Status {
	sessionName := "agent-" + proj.Name

	// Check if session exists
	if !tmux.HasSession(sessionName) {
		return Stopped
	}

	// For opencode, try to read status file first
	if proj.Agent == "opencode" {
		if status, err := ReadStatusFile(proj.Name); err == nil {
			return status
		}
	}

	// Get pane PID
	pid, err := tmux.ListPanes(sessionName)
	if err != nil {
		return Idle
	}

	// Check if agent process is running
	if IsAgentRunning(pid, proj.Agent) {
		return Running
	}

	// For non-opencode agents, check output patterns
	if proj.Agent != "opencode" {
		output, err := tmux.CapturePane(sessionName+":0.0", 20)
		if err == nil && NeedsReviewPattern(output) {
			return NeedsReview
		}
	}

	return Idle
}

// IsAgentRunning checks if the agent process is still running
func IsAgentRunning(pid int, agentType string) bool {
	cmd := exec.Command("ps", "-p", fmt.Sprintf("%d", pid), "-o", "command=")
	output, err := cmd.CombinedOutput()
	if err != nil {
		return false
	}

	command := strings.ToLower(string(output))

	switch agentType {
	case "opencode":
		return strings.Contains(command, "opencode")
	case "claude":
		return strings.Contains(command, "claude")
	case "codex":
		return strings.Contains(command, "codex")
	default:
		return strings.Contains(command, agentType)
	}
}

// NeedsReviewPattern checks if output indicates the agent needs review
func NeedsReviewPattern(output string) bool {
	patterns := []string{
		"waiting for your response",
		"waiting for input",
		"please verify",
		"needs review",
		"ready for your review",
	}

	lower := strings.ToLower(output)
	for _, pattern := range patterns {
		if strings.Contains(lower, pattern) {
			return true
		}
	}
	return false
}

// ReadStatusFile reads the status from a project's status file
func ReadStatusFile(project string) (Status, error) {
	statusPath := filepath.Join(config.GetStatusDir(), project+".json")

	data, err := os.ReadFile(statusPath)
	if err != nil {
		return Idle, err
	}

	var sf StatusFile
	if err := json.Unmarshal(data, &sf); err != nil {
		return Idle, err
	}

	// Check staleness
	timestamp, err := time.Parse(time.RFC3339, sf.Timestamp)
	if err != nil {
		return Idle, fmt.Errorf("parsing timestamp: %w", err)
	}

	if time.Since(timestamp) > statusTimeout {
		return Idle, fmt.Errorf("status file is stale")
	}

	switch sf.Status {
	case "running":
		return Running, nil
	case "idle":
		return Idle, nil
	case "needs_review":
		return NeedsReview, nil
	default:
		return Idle, fmt.Errorf("unknown status: %s", sf.Status)
	}
}

// WriteStatusFile writes a status update for a project
func WriteStatusFile(project string, status Status) error {
	statusDir := config.GetStatusDir()
	if err := os.MkdirAll(statusDir, 0700); err != nil {
		return fmt.Errorf("creating status directory: %w", err)
	}

	statusPath := filepath.Join(statusDir, project+".json")

	sf := StatusFile{
		Status:    status.String(),
		Timestamp: time.Now().Format(time.RFC3339),
	}

	data, err := json.Marshal(sf)
	if err != nil {
		return fmt.Errorf("marshaling status: %w", err)
	}

	if err := os.WriteFile(statusPath, data, 0600); err != nil {
		return fmt.Errorf("writing status file: %w", err)
	}

	return nil
}
