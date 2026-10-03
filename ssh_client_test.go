package main

import (
	"crypto/ed25519"
	"crypto/rand"
	"crypto/x509"
	"encoding/pem"
	"fmt"
	"net"
	"os"
	"path/filepath"
	"strings"
	"testing"

	"golang.org/x/crypto/ssh"
	"golang.org/x/crypto/ssh/knownhosts"
)

func TestConnectVerifiesHostKey(t *testing.T) {
	for _, mode := range []string{"trusted", "hashed", "unknown", "changed", "missing"} {
		t.Run(mode, func(t *testing.T) {
			home := t.TempDir()
			t.Setenv("HOME", home)
			dir := filepath.Join(home, ".ssh")
			if err := os.Mkdir(dir, 0700); err != nil {
				t.Fatal(err)
			}
			write := func(name, text string) {
				t.Helper()
				if err := os.WriteFile(filepath.Join(dir, name), []byte(text), 0600); err != nil {
					t.Fatal(err)
				}
			}
			_, clientKey, err := ed25519.GenerateKey(rand.Reader)
			if err != nil {
				t.Fatal(err)
			}
			der, err := x509.MarshalPKCS8PrivateKey(clientKey)
			if err != nil {
				t.Fatal(err)
			}
			write("id_ed25519", string(pem.EncodeToMemory(&pem.Block{Type: "PRIVATE KEY", Bytes: der})))
			clientSigner, err := ssh.NewSignerFromKey(clientKey)
			if err != nil {
				t.Fatal(err)
			}
			newHostKey := func() ssh.Signer {
				t.Helper()
				_, key, err := ed25519.GenerateKey(rand.Reader)
				if err != nil {
					t.Fatal(err)
				}
				signer, err := ssh.NewSignerFromKey(key)
				if err != nil {
					t.Fatal(err)
				}
				return signer
			}
			hostKey := newHostKey()
			listener, err := net.Listen("tcp", "127.0.0.1:0")
			if err != nil {
				t.Fatal(err)
			}
			defer listener.Close()
			_, port, _ := net.SplitHostPort(listener.Addr().String())
			write("config", fmt.Sprintf("Host test-host\n HostName 127.0.0.1\n Port %s\n User deploy\n IdentityFile ~/.ssh/id_ed25519\n", port))
			trustedKey := hostKey.PublicKey()
			if mode == "changed" {
				trustedKey = newHostKey().PublicKey()
			}
			host := knownhosts.Normalize(listener.Addr().String())
			if mode == "unknown" {
				host = "other.example.test"
			}
			if mode == "hashed" {
				host = knownhosts.HashHostname(host)
			}
			if mode != "missing" {
				write("known_hosts", host+" "+strings.TrimSpace(string(ssh.MarshalAuthorizedKey(trustedKey)))+"\n")
			}
			if mode != "missing" {
				done := make(chan struct{})
				defer func() { listener.Close(); <-done }()
				go func() {
					defer close(done)
					conn, err := listener.Accept()
					if err != nil {
						return
					}
					defer conn.Close()
					config := &ssh.ServerConfig{PublicKeyCallback: func(_ ssh.ConnMetadata, key ssh.PublicKey) (*ssh.Permissions, error) {
						if string(key.Marshal()) != string(clientSigner.PublicKey().Marshal()) {
							return nil, fmt.Errorf("untrusted client")
						}
						return nil, nil
					}}
					config.AddHostKey(hostKey)
					server, _, _, err := ssh.NewServerConn(conn, config)
					if err == nil {
						server.Close()
					}
				}()
			}
			client, err := connect("test-host")
			if client != nil {
				client.Close()
			}
			if mode == "trusted" || mode == "hashed" {
				if err != nil {
					t.Fatalf("trusted host rejected: %v", err)
				}
			} else if err == nil {
				t.Fatalf("%s host key accepted", mode)
			} else if !strings.Contains(err.Error(), "known_hosts") && !strings.Contains(err.Error(), "knownhosts") {
				t.Fatalf("failed for a reason other than host verification: %v", err)
			}
		})
	}
}
