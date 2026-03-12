package main

import (
	"fmt"
	"os"

	"github.com/user/amux/internal/config"
	"github.com/user/amux/internal/tui"
)

func main() {
	if len(os.Args) < 2 {
		showHelp()
		os.Exit(1)
	}

	cmd := os.Args[1]

	switch cmd {
	case "init":
		if err := config.InitConfig(); err != nil {
			fmt.Fprintf(os.Stderr, "Error: %v\n", err)
			os.Exit(1)
		}
	case "start":
		cfg, err := config.LoadConfig()
		if err != nil {
			fmt.Fprintf(os.Stderr, "Error loading config: %v\n", err)
			os.Exit(1)
		}
		if err := tui.Run(cfg); err != nil {
			fmt.Fprintf(os.Stderr, "Error: %v\n", err)
			os.Exit(1)
		}
	default:
		showHelp()
		os.Exit(1)
	}
}

func showHelp() {
	fmt.Println("amux - Agent Multiplexer")
	fmt.Println()
	fmt.Println("Usage: amux <command>")
	fmt.Println()
	fmt.Println("Commands:")
	fmt.Println("  init    Create sample configuration file")
	fmt.Println("  start   Start the TUI and attach")
	fmt.Println()
	fmt.Println("Key bindings (in TUI):")
	fmt.Println("  Ctrl+A  Toggle between sidebar and terminal mode")
	fmt.Println("  1-9     Switch to project N")
	fmt.Println("  ↑/↓     Navigate projects (sidebar mode)")
	fmt.Println("  Enter   Activate selected project (sidebar mode)")
	fmt.Println("  q       Quit (sidebar mode)")
	fmt.Println("  Ctrl+C  Force quit")
}
