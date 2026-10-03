# Lingara app kits

An **app** is a small web service that puts a card inside Lingara. When a
learner opens a slot your app fills, Lingara sends your app a signed request
with the slot, the learner's locale and only the context the learner agreed
to share. Your app replies with a **card**: a short list of declarative
elements (headings, text, terms, lists, a progress bar, a divider, buttons
and links) that each Lingara client draws natively. No code of yours runs on
a learner's device. When the learner presses a button on your card, Lingara
sends your app an action and draws the card it replies with.

An app kit is the library that answers those requests. It verifies the
signature before it reads anything, decodes the request into typed models,
calls your function, checks your card against the limits Lingara enforces,
and replies. A card a kit sends is never clamped or refused by Lingara.

## Kits

| Language | Package | Install | Server adapter |
|---|---|---|---|
| TypeScript | `@lingara/apps` | `npm install @lingara/apps@next` | `nodeHandler(app)`, a `node:http` listener |
| Rust | `lingara-apps` | `cargo add lingara-apps --features axum` | `lingara_apps::axum::router(app)` (feature `axum`) |
| Go | `github.com/Spinning-Cat-Studios/lingara_app_framework_sdk/go` | `go get github.com/Spinning-Cat-Studios/lingara_app_framework_sdk/go` | `lingaraapps.NewHandler(app, secrets...)`, an `http.Handler` |
| Java | `com.getlingara:lingara-apps-java` | Maven Central | `JdkHttpHandler.of(app)` for the JDK's `HttpServer` |
| Kotlin | `com.getlingara:lingara-apps-kotlin` | Maven Central | `Route.lingaraApp(app)` for Ktor 3 |
| Ruby | `lingara-apps` | `gem install lingara-apps --pre` | `Lingara::Apps::RackApp.new(app)` |
| PHP | `spinningcatstudios/lingara-apps` | `composer require spinningcatstudios/lingara-apps:@alpha` | `Psr15Handler`, a PSR-15 request handler |

Every kit is a pre-release until `1.0.0`, and every kit depends on its
language's `lingara` client library from the version in
[`LIBRARY_FLOOR`](LIBRARY_FLOOR): the verifier is the library's, never the
kit's own. Each kit's README (`<language>/README.md`) has its quick start, and
`snippets/<language>/` holds complete examples that build in CI.

## Building an app without a kit

Everything a kit does is specified in [`conformance/CONTRACT.md`](conformance/CONTRACT.md),
and `conformance/host` is the test Lingara holds every kit to: it plays
Lingara's part, sends your app signed requests, and judges each reply. An
app built without a kit can run the same host against itself.

The request and reply schemas are in `spec/`: `spec/asyncapi.json` is
Lingara's event and app catalogue, and `spec/generator/apps.3.1.json` (with
its OpenAPI 3.0 twin) is the app half of it in a form OpenAPI code generators
read.

## Licence

MIT. See [`LICENSE`](LICENSE).
