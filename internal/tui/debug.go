// Package tui provides the Bubble Tea TUI implementation
package tui

import (
	"log"
	"os"
	"path/filepath"
)

// Debug logger
var debugLogger *log.Logger

func init() {
	// Create log file in temp directory
	home, _ := os.UserHomeDir()
	logDir := filepath.Join(home, ".local/share/amux")
	os.MkdirAll(logDir, 0755)
	logFile := filepath.Join(logDir, "tui-debug.log")

	f, err := os.OpenFile(logFile, os.O_CREATE|os.O_WRONLY|os.O_APPEND, 0644)
	if err == nil {
		debugLogger = log.New(f, "[TUI] ", log.Ltime|log.Lmicroseconds)
	} else {
		// Fallback to stderr
		debugLogger = log.New(os.Stderr, "[TUI] ", log.Ltime|log.Lmicroseconds)
	}
}

func logf(format string, v ...interface{}) {
	if debugLogger != nil {
		debugLogger.Printf(format, v...)
	}
}

func logMsg(msg string) {
	if debugLogger != nil {
		debugLogger.Println(msg)
	}
}

// BufferedSession wraps a PTY session with output buffering
type BufferedSession struct {
	PTY    interface{} // *pty.Session
	Buffer OutputBuffer
}

// OutputBuffer stores PTY output
type OutputBuffer struct {
	Lines []string
	mu    chan struct{}
}

// NewOutputBuffer creates a new output buffer
func NewOutputBuffer() OutputBuffer {
	return OutputBuffer{
		Lines: make([]string, 0, 1000),
		mu:    make(chan struct{}, 1),
	}
}

// Add adds output to the buffer
func (b *OutputBuffer) Add(output string) {
	select {
	case b.mu <- struct{}{}:
		b.Lines = append(b.Lines, output)
		// Keep only last 1000 lines
		if len(b.Lines) > 1000 {
			b.Lines = b.Lines[len(b.Lines)-1000:]
		}
		<-b.mu
	default:
		// Lock not available, skip
	}
}

// Get returns all buffered output
func (b *OutputBuffer) Get() string {
	select {
	case b.mu <- struct{}{}:
		result := ""
		for _, line := range b.Lines {
			result += line
		}
		<-b.mu
		return result
	default:
		return ""
	}
}

// GetRecent returns the last n lines
func (b *OutputBuffer) GetRecent(n int) []string {
	select {
	case b.mu <- struct{}{}:
		start := len(b.Lines) - n
		if start < 0 {
			start = 0
		}
		result := make([]string, len(b.Lines)-start)
		copy(result, b.Lines[start:])
		<-b.mu
		return result
	default:
		return []string{}
	}
}
