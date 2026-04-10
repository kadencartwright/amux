// Package status provides status file reading and management
package status

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"time"

	"github.com/user/amux/internal/config"
)

// ProjectStatus represents the status of a project
type ProjectStatus struct {
	Project   string    `json:"project"`
	Status    string    `json:"status"`
	Timestamp time.Time `json:"timestamp"`
}

// Status constants
const (
	Running     = "running"
	Idle        = "idle"
	NeedsReview = "needs_review"
	Stopped     = "stopped"
)

// ReadStatus reads the status file for a project
func ReadStatus(projectName string) (*ProjectStatus, error) {
	statusFile := filepath.Join(config.GetStatusDir(), projectName+".json")

	data, err := os.ReadFile(statusFile)
	if err != nil {
		if os.IsNotExist(err) {
			return &ProjectStatus{
				Project: projectName,
				Status:  Stopped,
			}, nil
		}
		return nil, fmt.Errorf("reading status file: %w", err)
	}

	var status ProjectStatus
	if err := json.Unmarshal(data, &status); err != nil {
		// Malformed file - return stopped
		return &ProjectStatus{
			Project: projectName,
			Status:  Stopped,
		}, nil
	}

	// Validate status value
	switch status.Status {
	case Running, Idle, NeedsReview, Stopped:
		// Valid
	default:
		status.Status = Stopped
	}

	return &status, nil
}

// ReadAllStatuses reads all project statuses
func ReadAllStatuses(cfg *config.Config) (map[string]*ProjectStatus, error) {
	statuses := make(map[string]*ProjectStatus)

	for _, proj := range cfg.Projects {
		status, err := ReadStatus(proj.Name)
		if err != nil {
			// Log error but continue with other projects
			status = &ProjectStatus{
				Project: proj.Name,
				Status:  Stopped,
			}
		}
		statuses[proj.Name] = status
	}

	return statuses, nil
}

// WriteStatus writes a status file for a project
func WriteStatus(projectName, status string) error {
	statusDir := config.GetStatusDir()
	if err := os.MkdirAll(statusDir, 0700); err != nil {
		return fmt.Errorf("creating status directory: %w", err)
	}

	statusFile := filepath.Join(statusDir, projectName+".json")

	data := ProjectStatus{
		Project:   projectName,
		Status:    status,
		Timestamp: time.Now(),
	}

	jsonData, err := json.Marshal(data)
	if err != nil {
		return fmt.Errorf("marshaling status: %w", err)
	}

	if err := os.WriteFile(statusFile, jsonData, 0600); err != nil {
		return fmt.Errorf("writing status file: %w", err)
	}

	return nil
}

// GetStatusInfo returns display info for a status
func GetStatusInfo(status string) StatusInfo {
	switch status {
	case Running:
		return StatusInfo{Symbol: "●", Color: "#00ff00", Name: "running"}
	case Idle:
		return StatusInfo{Symbol: "○", Color: "#808080", Name: "idle"}
	case NeedsReview:
		return StatusInfo{Symbol: "◐", Color: "#ffff00", Name: "needs review"}
	case Stopped:
		return StatusInfo{Symbol: "✗", Color: "#ff0000", Name: "stopped"}
	default:
		return StatusInfo{Symbol: "○", Color: "#808080", Name: "idle"}
	}
}

// StatusInfo holds display info for a status
type StatusInfo struct {
	Symbol string
	Color  string
	Name   string
}
