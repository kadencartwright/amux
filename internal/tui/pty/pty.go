// Package pty provides PTY management for terminal emulation
package pty

import (
	"fmt"
	"os"
	"os/exec"
	"syscall"

	"github.com/creack/pty"
)

// Session represents a PTY session
type Session struct {
	PTY    *os.File
	Cmd    *exec.Cmd
	Active bool
}

// Create creates a new PTY and starts the given command in it
func Create(command string, args []string, dir string) (*Session, error) {
	cmd := exec.Command(command, args...)
	cmd.Dir = dir

	// Start the command with a PTY
	ptyFile, err := pty.Start(cmd)
	if err != nil {
		return nil, fmt.Errorf("starting PTY: %w", err)
	}

	return &Session{
		PTY:    ptyFile,
		Cmd:    cmd,
		Active: true,
	}, nil
}

// CreateWithShell creates a PTY with a shell running the given command
func CreateWithShell(command string, dir string) (*Session, error) {
	shell := os.Getenv("SHELL")
	if shell == "" {
		shell = "/bin/bash"
	}

	cmd := exec.Command(shell, "-c", command)
	cmd.Dir = dir

	ptyFile, err := pty.Start(cmd)
	if err != nil {
		return nil, fmt.Errorf("starting PTY with shell: %w", err)
	}

	return &Session{
		PTY:    ptyFile,
		Cmd:    cmd,
		Active: true,
	}, nil
}

// Resize resizes the PTY to the given dimensions
func (s *Session) Resize(rows, cols int) error {
	return pty.Setsize(s.PTY, &pty.Winsize{
		Rows: uint16(rows),
		Cols: uint16(cols),
	})
}

// Close closes the PTY session
func (s *Session) Close() error {
	s.Active = false
	if s.PTY != nil {
		s.PTY.Close()
	}
	if s.Cmd != nil && s.Cmd.Process != nil {
		s.Cmd.Process.Signal(syscall.SIGTERM)
	}
	return nil
}

// Write writes data to the PTY
func (s *Session) Write(p []byte) (n int, err error) {
	return s.PTY.Write(p)
}

// Read reads data from the PTY
func (s *Session) Read(p []byte) (n int, err error) {
	return s.PTY.Read(p)
}
