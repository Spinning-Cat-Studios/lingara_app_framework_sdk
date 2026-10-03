# lingara-apps (Ruby)

The Ruby kit for building a [Lingara](https://getlingara.com) app: a server
that answers Lingara's two signed requests, `app.render` and `app.action`,
with a card. The kit verifies each request, decodes it, calls your block,
checks the card it returns against Lingara's card rules and sends it. Its
one runtime dependency is the
[`lingara` gem](https://github.com/Spinning-Cat-Studios/lingara_api_clients/tree/main/ruby),
whose webhook verifier checks every signature; the adapter speaks the Rack
protocol without depending on the `rack` gem.

## Install

```sh
gem install lingara-apps
```

or, in a Gemfile, `gem "lingara-apps"`. **Ruby 3.3 or newer**, the library's
floor. While both are pre-releases, add `--pre` (or name the version).

## Quick start

```ruby
# config.ru
require "lingara/apps"

# The app's signing secret, lgr_whsec_…, or an Array of both during a
# rotation. A malformed secret raises ArgumentError here, at startup.
app = Lingara::Apps::App.new(secret: ENV.fetch("LINGARA_APP_SECRET"))

app.render do |request|
  Lingara::Apps.card
    .heading("Today", 1)
    .term(word: "雨", reading: "yǔ", gloss: "rain", lang: "zh")
    .button("Next", "next")
    .build
end

app.action("next") do |request|
  card = Lingara::Apps.card.heading("Nice work", 1).build
  Lingara::Apps.reply(card).tutor_note("The learner finished today's word.")
end

run Lingara::Apps::RackApp.new(app)
```

`Lingara::Apps::RackApp.new(app)` is a Rack endpoint (`#call(env)`), so it
runs under Puma, Falcon or `rackup`, and mounts in Rails
(`mount Lingara::Apps::RackApp.new(app), at: "/lingara"`) or Sinatra. It
reads `rack.input` itself, as raw bytes, before anything parses it: mount it
where no middleware has already read or decoded the body (no JSON
body-parsing middleware in front of it), because the signature covers the
exact bytes Lingara sent.

### What your blocks receive

`Lingara::Apps::AppRenderRequest` and `AppActionRequest` carry `id` (the
`lgr_msg_…` request id), `install_id`, `subject` (the `lgr_sub_…` value your
app's events carry too: key per-learner state on it), `slot`, `locale` and
`context`, and an action adds `action_id` (the button's `action`) and
`card_etag`. `card_etag` is opaque: it tells two presses of one card apart,
and nothing more.

`context` holds the slices the learner agreed to share, each an arm of the
sealed `Lingara::Apps::ContextSlice`: match on `ContextSliceLanguages`,
`ContextSlicePlanSummary`, `ContextSliceReviewDue` or
`ContextSliceTutorTopic`, for example
`in Lingara::Apps::ContextSlicePlanSummary(plan_id:)`. A kind this kit does
not know yet is skipped and logged, never an error. The context is read-only
data. To read the plan behind a `plan_summary` slice, call the library's
`client.get_lesson_plan(plan_id)`. For a private app, that is your app's own
client-credentials client (`Lingara::Client.new(client_id:, client_secret:)`),
which speaks for its owner; for any other learner it needs that learner's
own OAuth token.

### What your blocks return

A `Lingara::Apps::Card` (from `Lingara::Apps.card…build`), or
`Lingara::Apps.reply(card).tutor_note(text)` for a card with a tutor note:
plain text, at most 280 characters, that Lingara's tutor may use. The kit
runs `Lingara::Apps.validate_reply` on every reply. A block that raises, or a
reply that breaks a card rule or encodes to more than 32 768 bytes, is
answered `500 {"error":"handler_failed"}` and logged through the `lingara`
library's logger (the operation, the install id and the reason, never the
body, a secret or a signature). Pass `logger:` (anything with `#warn`, such
as `Rails.logger`) to use your own.

## The builders refuse; they never cut

Lingara cuts, strips or drops whatever in a card breaks its rules. The kit
refuses instead, so that never happens to a card you built:
`CardBuilder#build` raises `Lingara::Apps::CardLimitError`, whose `reason`
names the rule (`:text_length`, `:list_items`, `:link` and so on). The limits
are constants in `Lingara::Apps::Limits`. Characters are Unicode scalar
values (`String#length` on UTF-8). `Lingara::Apps.truncate(text, limit)` gives
a string the clamp's shape (its first `limit − 1` characters and `…`) when
you want one; the kit never calls it for you.

`Lingara::Apps.manifest` builds the manifest you upload in the console;
`build` raises `Lingara::Apps::ManifestError`, whose `rule` names the rule it
breaks, and `to_json` writes `manifest.json`. A bare `name("…")` is the name
in the default locale.

## Things to know

- **The user agent.** Lingara's requests carry
  `user-agent: Lingara-Apps/1 (+https://getlingara.com/docs/apps/)`, so you
  can recognise them in your logs. It is unsigned, so never use it as a gate:
  the signature is the gate.
- **Time budgets.** Lingara waits 3 s for a render and 5 s for an action, and
  never retries. A slow render shows the learner a stale or fallback card.
  The kit enforces no timeout of its own.
- **Actions may arrive twice.** The kit stores nothing, so make each action
  block safe to run twice.
- **Events are the library's.** `app.installed`, `app.uninstalled` and the
  other events reach a webhook endpoint, verified by the library's own
  `Lingara::Events::Webhook#verify` and its event types, not by this kit.

## The contract

Every Lingara app kit keeps one contract, checked by one conformance host
against a fixture app in each language:
[`../conformance/CONTRACT.md`](../conformance/CONTRACT.md).
