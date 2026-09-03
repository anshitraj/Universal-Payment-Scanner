package httpapi

import (
	"context"
	"encoding/json"
	"errors"
	"io"
	"log/slog"
	"net/http"
	"time"

	"github.com/example/universal-payment-qr/services/api-go/internal/core"
)

const maxRequestBytes = 16 * 1024

type Server struct {
	core core.Runner
	log  *slog.Logger
}

type parseRequest struct {
	Payload string          `json:"payload"`
	Policy  json.RawMessage `json:"policy,omitempty"`
}

func New(runner core.Runner, logger *slog.Logger) http.Handler {
	s := &Server{core: runner, log: logger}
	mux := http.NewServeMux()
	mux.HandleFunc("GET /health", s.health)
	mux.HandleFunc("GET /v1/schemes", s.metadata("schemes"))
	mux.HandleFunc("GET /v1/capabilities", s.metadata("capabilities"))
	mux.HandleFunc("POST /v1/parse", s.parse("parse"))
	mux.HandleFunc("POST /v1/detect", s.parse("detect"))
	mux.HandleFunc("POST /v1/validate", s.parse("validate"))
	return securityHeaders(requestLog(logger, mux))
}

func (s *Server) health(w http.ResponseWriter, _ *http.Request) {
	writeJSON(w, http.StatusOK, []byte(`{"status":"ok"}`))
}

func (s *Server) metadata(command string) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		ctx, cancel := context.WithTimeout(r.Context(), 3*time.Second)
		defer cancel()
		result, err := s.core.Run(ctx, command, nil, "")
		if err != nil {
			s.log.Error("core metadata request failed", "command", command, "error", err)
			writeError(w, http.StatusServiceUnavailable, "CORE_UNAVAILABLE", "Payment parser is unavailable.")
			return
		}
		writeJSON(w, http.StatusOK, result)
	}
}

func (s *Server) parse(command string) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		r.Body = http.MaxBytesReader(w, r.Body, maxRequestBytes)
		decoder := json.NewDecoder(r.Body)
		decoder.DisallowUnknownFields()
		var request parseRequest
		if err := decoder.Decode(&request); err != nil {
			status := http.StatusBadRequest
			var tooLarge *http.MaxBytesError
			if errors.As(err, &tooLarge) {
				status = http.StatusRequestEntityTooLarge
			}
			writeError(w, status, "INVALID_REQUEST", "Request must contain a bounded JSON payload string.")
			return
		}
		if request.Payload == "" {
			writeError(w, http.StatusBadRequest, "INVALID_REQUEST", "payload is required.")
			return
		}
		if err := ensureEOF(decoder); err != nil {
			writeError(w, http.StatusBadRequest, "INVALID_REQUEST", "Only one JSON value is accepted.")
			return
		}
		ctx, cancel := context.WithTimeout(r.Context(), 3*time.Second)
		defer cancel()
		result, err := s.core.Run(ctx, command, []byte(request.Payload), string(request.Policy))
		if err != nil {
			// Never log payloads or policies: they can contain payment identifiers.
			s.log.Error("core parse request failed", "command", command, "error", err)
			writeError(w, http.StatusServiceUnavailable, "CORE_UNAVAILABLE", "Payment parser is unavailable.")
			return
		}
		writeJSON(w, http.StatusOK, result)
	}
}

func ensureEOF(decoder *json.Decoder) error {
	var extra any
	if err := decoder.Decode(&extra); !errors.Is(err, io.EOF) {
		return errors.New("trailing JSON")
	}
	return nil
}

func writeJSON(w http.ResponseWriter, status int, body []byte) {
	w.Header().Set("Content-Type", "application/json; charset=utf-8")
	w.WriteHeader(status)
	_, _ = w.Write(body)
}

func writeError(w http.ResponseWriter, status int, code, message string) {
	body, _ := json.Marshal(map[string]string{"code": code, "message": message})
	writeJSON(w, status, body)
}

func securityHeaders(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Cache-Control", "no-store")
		w.Header().Set("Content-Security-Policy", "default-src 'none'; frame-ancestors 'none'")
		w.Header().Set("X-Content-Type-Options", "nosniff")
		next.ServeHTTP(w, r)
	})
}

func requestLog(logger *slog.Logger, next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		start := time.Now()
		next.ServeHTTP(w, r)
		logger.Info("request", "method", r.Method, "path", r.URL.Path, "duration_ms", time.Since(start).Milliseconds())
	})
}
