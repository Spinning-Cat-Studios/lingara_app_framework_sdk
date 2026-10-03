# lingara-apps-kotlin

The official Kotlin kit for building a [Lingara](https://getlingara.com) app: a server that answers Lingara's two signed requests, `app.render` and `app.action`, with a card a learner sees inside Lingara.

It keeps the [app kit contract](../conformance/CONTRACT.md): it verifies every request before anything parses it, decodes it leniently, calls your function, refuses a card Lingara would have to clamp, and answers every failure with a fixed, quiet error.

- Kotlin 2.2 or newer on JVM 17 or newer.
- One runtime dependency: the Lingara library, `com.getlingara:lingara-kotlin`, whose webhook verifier checks every request. kotlinx-serialization and coroutines come through it.
- One adapter, `Route.lingaraApp(app)` for Ktor 3. Ktor is not a dependency of the kit: bring your own Ktor 3, and an app without Ktor never loads the adapter.

## Install

Gradle:

```kotlin
implementation("com.getlingara:lingara-apps-kotlin:VERSION")
```

Maven:

```xml
<dependency>
  <groupId>com.getlingara</groupId>
  <artifactId>lingara-apps-kotlin</artifactId>
  <version>VERSION</version>
</dependency>
```

## Quick start

```kotlin
import com.getlingara.apps.kotlin.card
import com.getlingara.apps.kotlin.ktor.lingaraApp
import com.getlingara.apps.kotlin.lingaraApp
import io.ktor.server.cio.CIO
import io.ktor.server.engine.embeddedServer
import io.ktor.server.routing.route
import io.ktor.server.routing.routing

val app = lingaraApp {
    // The app's signing secret, lgr_whsec_…; pass two during a rotation.
    secrets(System.getenv("LINGARA_APP_SECRET"))
    render { request ->
        card {
            heading("Today's five", 1)
            term("雨", reading = "yǔ", gloss = "rain", lang = "zh")
            button("Next", "next")
        }
    }
    action("next") { request -> card { text("Well done.") } }
}

embeddedServer(CIO, port = 8080) {
    routing { route("/lingara/render") { lingaraApp(app) } }
}.start(wait = true)
```

`lingaraApp {}` constructs the library's `Webhook`, so a malformed secret fails at startup, not on the first request.

Render and action functions are `suspend`. A render function receives the decoded `AppRenderRequest`: `subject` (the learner, as the app's events name them, and the key to hold per-learner state on), `slot`, `locale` and `context`, the slices the learner agreed to share. An action function receives an `AppActionRequest`, which adds `actionId` (the pressed button's `action`) and `cardEtag`, an opaque token passed to you byte for byte.

A function returns a `Card`, or a card with a tutor note: `reply(card).tutorNote("…")`.

## The raw body

`Route.lingaraApp` reads the request body itself, as bytes, so Ktor's `ContentNegotiation` never parses it. Do not put a route that receives a parsed object in front of it: a re-serialised body breaks the signature. On any other server, call `app.handle(method, headers, readBody)` with the raw bytes and write back the `AppResponse` it returns (a status and, when not empty, an `application/json` body).

## Cards that keep the rules

Each element method in `card {}` takes exactly its element's fields, and list items are `item.text(…)` and `item.term(…)`. The card is refused when Lingara would otherwise clamp it: `CardLimitException` names the first broken rule (`text_length`, `buttons`, `list_items`, …). The kit never cuts, strips or drops on your behalf; `truncate(text, limit)` gives you Lingara's cut when you want it. Every reply is checked once more, whole, before it is sent: a reply over 32 768 bytes is refused as `reply_too_large`. `Limits` holds every bound.

`manifest {}` builds the manifest you upload in the console, and refuses one the upload would (`ManifestException`, naming the rule).

## What Lingara sends, and when

- Every request carries `user-agent: Lingara-Apps/1 (+https://getlingara.com/docs/apps/)`. Recognise it in your logs; it is unsigned, so never treat it as proof of anything.
- Lingara waits **3 s** for a render and **5 s** for an action. The kit enforces no timeout; a slow render shows the learner a stale or fallback card instead of yours.
- An action can arrive twice. Make your action functions safe to receive twice.
- Events such as `app.installed` and `app.uninstalled` are webhook deliveries, not app requests: verify and parse them with the library's own `Webhook.verify`.
- The context is read-only data. To read the plan behind a `plan_summary` slice, call the library's `getLessonPlan(planId)`: with your app's own client-credentials client it speaks for you, the owner; for any other learner it needs that learner's token.

## Failures

A function that throws, or a reply that breaks a rule, is answered `500 {"error":"handler_failed"}` and logged through `System.Logger` (`com.getlingara.apps.kotlin`) with the operation, the install id and the reason, never the body, a secret or a signature. A request that fails verification is `401` with no body; one that does not decode, or names an action you did not register, is `400 {"error":"bad_request"}`.
