// Package sidebar handles sidebar rendering and updates
package sidebar

import (
	"fmt"
	"os"
	"os/exec"
	"strings"
	"time"

	"github.com/user/amux/internal/agents"
	"github.com/user/amux/internal/config"
	"github.com/user/amux/internal/tmux"
)

const (
	orchestratorName = "amux-orchestrator"
	pollInterval     = 2 * time.Second
)

// RenderContent generates the sidebar content
func RenderContent(projects []config.Project) string {
	var sb strings.Builder

	// Header
	sb.WriteString("  AMUX\n")
	sb.WriteString("  ────────────────────\n\n")
	sb.WriteString("  PROJECTS\n\n")

	// Project list
	for _, proj := range projects {
		status := agents.GetAgentStatus(proj)
		info := agents.GetStatusInfo(status)

		sb.WriteString(fmt.Sprintf("  #[fg=%s]%s#[default] %s\n", info.Color, info.Symbol, proj.Name))
		sb.WriteString(fmt.Sprintf("    #[fg=colour244]%s#[default]\n\n", proj.Agent))
	}

	// Legend
	sb.WriteString("\n  ────────────────────\n")
	sb.WriteString("  STATUS LEGEND\n")

	runningInfo := agents.GetStatusInfo(agents.Running)
	idleInfo := agents.GetStatusInfo(agents.Idle)
	reviewInfo := agents.GetStatusInfo(agents.NeedsReview)
	stoppedInfo := agents.GetStatusInfo(agents.Stopped)

	sb.WriteString(fmt.Sprintf("  #[fg=%s]%s#[default] running\n", runningInfo.Color, runningInfo.Symbol))
	sb.WriteString(fmt.Sprintf("  #[fg=%s]%s#[default] idle\n", idleInfo.Color, idleInfo.Symbol))
	sb.WriteString(fmt.Sprintf("  #[fg=%s]%s#[default] needs review\n", reviewInfo.Color, reviewInfo.Symbol))
	sb.WriteString(fmt.Sprintf("  #[fg=%s]%s#[default] stopped\n", stoppedInfo.Color, stoppedInfo.Symbol))

	return sb.String()
}

// Update updates the sidebar pane with current status
func Update(projects []config.Project) error {
	content := RenderContent(projects)

	// Clear sidebar
	tmux.SendKeys(orchestratorName+":0.0", "C-l")

	// Send content line by line
	lines := strings.Split(content, "\n")
	for _, line := range lines {
		escaped := strings.ReplaceAll(line, "\"", "\\\"")
		cmd := exec.Command("tmux", "send-keys", "-t", orchestratorName+":0.0", escaped)
		if err := cmd.Run(); err != nil {
			return fmt.Errorf("sending to sidebar: %w", err)
		}
		cmd = exec.Command("tmux", "send-keys", "-t", orchestratorName+":0.0", "Enter")
		if err := cmd.Run(); err != nil {
			return fmt.Errorf("sending enter: %w", err)
		}
	}

	return nil
}

// StartUpdater starts a goroutine that updates the sidebar periodically
func StartUpdater(projects []config.Project, stop chan bool) {
	ticker := time.NewTicker(pollInterval)
	defer ticker.Stop()

	for {
		select {
		case <-ticker.C:
			if err := Update(projects); err != nil {
				fmt.Fprintf(os.Stderr, "Sidebar update error: %v\n", err)
			}
		case <-stop:
			return
		}
	}
}
