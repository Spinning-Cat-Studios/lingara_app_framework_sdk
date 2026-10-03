# @lingara/apps

The official Lingara app kit for Node. An app is a server that answers two
signed requests from Lingara, `app.render` and `app.action`, with a card.
This package verifies each request, decodes it, calls your function and
sends back the card, refusing at build time anything Lingara would otherwise
cut at run time.

It depends on [`@lingara/api`](https://www.npmjs.com/package/@lingara/api)
alone, and reuses its signature verifier. Node 22 or newer (the library's
floor).

## Install

```sh
npm install @lingara/apps
```

## Quick start

```ts
import { createServer } from "node:http";
import { card, lingaraApp, nodeHandler, reply } from "@lingara/apps";

const app = lingaraApp({
  // The app's signing secret, lgr_whsec_…, from its settings. Pass an array
  // of two during a rotation. A malformed secret throws here, at startup.
  secret: process.env.LINGARA_APP_SECRET!,
  render: (request) => {
    const today = card().heading("Today's five", 1).term({ word: "雨", reading: "yǔ", gloss: "rain", lang: "zh" })
      .button("Done", "done").build();
    return request.slot === "home.side" ? reply(today).tutorNote("The learner is reviewing weather words.") : today;
  },
  actions: {
    done: (request) => card().text(`Well done, ${request.subject}`).build(),
  },
});

createServer(nodeHandler(app)).listen(8787);
```

- **`render`** receives the decoded request: `id`, `install_id`, `subject`
  (the per-learner key your events also carry: hold state on it), `slot`,
  `locale` and `context`, a list of the slices the learner agreed to share.
  A slice of a kind this version does not know is skipped.
- **`actions`** are keyed on a button's `action`, which comes back as the
  request's `action_id`. An action also carries `card_etag`, an opaque token
  naming the card the learner pressed; it is passed to you unchanged. Lingara
  may deliver an action twice, so make each one safe to receive twice.
- **A function returns a card** (`card()…build()`), or `reply(card).tutorNote(text)`
  to add a plain-text note for the tutor. Throwing, or returning a card that
  breaks a rule, answers `500 {"error":"handler_failed"}` and is logged with
  the operation, the install id and the reason, never the body or a secret.
- **The builders refuse.** `build()` throws `CardLimitError` with the rule
  (`text_length`, `buttons`, …) and the reply is checked again, encoded, before
  it is sent (at most 32 768 bytes). `truncate(text, limit)` gives you
  Lingara's own cut when you want a shorter string.

## The manifest

```ts
import { manifest } from "@lingara/apps";

const json = manifest().defaultLocale("en").name("Daily five").description("Five words to review.")
  .renderUrl("https://apps.example.com/lingara/render").slots("home.side").context("languages", "plan_summary")
  .toJson(); // Throws ManifestError naming the rule an upload would refuse.
```

Upload the result in the console; the icon is uploaded there too.

## Context and your plan data

The context is read-only. To read the plan behind a `plan_summary` slice, call
`getLessonPlan({ id: slice.plan_id })` through `@lingara/api`. For a private
app, use the app's own client-credentials client, which speaks for its owner;
for any other learner, it needs that learner's O5 token.

## Other frameworks: the raw body

The signature covers the bytes Lingara sent, so the kit must read them before
anything parses them. `nodeHandler(app)` is a plain `node:http` listener, so
it mounts on any route whose body has not been parsed. In Express, mount it
before (or instead of) `express.json()` on its path:

```ts
expressApp.post("/lingara/render", nodeHandler(app));
```

Anything else can call the framework-neutral core, `handle(app, { method, headers, body })`.

## What Lingara sends and expects

- Requests carry `user-agent: Lingara-Apps/1 (+https://getlingara.com/docs/apps/)`.
  Use it to recognise Lingara in your logs; it is unsigned, so it is never a gate.
- Lingara waits **3 s** for a render and **5 s** for an action. A slow render
  shows the learner a stale or fallback card. The kit enforces no time limit.
- Events (`app.installed`, `app.uninstalled`, …) are not app requests: verify
  them with `@lingara/api`'s own `Webhook.verify` on your webhook endpoint.

This kit keeps the [app kit contract](../conformance/CONTRACT.md).
