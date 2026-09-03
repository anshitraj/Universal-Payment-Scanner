package httpapi

import (
	"context"
	"io"
	"log/slog"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
)

type fakeCore struct{ command, payload, policy string }

func (f *fakeCore) Run(_ context.Context, command string, payload []byte, policy string) ([]byte, error) {
	f.command, f.payload, f.policy = command, string(payload), policy
	return []byte(`{"recognized":true,"scheme":"upi"}`), nil
}

func TestParseDelegatesToRustCoreWithoutLoggingPayload(t *testing.T) {
	fake := &fakeCore{}
	server := New(fake, slog.New(slog.NewTextHandler(io.Discard, nil)))
	req := httptest.NewRequest(http.MethodPost, "/v1/parse", strings.NewReader(`{"payload":"upi://pay?pa=test@bank","policy":{"schemes":{"upi":false}}}`))
	res := httptest.NewRecorder()
	server.ServeHTTP(res, req)
	if res.Code != http.StatusOK {
		t.Fatalf("got status %d", res.Code)
	}
	if fake.command != "parse" || fake.payload != "upi://pay?pa=test@bank" {
		t.Fatalf("unexpected delegation: %#v", fake)
	}
	if fake.policy == "" {
		t.Fatal("policy was not forwarded")
	}
}

func TestRequestSizeIsBounded(t *testing.T) {
	server := New(&fakeCore{}, slog.New(slog.NewTextHandler(io.Discard, nil)))
	body := `{"payload":"` + strings.Repeat("x", maxRequestBytes) + `"}`
	res := httptest.NewRecorder()
	server.ServeHTTP(res, httptest.NewRequest(http.MethodPost, "/v1/parse", strings.NewReader(body)))
	if res.Code != http.StatusRequestEntityTooLarge {
		t.Fatalf("got status %d", res.Code)
	}
}
