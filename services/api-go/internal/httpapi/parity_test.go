package httpapi

import (
	"encoding/json"
	"io"
	"log/slog"
	"net/http"
	"net/http/httptest"
	"os"
	"strings"
	"testing"

	"github.com/example/universal-payment-qr/services/api-go/internal/core"
)

func TestRustCoreHTTPParity(t *testing.T) {
	binary := os.Getenv("UPQR_CORE_BIN")
	if binary == "" {
		t.Skip("set UPQR_CORE_BIN to run the Rust/Go parity test")
	}
	server := New(core.Client{Binary: binary}, slog.New(slog.NewTextHandler(io.Discard, nil)))
	request := httptest.NewRequest(http.MethodPost, "/v1/parse", strings.NewReader(`{"payload":"upi://pay?pa=merchant%40bank&am=12.34&cu=INR"}`))
	response := httptest.NewRecorder()
	server.ServeHTTP(response, request)
	if response.Code != http.StatusOK {
		t.Fatalf("status %d: %s", response.Code, response.Body.String())
	}
	var intent struct {
		Scheme string `json:"scheme"`
		Amount string `json:"amount"`
	}
	if err := json.Unmarshal(response.Body.Bytes(), &intent); err != nil {
		t.Fatal(err)
	}
	if intent.Scheme != "upi" || intent.Amount != "12.34" {
		t.Fatalf("unexpected core output: %#v", intent)
	}
}
