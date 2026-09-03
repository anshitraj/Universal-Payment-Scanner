package main

import (
	"log/slog"
	"net/http"
	"os"
	"time"

	"github.com/example/universal-payment-qr/services/api-go/internal/core"
	"github.com/example/universal-payment-qr/services/api-go/internal/httpapi"
)

func main() {
	logger := slog.New(slog.NewJSONHandler(os.Stdout, nil))
	coreBinary := envOr("UPQR_CORE_BIN", "upqr-core")
	address := envOr("UPQR_LISTEN_ADDR", ":8080")
	server := &http.Server{
		Addr: address, Handler: httpapi.New(core.Client{Binary: coreBinary}, logger),
		ReadHeaderTimeout: 5 * time.Second, ReadTimeout: 5 * time.Second, WriteTimeout: 10 * time.Second, IdleTimeout: 60 * time.Second,
		MaxHeaderBytes: 16 * 1024,
	}
	logger.Info("starting API", "address", address)
	if err := server.ListenAndServe(); err != nil && err != http.ErrServerClosed {
		logger.Error("server stopped", "error", err)
		os.Exit(1)
	}
}

func envOr(key, fallback string) string {
	if value := os.Getenv(key); value != "" {
		return value
	}
	return fallback
}
