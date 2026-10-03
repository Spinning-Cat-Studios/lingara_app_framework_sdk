# lingara-apps

The official Lingara app kit for Rust. An app is a server that answers two
signed requests from Lingara, `app.render` and `app.action`, with a card.
This crate verifies each request, decodes it, calls your function and sends
back the card, refusing at build time anything Lingara would otherwise cut at
run time.

It depends on the [`lingara`](https://crates.io/crates/lingara) library and
reuses its signature verifier; its only other dependencies are the `serde`,
`serde_json` and `log` the library already carries. The axum adapter is the
optional `axum` feature, off by default. Rust 1.87 or newer (the library's
floor), edition 2024.

## Install

```sh
cargo add lingara-apps --features axum
cargo add axum@0.8
cargo add tokio --features macros,net,rt-multi-thread
```

`lingara_apps::lingara` re-exports the library, so one dependency gives you
both.

## Quick start

```rust
use lingara_apps::{App, AppActionRequest, AppRenderRequest, AppSlotName, BoxError, Term, card, reply};

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    // The app's signing secret, lgr_whsec_…, from its settings. Pass two
    // during a rotation. A malformed secret fails here, at startup.
    let secret = std::env::var("LINGARA_APP_SECRET")?;
    let app = App::new([secret], |request: AppRenderRequest| async move {
        let today = card()
            .heading("Today's five", 1)
            .term(Term::new("雨").reading("yǔ").gloss("rain").lang("zh"))
            .button("Done", "done")
            .build()?;
        Ok::<_, BoxError>(if request.slot == AppSlotName::HomeSide {
            reply(today).tutor_note("The learner is reviewing weather words.")?
        } else {
            today.into()
        })
    })?
    .action("done", |request: AppActionRequest| async move {
        Ok::<_, BoxError>(card().text(format!("Well done, {}", request.subject)).build()?)
    });

    let routes = axum::Router::new().nest("/lingara/render", lingara_apps::axum::router(app));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
    axum::serve(listener, routes).await?;
    Ok(())
}
```

- **The render function** receives the decoded `AppRenderRequest`: `id`,
  `install_id`, `subject` (the per-learner key your events also carry: hold
  state on it), `slot`, `locale` and `context`, a list of the `ContextSlice`s
  the learner agreed to share. A slice of a kind this version does not know is
  skipped, and an unknown field is ignored.
- **Actions** are registered on a button's `action`, which comes back as the
  request's `action_id`. An action also carries `card_etag`, an opaque token
  naming the card the learner pressed; it is passed to you unchanged. Lingara
  may deliver an action twice, so make each one safe to receive twice.
- **A function returns a card** (`card()…build()?`), or
  `reply(card).tutor_note(text)?` to add a plain-text note for the tutor.
  Returning an error, panicking, or returning a card that breaks a rule
  answers `500 {"error":"handler_failed"}` and is logged through the `log`
  facade (target `lingara_apps`) with the operation, the install id and the
  reason, never the body or a secret.
- **The builders refuse.** `build()` returns `CardLimitError` with the rule
  (`text_length`, `buttons`, …), and the reply is checked again, encoded, before
  it is sent (at most 32 768 bytes). `validate_reply` is the same check over
  any `serde_json::Value`. `truncate(text, limit)` gives you Lingara's own cut
  when you want a shorter string.

## The manifest

```rust
use lingara_apps::{AppSlotName, ContextSliceKind, manifest};

let manifest = manifest()
    .default_locale("en")
    .name("Daily five")
    .description("Five words to review.")
    .render_url("https://apps.example.com/lingara/render")
    .slots([AppSlotName::HomeSide])
    .context([ContextSliceKind::Languages, ContextSliceKind::PlanSummary])
    .build()?; // ManifestError names the rule an upload would refuse.
std::fs::write("manifest.json", manifest.to_json())?;
```

A bare string name or description is the default locale's; pass
`[("en", "…"), ("zh-Hans", "…")]` for more. Upload the result in the console;
the icon is uploaded there too.

## Context and your plan data

The context is read-only. To read the plan behind a `plan_summary` slice, call
`client.get_lesson_plan(&slice.plan_id)` through the library. For a private
app, use the app's own client-credentials client, which speaks for its owner;
for any other learner, it needs that learner's O5 token.

## Other frameworks: the raw body

The signature covers the bytes Lingara sent, so the kit must read them before
anything parses them. The axum router takes the raw body itself; do not put a
`Json` extractor in front of it. Any other server can call the
framework-neutral core: `App::check_method(method)` before reading the body,
then `app.handle(&headers, &body).await` with at most 65 536 bytes of it, and
write the `Response`'s status, `content_type` and body as they are. `headers`
is anything that implements the library's `WebhookHeaders` (an `http`
`HeaderMap`, or a `HashMap<String, String>`).

## What Lingara sends and expects

- Requests carry `user-agent: Lingara-Apps/1 (+https://getlingara.com/docs/apps/)`.
  Use it to recognise Lingara in your logs; it is unsigned, so it is never a gate.
- Lingara waits **3 s** for a render and **5 s** for an action. A slow render
  shows the learner a stale or fallback card. The kit enforces no time limit.
- Events (`app.installed`, `app.uninstalled`, …) are not app requests: verify
  them with the library's own `Webhook::verify` on your webhook endpoint.

This kit keeps the [app kit contract](../conformance/CONTRACT.md).
