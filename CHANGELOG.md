# Changelog

## v0.1.0-alpha.3 — 2026-10-03


Generated from Lingara API 2026-10-affable-towhee (supported) at spec backend@43259e6fb944ab0c816525aee47c0598af8b9503.

### Fixed

- The `lingara-apps` crate's tests pass when the crate is unpacked on its own, as crates.io ships it: the two tests that read the repository's conformance vectors and `LIBRARY_FLOOR` now run from the repository instead of the package. v0.1.0-alpha.2 stopped in staging on this and reached no registry, so this is the kits' first release; what they do is listed under v0.1.0-alpha.2. The kits, and the API spec they are generated from, are otherwise unchanged.

## v0.1.0-alpha.2 — 2026-10-03


Generated from Lingara API 2026-10-affable-towhee (supported) at spec backend@43259e6fb944ab0c816525aee47c0598af8b9503.

### Added

- The first app kits, in seven languages at one version: typescript (`@lingara/apps`), rust (`lingara-apps`), go (`github.com/Spinning-Cat-Studios/lingara_app_framework_sdk/go`), java (`com.getlingara:lingara-apps-java`), kotlin (`com.getlingara:lingara-apps-kotlin`), ruby (`lingara-apps`) and php (`spinningcatstudios/lingara-apps`).
- Each kit verifies a request's signature with its language's `lingara` library (`verifySignature`), decodes it into generated types, calls your render or action function, checks your reply against the card rules, and answers. One adapter per language: `nodeHandler` (Node), an `axum` router behind an optional feature (Rust), `NewHandler` (`net/http`), `JdkHttpHandler` (the JDK's `HttpServer`), `Route.lingaraApp` (Ktor 3), `RackApp` (Rack) and `Psr15Handler` (PSR-15).
- Card, reply and manifest builders that refuse at build time what Lingara would otherwise clamp, `validateReply`, `validateManifest` and `truncate`, held to one answer by `conformance/vectors/card-limits.json` and the new `conformance/vectors/manifest.json`.
- Every kit depends on its language's `lingara` library from 0.1.0-alpha.10, the first release with `verifySignature` (`LIBRARY_FLOOR`).
