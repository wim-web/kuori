package main

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"strings"
)

type Task struct {
	Name         string            `json:"name"`
	Host         string            `json:"host"`
	ScriptPath   string            `json:"script_path"`
	WorkingDir   string            `json:"working_dir"`
	Sudo         bool              `json:"sudo"`
	Environments map[string]string `json:"environments"`
	TimeoutSec   *uint64           `json:"timeout_sec"`
	Retry        *uint32           `json:"retry"`
}

type Config struct {
	Tasks []Task `json:"tasks"`
}

func (c *Config) Validate() error {
	if len(c.Tasks) == 0 {
		return fmt.Errorf("`tasks` must contain at least one task")
	}

	seenNames := make(map[string]bool)

	for idx, task := range c.Tasks {
		if err := validateNonEmpty(task.Name, "name", idx); err != nil {
			return err
		}
		if err := validateNonEmpty(task.Host, "host", idx); err != nil {
			return err
		}
		if err := validateNonEmpty(task.ScriptPath, "script_path", idx); err != nil {
			return err
		}
		if err := validateNonEmpty(task.WorkingDir, "working_dir", idx); err != nil {
			return err
		}
		if err := validateTimeoutSec(task.TimeoutSec, idx); err != nil {
			return err
		}

		if seenNames[task.Name] {
			return fmt.Errorf("tasks[%d].name is duplicated: %s", idx, task.Name)
		}
		seenNames[task.Name] = true
	}

	return nil
}

func validateNonEmpty(value, fieldName string, taskIndex int) error {
	if strings.TrimSpace(value) == "" {
		return fmt.Errorf("tasks[%d].%s must not be empty", taskIndex, fieldName)
	}
	return nil
}

func validateTimeoutSec(timeoutSec *uint64, taskIndex int) error {
	if timeoutSec != nil && *timeoutSec == 0 {
		return fmt.Errorf("tasks[%d].timeout_sec must be greater than 0", taskIndex)
	}
	return nil
}

func loadConfig(configPath string) (*Config, error) {
	data, err := os.ReadFile(configPath)
	if err != nil {
		return nil, fmt.Errorf("failed to read config file: %s: %w", configPath, err)
	}

	var config Config
	if err := json.Unmarshal(data, &config); err != nil {
		return nil, fmt.Errorf("failed to parse config file: %s: %w", configPath, err)
	}

	if err := config.Validate(); err != nil {
		return nil, fmt.Errorf("invalid config file: %s: %w", configPath, err)
	}

	configDir := filepath.Dir(configPath)
	for i := range config.Tasks {
		if !filepath.IsAbs(config.Tasks[i].ScriptPath) {
			config.Tasks[i].ScriptPath = filepath.Join(configDir, config.Tasks[i].ScriptPath)
		}
	}

	return &config, nil
}
