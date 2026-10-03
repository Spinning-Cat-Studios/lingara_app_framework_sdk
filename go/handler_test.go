package lingaraapps

import (
	"context"
	"crypto/hmac"
	"crypto/sha256"
	"encoding/base64"
	"errors"
	"io"
	"log/slog"
	"net/http"
	"net/http/httptest"
	"slices"
	"strconv"
	"strings"
	"testing"
	"time"
)

// testKey is a visibly fake 32-byte signing key.
var testKey = []byte("handler-test-secret-0001!!!!!!!!")

func testSecret() string { return "lgr_whsec_" + base64.StdEncoding.EncodeToString(testKey) }

// signed builds a POST signed as Lingara signs it: Standard Webhooks,
// HMAC-SHA256 over id.timestamp.body.
func signed(body string) *http.Request {
	id := "lgr_msg_0123456789abcdef0123456789abcdef"
	stamp := strconv.FormatInt(time.Now().Unix(), 10)
	mac := hmac.New(sha256.New, testKey)
	mac.Write([]byte(id + "." + stamp + "." + body))
	r := httptest.NewRequest(http.MethodPost, "/lingara", strings.NewReader(body))
	r.Header.Set("webhook-id", id)
	r.Header.Set("webhook-timestamp", stamp)
	r.Header.Set("webhook-signature", "v1,"+base64.StdEncoding.EncodeToString(mac.Sum(nil)))
	r.Header.Set("content-type", "application/json")
	return r
}

const renderBody = `{"type":"app.render","id":"lgr_msg_0123456789abcdef0123456789abcdef",` +
	`"install_id":"7c1d2e3f-4a5b-4c6d-8e7f-9a0b1c2d3e4f","subject":"lgr_sub_learner","slot":"home.side",` +
	`"locale":"ja","context":[{"kind":"languages","source_lang":"en","target_lang":"ja","level":3},` +
	`{"kind":"weather","sky":"clear"},{"kind":"review_due","due":12,"learned":340}],"future_field":1}`

const actionBody = `{"type":"app.action","id":"lgr_msg_0123456789abcdef0123456789abcdef",` +
	`"install_id":"7c1d2e3f-4a5b-4c6d-8e7f-9a0b1c2d3e4f","subject":"lgr_sub_learner","slot":"home.side",` +
	`"locale":"ja","context":[],"action_id":"inc","card_etag":"c1_étag/opaque+bytes=="}`

// recorder is an app that records what reached it.
type recorder struct {
	renders []AppRenderRequest
	actions []AppActionRequest
}

func (rec *recorder) app() App {
	card := Card{Elements: []CardElement{CardElementText{Type: CardElementTextTypeText, Text: "ok"}}}
	return App{
		Render: func(_ context.Context, req AppRenderRequest) (Replier, error) {
			rec.renders = append(rec.renders, req)
			return card, nil
		},
		Actions: map[string]ActionFunc{"inc": func(_ context.Context, req AppActionRequest) (Replier, error) {
			rec.actions = append(rec.actions, req)
			return card, nil
		}},
		Logger: slog.New(slog.NewTextHandler(io.Discard, nil)),
	}
}

func serve(t *testing.T, h http.Handler, r *http.Request) *httptest.ResponseRecorder {
	t.Helper()
	w := httptest.NewRecorder()
	h.ServeHTTP(w, r)
	return w
}

// TestHandlerVerifiesTheRawBodyBeforeDispatch: 30.9.26am AC10. Through
// NewHandler, a tampered body is a 401 with an empty body that never reaches
// the render function; a valid render reaches it with the decoded subject,
// slot and slices, an unknown slice kind skipped; and an action's card_etag
// reaches its function byte for byte.
func TestHandlerVerifiesTheRawBodyBeforeDispatch(t *testing.T) {
	rec := &recorder{}
	h, err := NewHandler(rec.app(), testSecret())
	if err != nil {
		t.Fatal(err)
	}
	tampered := signed(renderBody)
	tampered.Body = io.NopCloser(strings.NewReader(strings.Replace(renderBody, "home.side", "home.xide", 1)))
	if w := serve(t, h, tampered); w.Code != http.StatusUnauthorized || w.Body.Len() != 0 || len(rec.renders) != 0 {
		t.Fatalf("tampered: %d %q, %d renders", w.Code, w.Body.String(), len(rec.renders))
	}
	w := serve(t, h, signed(renderBody))
	if w.Code != http.StatusOK || w.Header().Get("Content-Type") != "application/json" || len(rec.renders) != 1 {
		t.Fatalf("render: %d %q", w.Code, w.Body.String())
	}
	checkRenderRequest(t, rec.renders[0])
	checkAction(t, h, rec)
}

// checkAction: an action's card_etag reaches its function byte for byte.
func checkAction(t *testing.T, h http.Handler, rec *recorder) {
	t.Helper()
	if w := serve(t, h, signed(actionBody)); w.Code != http.StatusOK || len(rec.actions) != 1 {
		t.Fatalf("action: %d %q", w.Code, w.Body.String())
	}
	if etag := rec.actions[0].CardEtag; etag != "c1_étag/opaque+bytes==" {
		t.Errorf("card_etag: %q", etag)
	}
}

// checkRenderRequest: the decoded subject and slot, and the two known
// slices in order, the unknown "weather" slice skipped.
func checkRenderRequest(t *testing.T, got AppRenderRequest) {
	t.Helper()
	if got.Subject != "lgr_sub_learner" || got.Slot != AppSlotNameHomeSide || len(got.Context) != 2 {
		t.Fatalf("render request: %+v", got)
	}
	if l, ok := got.Context[0].(ContextSliceLanguages); !ok || l.TargetLang != "ja" || *l.Level != 3 {
		t.Errorf("first slice: %#v", got.Context[0])
	}
	if d, ok := got.Context[1].(ContextSliceReviewDue); !ok || d.Due != 12 {
		t.Errorf("second slice: %#v", got.Context[1])
	}
}

// TestDispatchCoversEveryOperation: 30.9.26am AC18. The core's dispatch
// table keys equal the generated operation constants, so a third operation
// in the view fails here until the core handles it.
func TestDispatchCoversEveryOperation(t *testing.T) {
	keys := make([]string, 0, len(dispatch))
	for op := range dispatch {
		keys = append(keys, string(op))
	}
	generated := make([]string, 0, len(Operations()))
	for _, op := range Operations() {
		generated = append(generated, string(op))
	}
	slices.Sort(keys)
	slices.Sort(generated)
	if !slices.Equal(keys, generated) {
		t.Errorf("dispatch handles %v, the view has %v", keys, generated)
	}
}

// TestTheErrorRepliesAreFixed: AK2 and AK4's statuses and bodies.
func TestTheErrorRepliesAreFixed(t *testing.T) {
	rec := &recorder{}
	app := rec.app()
	app.Actions["boom"] = func(context.Context, AppActionRequest) (Replier, error) { return nil, errors.New("boom") }
	app.Actions["panic"] = func(context.Context, AppActionRequest) (Replier, error) { panic("boom") }
	app.Actions["empty"] = func(context.Context, AppActionRequest) (Replier, error) { return Card{}, nil }
	h, err := NewHandler(app, testSecret())
	if err != nil {
		t.Fatal(err)
	}
	action := func(id string) string { return strings.Replace(actionBody, `"inc"`, strconv.Quote(id), 1) }
	for _, c := range []struct {
		name   string
		req    *http.Request
		status int
		body   string
	}{
		{"get", httptest.NewRequest(http.MethodGet, "/", nil), 405, ""},
		{"oversized", signed(strings.Repeat(" ", MaxRequestBytes+1)), 413, ""},
		{"not json", signed(`{"type":`), 400, `{"error":"bad_request"}`},
		{"unknown type", signed(`{"type":"app.unknown"}`), 400, `{"error":"bad_request"}`},
		{"unregistered", signed(action("nope")), 400, `{"error":"bad_request"}`},
		{"undecodable", signed(strings.Replace(renderBody, `"home.side"`, `5`, 1)), 400, `{"error":"bad_request"}`},
		{"raises", signed(action("boom")), 500, `{"error":"handler_failed"}`},
		{"panics", signed(action("panic")), 500, `{"error":"handler_failed"}`},
		{"refused", signed(action("empty")), 500, `{"error":"handler_failed"}`},
	} {
		w := serve(t, h, c.req)
		if w.Code != c.status || w.Body.String() != c.body {
			t.Errorf("%s: %d %q", c.name, w.Code, w.Body.String())
		}
		if c.body != "" && w.Header().Get("Content-Type") != "application/json" {
			t.Errorf("%s: content-type %q", c.name, w.Header().Get("Content-Type"))
		}
	}
	if _, err := NewHandler(app, "lgr_whsec_not base64"); err == nil {
		t.Error("a malformed secret was accepted")
	}
}
