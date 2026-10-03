# lingara-apps-java

The official Java kit for building a [Lingara](https://getlingara.com) app: a server that answers Lingara's two signed requests, `app.render` and `app.action`, with a card a learner sees inside Lingara.

It keeps the [app kit contract](../conformance/CONTRACT.md): it verifies every request before anything parses it, decodes it leniently, calls your function, refuses a card Lingara would have to clamp, and answers every failure with a fixed, quiet error.

- Java 17 or newer, server-side JVM.
- One runtime dependency: the Lingara library, `com.getlingara:lingara-java`, whose webhook verifier checks every request. Jackson comes through it.
- One adapter, `JdkHttpHandler`, for the JDK's own `com.sun.net.httpserver`. The module requires `jdk.httpserver` statically, so the rest of the kit runs on an image without it.

## Install

Maven:

```xml
<dependency>
  <groupId>com.getlingara</groupId>
  <artifactId>lingara-apps-java</artifactId>
  <version>VERSION</version>
</dependency>
```

Gradle:

```kotlin
implementation("com.getlingara:lingara-apps-java:VERSION")
```

The kit is a named module, `com.getlingara.apps`, and works on the class path too.

## Quick start

```java
import com.getlingara.apps.Card;
import com.getlingara.apps.JdkHttpHandler;
import com.getlingara.apps.LingaraApp;
import com.sun.net.httpserver.HttpServer;
import java.net.InetSocketAddress;

LingaraApp app = LingaraApp.builder()
    // The app's signing secret, lgr_whsec_…; pass two during a rotation.
    .secrets(System.getenv("LINGARA_APP_SECRET"))
    .render(request -> Card.card()
        .heading("Today's five", 1)
        .term("雨", "yǔ", "rain", "zh")
        .button("Next", "next")
        .build())
    .action("next", request -> Card.card().text("Well done.").build())
    .build();

HttpServer server = HttpServer.create(new InetSocketAddress(8080), 0);
server.createContext("/lingara/render", JdkHttpHandler.of(app));
server.start();
```

`build()` constructs the library's `Webhook`, so a malformed secret fails at startup, not on the first request.

A render function receives the decoded `AppRenderRequest`: `getSubject()` (the learner, as the app's events name them, and the key to hold per-learner state on), `getSlot()`, `getLocale()` and `getContext()`, the slices the learner agreed to share. An action function receives an `AppActionRequest`, which adds `getActionId()` (the pressed button's `action`) and `getCardEtag()`, an opaque token passed to you byte for byte.

A function returns a `Card`, or a card with a tutor note: `Reply.reply(card).tutorNote("…")`.

## Other servers

`LingaraApp.handle(method, headers, body)` is the framework-neutral core: hand it the method, the headers as a `Map<String, List<String>>` and the raw body as an `InputStream`, and write back the `AppResponse` it returns (a status and, when not empty, an `application/json` body). The body must reach it **unparsed**: in Spring, take `@RequestBody byte[]` rather than an object, and in a servlet `request.getInputStream()`. A framework that has already parsed and re-serialised the JSON breaks the signature.

## Cards that keep the rules

Each element method takes exactly its element's fields, and `build()` refuses a card Lingara would otherwise clamp, throwing `CardLimitException` with the first broken rule (`text_length`, `buttons`, `list_items`, …). The kit never cuts, strips or drops on your behalf; `Limits.truncate(text, limit)` gives you Lingara's cut when you want it. Every reply is checked once more, whole, before it is sent: a reply over 32 768 bytes is refused as `reply_too_large`. `Limits` holds every bound.

`Manifest.manifest()` builds the manifest you upload in the console, and refuses one the upload would (`ManifestException`, naming the rule).

## What Lingara sends, and when

- Every request carries `user-agent: Lingara-Apps/1 (+https://getlingara.com/docs/apps/)`. Recognise it in your logs; it is unsigned, so never treat it as proof of anything.
- Lingara waits **3 s** for a render and **5 s** for an action. The kit enforces no timeout; a slow render shows the learner a stale or fallback card instead of yours.
- An action can arrive twice. Make your action functions safe to receive twice.
- Events such as `app.installed` and `app.uninstalled` are webhook deliveries, not app requests: verify and parse them with the library's own `Webhook.verify`.
- The context is read-only data. To read the plan behind a `plan_summary` slice, call the library's `getLessonPlan(planId)`: with your app's own client-credentials client it speaks for you, the owner; for any other learner it needs that learner's token.

## Failures

A function that throws, or a reply that breaks a rule, is answered `500 {"error":"handler_failed"}` and logged through `System.Logger` (`com.getlingara.apps`) with the operation, the install id and the reason, never the body, a secret or a signature. A request that fails verification is `401` with no body; one that does not decode, or names an action you did not register, is `400 {"error":"bad_request"}`.
