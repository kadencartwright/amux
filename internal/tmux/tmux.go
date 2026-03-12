// Package tmux provides wrappers for tmux commands
package tmux

import (
	"fmt"
	"os/exec"
)

// HasSession checks if a tmux session exists
func HasSession(session string) bool {
	cmd := exec.Command("tmux", "has-session", "-t", "="+session)
	err := cmd.Run()
	return err == nil
}

// NewSession creates a new detached tmux session
func NewSession(name, dir string) error {
	cmd := exec.Command("tmux", "new-session", "-d", "-s", name, "-c", dir)
	if output, err := cmd.CombinedOutput(); err != nil {
		return fmt.Errorf("tmux new-session: %w (output: %s)", err, string(output))
	}
	return nil
}

// SplitWindow creates a horizontal split
func SplitWindow(target string, width int) error {
	cmd := exec.Command("tmux", "split-window", "-h", "-t", target, "-l", fmt.Sprintf("%d", width))
	if output, err := cmd.CombinedOutput(); err != nil {
		return fmt.Errorf("tmux split-window: %w (output: %s)", err, string(output))
	}
	return nil
}

// LinkWindow links a window from one session to another
func LinkWindow(src, dst string) error {
	cmd := exec.Command("tmux", "link-window", "-s", src, "-t", dst)
	if output, err := cmd.CombinedOutput(); err != nil {
		return fmt.Errorf("tmux link-window: %w (output: %s)", err, string(output))
	}
	return nil
}

// SelectWindow selects a window
func SelectWindow(target string) error {
	cmd := exec.Command("tmux", "select-window", "-t", target)
	if output, err := cmd.CombinedOutput(); err != nil {
		return fmt.Errorf("tmux select-window: %w (output: %s)", err, string(output))
	}
	return nil
}

// SelectPane selects a pane
func SelectPane(target string) error {
	cmd := exec.Command("tmux", "select-pane", "-t", target)
	if output, err := cmd.CombinedOutput(); err != nil {
		return fmt.Errorf("tmux select-pane: %w (output: %s)", err, string(output))
	}
	return nil
}

// SendKeys sends keys to a tmux pane
func SendKeys(target, keys string) error {
	cmd := exec.Command("tmux", "send-keys", "-t", target, keys, "Enter")
	if output, err := cmd.CombinedOutput(); err != nil {
		return fmt.Errorf("tmux send-keys: %w (output: %s)", err, string(output))
	}
	return nil
}

// CapturePane captures the content of a pane
func CapturePane(target string, lines int) (string, error) {
	cmd := exec.Command("tmux", "capture-pane", "-t", target, "-p", "-S", fmt.Sprintf("-%d", lines))
	output, err := cmd.CombinedOutput()
	if err != nil {
		return "", fmt.Errorf("tmux capture-pane: %w", err)
	}
	return string(output), nil
}

// ListPanes lists panes in a session and returns the first pane's PID
func ListPanes(session string) (int, error) {
	cmd := exec.Command("tmux", "list-panes", "-t", session, "-F", "#{pane_pid}")
	output, err := cmd.CombinedOutput()
	if err != nil {
		return 0, fmt.Errorf("tmux list-panes: %w", err)
	}

	lines := splitLines(string(output))
	if len(lines) == 0 || lines[0] == "" {
		return 0, fmt.Errorf("no panes found in session %s", session)
	}

	var pid int
	_, err = fmt.Sscanf(lines[0], "%d", &pid)
	if err != nil {
		return 0, fmt.Errorf("parsing pane PID: %w", err)
	}

	return pid, nil
}

// Attach attaches to a session
func Attach(session string) error {
	cmd := exec.Command("tmux", "attach", "-t", session)
	cmd.Stdin = nil
	cmd.Stdout = nil
	cmd.Stderr = nil
	return cmd.Run()
}

// Detach detaches from a session
func Detach() error {
	cmd := exec.Command("tmux", "detach")
	return cmd.Run()
}

// BindKey binds a key in a session
func BindKey(session, key, command string) error {
	cmd := exec.Command("tmux", "bind-key", "-t", session, key, command)
	if output, err := cmd.CombinedOutput(); err != nil {
		return fmt.Errorf("tmux bind-key: %w (output: %s)", err, string(output))
	}
	return nil
}

// SetOption sets a tmux option
func SetOption(session, option, value string) error {
	var cmd *exec.Cmd
	if session == "" {
		cmd = exec.Command("tmux", "set-option", option, value)
	} else {
		cmd = exec.Command("tmux", "set-option", "-t", session, option, value)
	}
	if output, err := cmd.CombinedOutput(); err != nil {
		return fmt.Errorf("tmux set-option: %w (output: %s)", err, string(output))
	}
	return nil
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
