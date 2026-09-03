package core

import (
	"bytes"
	"context"
	"fmt"
	"os/exec"
)

// Runner abstracts the Rust core boundary for handlers and tests.
type Runner interface {
	Run(ctx context.Context, command string, payload []byte, policyJSON string) ([]byte, error)
}

// Client invokes the deterministic Rust CLI. Deployments can replace this boundary with a
// native library build later without duplicating payment parsing logic in Go.
type Client struct {
	Binary string
}

func (c Client) Run(ctx context.Context, command string, payload []byte, policyJSON string) ([]byte, error) {
	args := []string{command}
	if policyJSON != "" {
		args = append(args, policyJSON)
	}
	cmd := exec.CommandContext(ctx, c.Binary, args...)
	cmd.Stdin = bytes.NewReader(payload)
	var stderr bytes.Buffer
	cmd.Stderr = &stderr
	out, err := cmd.Output()
	if err != nil {
		return nil, fmt.Errorf("rust core failed: %w: %s", err, stderr.String())
	}
	return bytes.TrimSpace(out), nil
}
