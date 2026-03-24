package main

import (
	"bytes"
	"fmt"
	"io"
	"math/rand"
	"net"
	"os"
	"path"
	"sort"
	"strings"
	"sync"
	"time"

	"golang.org/x/crypto/ssh"
)

type SessionManager struct {
	clients map[string]*ssh.Client
}

func NewSessionManager() *SessionManager {
	return &SessionManager{
		clients: make(map[string]*ssh.Client),
	}
}

func (sm *SessionManager) GetOrConnect(host string) (*ssh.Client, error) {
	if client, ok := sm.clients[host]; ok {
		return client, nil
	}

	client, err := connect(host)
	if err != nil {
		return nil, err
	}
	sm.clients[host] = client
	return client, nil
}

func (sm *SessionManager) Close() {
	for _, client := range sm.clients {
		client.Close()
	}
}

func connect(host string) (*ssh.Client, error) {
	params, err := resolveSSHHostParams(host)
	if err != nil {
		return nil, err
	}

	keyData, err := os.ReadFile(params.IdentityFile)
	if err != nil {
		return nil, fmt.Errorf("failed to read identity file: %w", err)
	}

	signer, err := ssh.ParsePrivateKey(keyData)
	if err != nil {
		return nil, fmt.Errorf("failed to parse private key: %w", err)
	}

	config := &ssh.ClientConfig{
		User: params.User,
		Auth: []ssh.AuthMethod{
			ssh.PublicKeys(signer),
		},
		HostKeyCallback: ssh.InsecureIgnoreHostKey(),
		Timeout:         30 * time.Second,
	}

	addr := net.JoinHostPort(params.HostName, params.Port)
	client, err := ssh.Dial("tcp", addr, config)
	if err != nil {
		return nil, fmt.Errorf("SSH connection failed: %w", err)
	}

	return client, nil
}

func execScript(
	sm *SessionManager,
	host string,
	localScriptPath string,
	workingDir string,
	environments map[string]string,
	useSudo bool,
	timeoutSec *uint64,
) error {
	client, err := sm.GetOrConnect(host)
	if err != nil {
		return err
	}

	remotePath := path.Join(workingDir, generateRandomString(10))

	if err := sendScript(client, localScriptPath, remotePath); err != nil {
		return err
	}

	envCommand := buildEnvCommand(environments)

	sudoPrefix := ""
	if useSudo {
		sudoPrefix = "sudo "
	}
	command := fmt.Sprintf("cd %s; %s %sbash %s", workingDir, envCommand, sudoPrefix, remotePath)

	result := runRemoteCommand(client, command, timeoutSec)

	_ = runRemoteCommand(client, fmt.Sprintf("rm %s", remotePath), nil)

	return result
}

func buildEnvCommand(environments map[string]string) string {
	if len(environments) == 0 {
		return ""
	}
	parts := make([]string, 0, len(environments))
	for key, value := range environments {
		parts = append(parts, fmt.Sprintf("export %s=%s;", key, value))
	}
	return strings.Join(parts, " ")
}

func sendScript(client *ssh.Client, localPath, remotePath string) error {
	data, err := os.ReadFile(localPath)
	if err != nil {
		return fmt.Errorf("failed to read local script: %w", err)
	}

	session, err := client.NewSession()
	if err != nil {
		return err
	}
	defer session.Close()

	session.Stdin = bytes.NewReader(data)
	cmd := fmt.Sprintf("cat > '%s' && chmod 755 '%s'", remotePath, remotePath)
	return session.Run(cmd)
}

func runRemoteCommand(client *ssh.Client, command string, timeoutSec *uint64) error {
	session, err := client.NewSession()
	if err != nil {
		return err
	}
	defer session.Close()

	stdout, err := session.StdoutPipe()
	if err != nil {
		return err
	}
	stderr, err := session.StderrPipe()
	if err != nil {
		return err
	}

	if err := session.Start(command); err != nil {
		return err
	}

	var wg sync.WaitGroup
	wg.Add(2)
	go func() {
		defer wg.Done()
		io.Copy(os.Stdout, stdout)
	}()
	go func() {
		defer wg.Done()
		io.Copy(os.Stderr, stderr)
	}()

	done := make(chan error, 1)
	go func() {
		wg.Wait()
		done <- session.Wait()
	}()

	if timeoutSec != nil {
		select {
		case err := <-done:
			if err != nil {
				return fmt.Errorf("コマンド実行中にエラーが発生しました: %w", err)
			}
			return nil
		case <-time.After(time.Duration(*timeoutSec) * time.Second):
			_ = session.Signal(ssh.SIGKILL)
			return fmt.Errorf("コマンド実行がタイムアウトしました (%d秒): %s", *timeoutSec, command)
		}
	}

	if err := <-done; err != nil {
		return fmt.Errorf("コマンド実行中にエラーが発生しました: %w", err)
	}
	return nil
}

func formatEnvKeys(task *Task) string {
	if len(task.Environments) == 0 {
		return "(none)"
	}

	keys := make([]string, 0, len(task.Environments))
	for k := range task.Environments {
		keys = append(keys, k)
	}
	sort.Strings(keys)
	return strings.Join(keys, ",")
}

func generateRandomString(length int) string {
	const charset = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"
	b := make([]byte, length)
	for i := range b {
		b[i] = charset[rand.Intn(len(charset))]
	}
	return string(b)
}
