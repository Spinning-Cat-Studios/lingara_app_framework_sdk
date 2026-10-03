# lingaraapps (Go)

The Go kit for building a [Lingara](https://getlingara.com) app: a server that
answers Lingara's two signed requests, `app.render` and `app.action`, with a
card. The kit verifies each request, decodes it, calls your function, checks
the card it returns against Lingara's card rules and sends it. It is built on
`net/http` and the standard library; its one dependency is the
[Lingara Go library](https://github.com/Spinning-Cat-Studios/lingara_api_clients/tree/main/go),
whose own `require` block is empty.

## Install

```sh
go get github.com/Spinning-Cat-Studios/lingara_app_framework_sdk/go
```

**Go 1.23 or newer**, the library's floor. Import it as `lingaraapps`:

```go
import lingaraapps "github.com/Spinning-Cat-Studios/lingara_app_framework_sdk/go"
```

## Quick start

```go
render := func(ctx context.Context, req lingaraapps.AppRenderRequest) (lingaraapps.Replier, error) {
	return lingaraapps.NewCard().
		Heading("Today", 1).
		Term(lingaraapps.Term{Word: "雨", Reading: "yǔ", Gloss: "rain", Lang: "zh"}).
		Button("Next", "next").
		Build()
}
next := func(ctx context.Context, req lingaraapps.AppActionRequest) (lingaraapps.Replier, error) {
	card, err := lingaraapps.NewCard().Heading("Nice work", 1).Build()
	if err != nil {
		return nil, err
	}
	return lingaraapps.Reply(card).TutorNote("The learner finished today's word."), nil
}

// The app's signing secret, lgr_whsec_…, or both during a rotation. A
// malformed secret is an error here, at startup.
handler, err := lingaraapps.NewHandler(lingaraapps.App{
	Render:  render,
	Actions: map[string]lingaraapps.ActionFunc{"next": next},
}, os.Getenv("LINGARA_APP_SECRET"))
if err != nil {
	log.Fatal(err)
}
http.Handle("/lingara", handler)
log.Fatal(http.ListenAndServe(":8080", nil))
```

`NewHandler(app, secrets...) (http.Handler, error)` is a plain `net/http`
handler, so it mounts on any router that takes one. It reads the body itself,
as raw bytes, before anything parses it: mount it where no middleware has
already consumed or decoded the body, because the signature covers the exact
bytes Lingara sent.

### What your functions receive

`AppRenderRequest` and `AppActionRequest` carry `ID` (the `lgr_msg_…` request
id), `InstallID`, `Subject` (the `lgr_sub_…` value your app's events carry
too: key per-learner state on it), `Slot`, `Locale` and `Context`, and an
action adds `ActionID` (the button's `action`) and `CardEtag`. `CardEtag` is
opaque: it tells two presses of one card apart, and nothing more.

`Context` holds the slices the learner agreed to share, as the sealed
`ContextSlice` interface: type-switch on `ContextSliceLanguages`,
`ContextSlicePlanSummary`, `ContextSliceReviewDue` or `ContextSliceTutorTopic`.
A kind this kit does not know yet is skipped and logged, never an error. The
context is read-only data. To read the plan behind a `plan_summary` slice,
call the library's `client.GetLessonPlan(ctx, s.PlanID)`. For a private app,
that is your app's own client-credentials client (`lingara.New` with
`lingara.WithClientCredentials`), which speaks for its owner; for any other
learner it needs that learner's own OAuth token.

### What your functions return

A `Card` (from `NewCard()…Build()`), or `Reply(card).TutorNote(text)` for a
card with a tutor note: plain text, at most 280 characters, that Lingara's
tutor may use. The handler runs `ValidateReply` on every reply. A function
that returns an error or panics, or a reply that breaks a card rule or
encodes to more than 32 768 bytes, is answered `500 {"error":"handler_failed"}`
and logged through `log/slog` (the operation, the install id and the reason,
never the body, a secret or a signature). Set `App.Logger` to use your own.

## The builders refuse; they never cut

Lingara cuts, strips or drops whatever in a card breaks its rules. The kit
refuses instead, so that never happens to a card you built:
`CardBuilder.Build` returns a `*CardLimitError` naming the rule
(`text_length`, `list_items`, `link` and so on). The limits are constants in
`limits.go`. Characters are Unicode scalar values. `Truncate(text, limit)`
gives a string the clamp's shape (its first `limit − 1` characters and `…`)
when you want one; the kit never calls it for you.

`NewManifest()` builds the manifest you upload in the console;
`Build` returns a `*ManifestError` naming the rule it breaks, and `ToJSON`
writes `manifest.json`. A bare `Name("…")` is the name in the default locale.

## Things to know

- **The user agent.** Lingara's requests carry
  `user-agent: Lingara-Apps/1 (+https://getlingara.com/docs/apps/)`, so you
  can recognise them in your logs. It is unsigned, so never use it as a gate:
  the signature is the gate.
- **Time budgets.** Lingara waits 3 s for a render and 5 s for an action, and
  never retries. A slow render shows the learner a stale or fallback card.
  The kit enforces no timeout of its own.
- **Actions may arrive twice.** The kit stores nothing, so make each action
  function safe to run twice.
- **Events are the library's.** `app.installed`, `app.uninstalled` and the
  other events reach a webhook endpoint, verified by the library's own
  `Webhook.Verify` and its event types, not by this kit.

## The contract

Every Lingara app kit keeps one contract, checked by one conformance host
against a fixture app in each language:
[`../conformance/CONTRACT.md`](../conformance/CONTRACT.md).
