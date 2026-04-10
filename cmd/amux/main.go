package main

import (
	"fmt"
	"os"

	"github.com/user/amux/internal/config"
	"github.com/user/amux/internal/session"
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
		if err := session.Start(); err != nil {
			fmt.Fprintf(os.Stderr, "Error: %v\n", err)
			os.Exit(1)
		}
	case "stop":
		if err := session.Stop(); err != nil {
			fmt.Fprintf(os.Stderr, "Error: %v\n", err)
			os.Exit(1)
		}
	case "switch":
		if len(os.Args) < 3 {
			fmt.Fprintf(os.Stderr, "Error: project name required\n")
			os.Exit(1)
		}
		cfg, err := config.LoadConfig()
		if err != nil {
			fmt.Fprintf(os.Stderr, "Error: %v\n", err)
			os.Exit(1)
		}
		var targetProject *config.Project
		for _, proj := range cfg.Projects {
			if proj.Name == os.Args[2] {
				targetProject = &proj
				break
			}
		}
		if targetProject == nil {
			fmt.Fprintf(os.Stderr, "Error: project '%s' not found\n", os.Args[2])
			os.Exit(1)
		}
		if err := session.SwitchTo(*targetProject); err != nil {
			fmt.Fprintf(os.Stderr, "Error: %v\n", err)
			os.Exit(1)
		}
	default:
		showHelp()
		os.Exit(1)
	}
}

func showHelp() {
	fmt.Println("amux - Agent Multiplexer for tmux")
	fmt.Println()
	fmt.Println("Usage: amux <command>")
	fmt.Println()
	fmt.Println("Commands:")
	fmt.Println("  init    Create sample configuration file")
	fmt.Println("  start   Start the orchestrator and attach")
	fmt.Println("  stop    Detach from orchestrator")
	fmt.Println("  switch  Switch to a project")
	fmt.Println()
	fmt.Println("Key bindings (in amux session):")
	fmt.Println("  1-9     Switch to project N")
	fmt.Println("  r       Refresh sidebar")
	fmt.Println("  q/Esc   Hide sidebar")
	fmt.Println("  Prefix+S  Toggle sidebar visibility")
}
