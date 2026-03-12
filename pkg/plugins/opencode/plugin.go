// Package amux provides integration with the amux orchestrator for opencode.
// This plugin allows opencode to report its status to the amux orchestrator.
package amux

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"time"
)

// Status represents the agent's current state
type Status string

const (
	StatusRunning     Status = "running"
	StatusIdle        Status = "idle"
	StatusNeedsReview Status = "needs_review"
)

// StatusFile represents the JSON structure written to disk
type StatusFile struct {
	Status    string `json:"status"`
	Timestamp string `json:"timestamp"`
}

// Client provides methods to report status to amux
type Client struct {
	project   string
	statusDir string
}

// NewClient creates a new amux client for the given project name
func NewClient(projectName string) (*Client, error) {
	home, err := os.UserHomeDir()
	if err != nil {
		return nil, fmt.Errorf("getting home directory: %w", err)
	}

	statusDir := filepath.Join(home, ".local/share/amux/status")

	// Ensure directory exists
	if err := os.MkdirAll(statusDir, 0700); err != nil {
		return nil, fmt.Errorf("creating status directory: %w", err)
	}

	return &Client{
		project:   projectName,
		statusDir: statusDir,
	}, nil
}

// SetStatus updates the status file with the given status
func (c *Client) SetStatus(status Status) error {
	sf := StatusFile{
		Status:    string(status),
		Timestamp: time.Now().Format(time.RFC3339),
	}

	data, err := json.Marshal(sf)
	if err != nil {
		return fmt.Errorf("marshaling status: %w", err)
	}

	statusPath := filepath.Join(c.statusDir, c.project+".json")
	if err := os.WriteFile(statusPath, data, 0600); err != nil {
		return fmt.Errorf("writing status file: %w", err)
	}

	return nil
}

// Running reports that the agent is actively working
func (c *Client) Running() error {
	return c.SetStatus(StatusRunning)
}

// Idle reports that the agent is idle and waiting
func (c *Client) Idle() error {
	return c.SetStatus(StatusIdle)
}

// NeedsReview reports that the agent has completed work and needs user review
func (c *Client) NeedsReview() error {
	return c.SetStatus(StatusNeedsReview)
}

// Close removes the status file (optional cleanup)
func (c *Client) Close() error {
	statusPath := filepath.Join(c.statusDir, c.project+".json")
	return os.Remove(statusPath)
}
