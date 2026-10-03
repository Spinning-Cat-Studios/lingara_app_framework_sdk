// Package lingaraapps is the Go kit for Lingara apps: servers that answer
// Lingara's two signed requests, app.render and app.action, with a card.
//
// NewHandler verifies each request with the lingara library's webhook
// verifier, decodes it, calls your function, checks the card it returns
// against Lingara's card rules and sends it. The builders (NewCard, Reply,
// NewManifest) refuse at build time what Lingara would otherwise clamp.
package lingaraapps

import (
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"log/slog"
	"net/http"

	lingara "github.com/Spinning-Cat-Studios/lingara_api_clients/go"
)

// RenderFunc answers app.render: Lingara wants a card for req.Slot.
type RenderFunc func(ctx context.Context, req AppRenderRequest) (Replier, error)

// ActionFunc answers app.action: the learner pressed the button whose
// action is req.ActionID. A press may arrive twice; make the function safe
// to run twice.
type ActionFunc func(ctx context.Context, req AppActionRequest) (Replier, error)

// App is your app: one render function, and an action function per button
// action.
type App struct {
	Render  RenderFunc
	Actions map[string]ActionFunc
	// Logger receives one line per refused or failed reply and per skipped
	// context slice. Nil is slog.Default(). No line ever holds a body, a
	// secret or a signature.
	Logger *slog.Logger
}

// NewHandler returns app's http.Handler. secrets are the app's signing
// secrets, lgr_whsec_…, two during a rotation; a malformed one is an error
// here, never on a request.
func NewHandler(app App, secrets ...string) (http.Handler, error) {
	if app.Render == nil {
		return nil, errors.New("lingaraapps: App.Render is nil")
	}
	webhook, err := lingara.NewWebhook(secrets...)
	if err != nil {
		return nil, err
	}
	logger := app.Logger
	if logger == nil {
		logger = slog.Default()
	}
	return &handler{app: app, webhook: webhook, log: logger}, nil
}

type handler struct {
	app     App
	webhook *lingara.Webhook
	log     *slog.Logger
}

// errBadRequest is a request that verified but does not decode, or names
// an operation or action the app does not have: a 400.
var errBadRequest = errors.New("bad request")

// dispatch is the core's operation table. Its keys must be exactly
// Operations(), which handler_test.go checks: a new operation in the view
// fails the build until it is handled here.
var dispatch = map[Operation]func(*handler, context.Context, []byte) (Replier, error){
	OperationAppRender: (*handler).render,
	OperationAppAction: (*handler).action,
}

var (
	badRequestBody    = []byte(`{"error":"bad_request"}`)
	handlerFailedBody = []byte(`{"error":"handler_failed"}`)
)

func (h *handler) ServeHTTP(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		w.Header().Set("Allow", http.MethodPost)
		w.WriteHeader(http.StatusMethodNotAllowed)
		return
	}
	if r.ContentLength > MaxRequestBytes {
		w.WriteHeader(http.StatusRequestEntityTooLarge)
		return
	}
	body, err := io.ReadAll(io.LimitReader(r.Body, MaxRequestBytes+1))
	if err != nil {
		writeJSON(w, http.StatusBadRequest, badRequestBody)
		return
	}
	if len(body) > MaxRequestBytes {
		w.WriteHeader(http.StatusRequestEntityTooLarge)
		return
	}
	if err := h.webhook.VerifySignature(body, r.Header); err != nil {
		w.WriteHeader(http.StatusUnauthorized)
		return
	}
	status, reply := h.handle(r.Context(), body)
	writeJSON(w, status, reply)
}

func writeJSON(w http.ResponseWriter, status int, body []byte) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	_, _ = w.Write(body)
}

// handle runs a verified body: dispatch on its type, call the app, and
// validate and encode what it returns.
func (h *handler) handle(ctx context.Context, body []byte) (int, []byte) {
	var head struct {
		Type      Operation `json:"type"`
		InstallID any       `json:"install_id"`
	}
	if err := json.Unmarshal(body, &head); err != nil {
		return http.StatusBadRequest, badRequestBody
	}
	op, ok := dispatch[head.Type]
	if !ok {
		return http.StatusBadRequest, badRequestBody
	}
	installID := fmt.Sprint(head.InstallID)
	replier, err := h.call(ctx, op, body)
	if errors.Is(err, errBadRequest) {
		return http.StatusBadRequest, badRequestBody
	}
	if err == nil {
		var encoded []byte
		if encoded, err = ValidateReply(replier); err == nil {
			return http.StatusOK, encoded
		}
	}
	h.log.ErrorContext(ctx, "lingaraapps: reply refused",
		slog.String("operation", string(head.Type)), slog.String("install_id", installID), slog.String("reason", reason(err)))
	return http.StatusInternalServerError, handlerFailedBody
}

// call runs op, turning a panic in the app's function into an error.
func (h *handler) call(ctx context.Context, op func(*handler, context.Context, []byte) (Replier, error), body []byte) (replier Replier, err error) {
	defer func() {
		if p := recover(); p != nil {
			replier, err = nil, fmt.Errorf("the function panicked: %v", p)
		}
	}()
	replier, err = op(h, ctx, body)
	if err == nil && replier == nil {
		err = errors.New("the function returned no reply")
	}
	return replier, err
}

// reason is what the log line names: a card rule, or that the function
// failed. The error's own text is the app's and may hold anything.
func reason(err error) string {
	var limit *CardLimitError
	if errors.As(err, &limit) {
		return string(limit.Reason)
	}
	return "function_failed"
}

func (h *handler) render(ctx context.Context, body []byte) (Replier, error) {
	var wire struct {
		AppRenderRequest
		Context []json.RawMessage `json:"context"`
	}
	if err := json.Unmarshal(body, &wire); err != nil {
		return nil, errBadRequest
	}
	req := wire.AppRenderRequest
	var err error
	if req.Context, err = h.slices(ctx, wire.Context); err != nil {
		return nil, err
	}
	return h.app.Render(ctx, req)
}

func (h *handler) action(ctx context.Context, body []byte) (Replier, error) {
	var wire struct {
		AppActionRequest
		Context []json.RawMessage `json:"context"`
	}
	if err := json.Unmarshal(body, &wire); err != nil {
		return nil, errBadRequest
	}
	fn, ok := h.app.Actions[wire.ActionID]
	if !ok || fn == nil {
		return nil, errBadRequest
	}
	req := wire.AppActionRequest
	var err error
	if req.Context, err = h.slices(ctx, wire.Context); err != nil {
		return nil, err
	}
	return fn(ctx, req)
}

// slices decodes the context one slice at a time: a slice of a kind this
// kit does not know is skipped with a log line, so a newer Lingara is never
// a 400.
func (h *handler) slices(ctx context.Context, raw []json.RawMessage) ([]ContextSlice, error) {
	out := make([]ContextSlice, 0, len(raw))
	for _, r := range raw {
		var tag struct {
			Kind string `json:"kind"`
		}
		if err := json.Unmarshal(r, &tag); err != nil {
			return nil, errBadRequest
		}
		slice, known, err := decodeContextSlice(tag.Kind, r)
		switch {
		case err != nil:
			return nil, errBadRequest
		case !known:
			h.log.InfoContext(ctx, "lingaraapps: skipped a context slice of an unknown kind", slog.String("kind", tag.Kind))
			continue
		}
		out = append(out, slice)
	}
	return out, nil
}
