// Package config handles configuration loading and validation
package config

import (
	"fmt"
	"os"
	"path/filepath"

	"gopkg.in/yaml.v3"
)

// Project represents a single project configuration
type Project struct {
	Name  string `yaml:"name"`
	Path  string `yaml:"path"`
	Agent string `yaml:"agent"`
}

// Config represents the main configuration
type Config struct {
	SidebarWidth     int       `yaml:"sidebar_width"`
	SidebarToggleKey string    `yaml:"sidebar_toggle_key"`
	Projects         []Project `yaml:"projects"`
}

const (
	configDir  = ".config/amux"
	configFile = "config.yaml"
)

func getHomeDir() string {
	home, err := os.UserHomeDir()
	if err != nil {
		panic(err)
	}
	return home
}

// GetConfigPath returns the full path to the config file
func GetConfigPath() string {
	return filepath.Join(getHomeDir(), configDir, configFile)
}

// GetStatusDir returns the full path to the status directory
func GetStatusDir() string {
	return filepath.Join(getHomeDir(), ".local/share/amux/status")
}

// InitConfig creates a sample configuration file
func InitConfig() error {
	configPath := GetConfigPath()

	// Create config directory
	cfgDir := filepath.Dir(configPath)
	if err := os.MkdirAll(cfgDir, 0755); err != nil {
		return fmt.Errorf("creating config directory: %w", err)
	}

	// Check if config already exists
	if _, err := os.Stat(configPath); err == nil {
		return fmt.Errorf("config already exists at %s", configPath)
	}

	sampleConfig := `# amux configuration
# Place this file at ~/.config/amux/config.yaml

sidebar_width: 25
sidebar_toggle_key: A  # Key to toggle sidebar visibility (Prefix + A)

projects:
  - name: project1
    path: ~/projects/project1
    agent: opencode
  - name: project2
    path: ~/projects/project2
    agent: claude
`

	if err := os.WriteFile(configPath, []byte(sampleConfig), 0644); err != nil {
		return fmt.Errorf("writing config file: %w", err)
	}

	fmt.Printf("Created sample config at %s\n", configPath)
	return nil
}

// LoadConfig reads and validates the configuration file
func LoadConfig() (*Config, error) {
	configPath := GetConfigPath()

	data, err := os.ReadFile(configPath)
	if err != nil {
		return nil, fmt.Errorf("reading config file: %w", err)
	}

	var cfg Config
	if err := yaml.Unmarshal(data, &cfg); err != nil {
		return nil, fmt.Errorf("parsing config file: %w", err)
	}

	// Set defaults
	if cfg.SidebarWidth == 0 {
		cfg.SidebarWidth = 25
	}
	if cfg.SidebarToggleKey == "" {
		cfg.SidebarToggleKey = "A"
	}

	// Validate toggle key is a single character
	if len(cfg.SidebarToggleKey) != 1 {
		return nil, fmt.Errorf("sidebar_toggle_key must be a single character, got: %s", cfg.SidebarToggleKey)
	}

	// Validate
	for i, proj := range cfg.Projects {
		if proj.Name == "" {
			return nil, fmt.Errorf("project %d: missing name", i)
		}
		if proj.Path == "" {
			return nil, fmt.Errorf("project %s: missing path", proj.Name)
		}
		if proj.Agent == "" {
			return nil, fmt.Errorf("project %s: missing agent", proj.Name)
		}
	}

	return &cfg, nil
}
