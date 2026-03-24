package main

import (
	"fmt"
	"os"
	"path/filepath"
	"strings"

	sshconfig "github.com/kevinburke/ssh_config"
)

type SSHHostParams struct {
	HostName     string
	Port         string
	User         string
	IdentityFile string
}

func resolveSSHHostParams(host string) (*SSHHostParams, error) {
	home, err := os.UserHomeDir()
	if err != nil {
		return nil, fmt.Errorf("failed to get home directory: %w", err)
	}

	configPath := filepath.Join(home, ".ssh", "config")
	f, err := os.Open(configPath)
	if err != nil {
		return nil, fmt.Errorf("could not open SSH config: %w", err)
	}
	defer f.Close()

	cfg, err := sshconfig.Decode(f)
	if err != nil {
		return nil, fmt.Errorf("failed to parse SSH config: %w", err)
	}

	hostName, _ := cfg.Get(host, "HostName")
	port, _ := cfg.Get(host, "Port")
	user, _ := cfg.Get(host, "User")
	identityFile, _ := cfg.Get(host, "IdentityFile")

	if hostName == "" {
		return nil, fmt.Errorf("not found hostname for host: %s", host)
	}
	if user == "" {
		return nil, fmt.Errorf("not found user for host: %s", host)
	}
	if identityFile == "" {
		return nil, fmt.Errorf("not found identity file for host: %s", host)
	}

	if strings.HasPrefix(identityFile, "~/") {
		identityFile = filepath.Join(home, identityFile[2:])
	}

	if port == "" {
		port = "22"
	}

	return &SSHHostParams{
		HostName:     hostName,
		Port:         port,
		User:         user,
		IdentityFile: identityFile,
	}, nil
}
