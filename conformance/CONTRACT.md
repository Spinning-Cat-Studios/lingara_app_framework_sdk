# The Lingara app kit contract

Every official Lingara app kit keeps this contract. It fixes the behaviour,
down to the header name, the byte and the second; the spelling of an API
(function, option and class names) is each language's own. A kit's README
links this file.

The key words MUST, MUST NOT, SHOULD and MAY are to be read as in RFC 2119.
The cases under [`cases/`](cases/) make the contract executable:
[`host/`](host/) plays Lingara's part, sends every case to a kit's fixture
app (§F) and judges each reply, and a kit that cannot pass one does not ship.
The direction is the reverse of a client library's: here Lingara calls the
app, so the host drives every case itself and there is no way to skip one.
[`README.md`](README.md) says how the cases and the host work.

Decided by ADR 30.9.26al. An app built without a kit can hold itself to the
same contract by running the host against itself.

## Protocol facts the contract relies on

These are properties of Lingara's relay that the rules below depend on.

- **Operations.** Two, each a signed `POST` to the app's `render_url` that
  expects a card back: `app.render` (Lingara wants a card for a slot) and
  `app.action` (the learner pressed a button on the app's card). The schemas
  are in `../spec/generator/apps.3.1.json`, generated from
  `../spec/asyncapi.json`; `x-lingara-app-operations` lists the two
  operations and `x-lingara-unions` the three tagged unions.
- **The request body** is `{type, id, install_id, subject, slot, locale,
  context}`, and an action adds `action_id` and `card_etag`. `type` is
  `app.render` or `app.action`. `id` is `lgr_msg_` followed by 32 lowercase
  hex digits. `context` is a list of slices tagged on `kind`; a slice is
  present only when the learner agreed to share it. `card_etag` names the
  card the learner saw; it is an opaque token, not something to verify.
- **The headers.** Exactly five: `webhook-id` (equal to the body's `id`),
  `webhook-timestamp` (Unix seconds), `webhook-signature`,
  `content-type: application/json` and `user-agent: Lingara-Apps/1
  (+https://getlingara.com/docs/apps/)`. The body is serialised once, and
  those bytes are signed and sent, so a kit MUST verify the bytes it
  received, never a re-serialisation.
- **The signature** is Standard Webhooks: `v1,<base64>` of HMAC-SHA256 over
  the UTF-8 bytes of `webhook-id`, `.`, `webhook-timestamp`, `.` and the raw
  body. The key is the standard, padded base64 after the secret's
  `lgr_whsec_` prefix. During a rotation the app holds two secrets and
  `webhook-signature` carries one `v1` entry per secret, newest first,
  separated by single spaces.
- **The reply** Lingara accepts is `200`, a `content-type` that starts with
  `application/json`, and at most **32 768 bytes** of body. It is
  `{card: {elements: [...]}, tutor_note?}` and nothing else. Lingara waits
  **3 s** for a render and **5 s** for an action, and never retries; a reply
  that misses any of this is replaced by a fallback card.
- **The clamp.** Lingara cuts, strips or drops whatever in a card breaks the
  card rules below before any client draws it. A kit refuses such a card
  instead (AK3), so the clamp never fires on a kit-built card.

## AK1 — verify before anything

A kit MUST read the raw body as bytes and verify the signature before it
parses anything or calls the developer's code.

- Header names are case-insensitive.
- A missing `webhook-id`, `webhook-timestamp` or `webhook-signature`, or a
  timestamp that is not one or more ASCII digits, fails verification.
- A timestamp more than **300 s** before or after the kit's clock fails
  verification. Exactly 300 s passes. The tolerance is not configurable.
- `webhook-signature` is split on single spaces. An entry with a version
  prefix other than `v1` is skipped, and so is one that does not decode. Each
  `v1` entry is compared, in **constant time**, with the expected HMAC under
  each of the app's secrets; any match passes.
- A secret MUST be `lgr_whsec_` followed by standard, padded base64. A kit
  refuses any other secret when it is configured, not when a request arrives.
- A failure is **`401` with an empty body**, and the developer's code is
  never called.

The algorithm is the one the Lingara client libraries' webhook verifier
implements (their `verifySignature`), and a kit SHOULD reuse its language's
library rather than reimplement it.

## AK2 — decode and dispatch

- A method other than `POST` is **`405`**, before verification.
- A body over **65 536 bytes** is **`413`**, before verification. A kit MAY
  decide this from `Content-Length` without reading the body; Lingara always
  sends one.
- After verification, the body's `type` names the operation, and the body
  decodes into the kit's generated request type. An unknown `type` or a body
  that does not decode is **`400`** with exactly `{"error":"bad_request"}`.
- **Forward compatibility.** An unknown request field is ignored, and a
  `context` slice of an unknown `kind` is skipped, so a kit built on an older
  schema still answers a newer Lingara. Neither is a `400`.
- A render goes to the developer's render function. An action goes to the
  function registered for its `action_id`; an `action_id` with no function is
  **`400`** with exactly `{"error":"bad_request"}`.
- `card_etag` reaches the developer's function byte for byte. A kit never
  parses, checks or compares it.

## AK3 — reply

- A kit replies **`200`** with `content-type: application/json` and the
  generated reply type, `{card, tutor_note?}`, and nothing else: `outcome`,
  `etag`, `unchanged` and `fallback` are Lingara's words to its own clients,
  never an app's. The reply is always a card.
- The kit encodes the reply; it never hands the developer a writer.
- Before sending, a kit validates the reply against the card rules (the same
  function its card builders run) and checks that the encoded body is at most
  32 768 bytes.
- **Size is measured on the bytes sent**, so a kit encodes compact JSON with
  raw UTF-8: no `\uXXXX` escapes beyond JSON's own, no HTML or `/` escaping,
  and an absent `tutor_note` omitted rather than `null`.

### The card rules

A **character** is a Unicode scalar value: never a grapheme cluster and never
a UTF-16 unit. **Empty** means empty after trimming Unicode `White_Space`
(U+3000 alone is empty; trimming only U+0000–U+0020 is wrong). A pattern
matches the **whole** string, never one line of it.

| Field | Rule |
|---|---|
| `heading.text` | 1–80 characters, not empty; `heading.level` is 1 or 2 |
| `text.text`, a list item's `text` | 1–600 characters, not empty; `\n` is allowed |
| `term.word` | 1–60 characters, not empty |
| `term.reading` | at most 120 characters, when present |
| `term.gloss` | at most 160 characters, when present |
| `progress.value` | 0 to 1 inclusive; `progress.label` 1–60 characters, not empty |
| `button.label` | 1–32 characters, not empty |
| `button.action` | matches `^[A-Za-z0-9_.:-]{1,64}$` |
| `link.label` | 1–60 characters, not empty |
| `link.url` | `https`, no user information, a host name (never an IP address), at most 2 048 bytes |
| `lang` (text, term, list items) | matches `^[A-Za-z]{2,3}(-[A-Za-z0-9]{2,8}){0,3}$`, when present |
| `list.items` | 1–20 items |
| buttons | at most 4 in a card |
| `card.elements` | 1–24 elements |
| `tutor_note` | at most 280 characters once `\n` becomes a space and the ends are trimmed |
| every string | no C0 or C1 control character (U+0000–U+001F, U+007F–U+009F), no bidi override (U+202A–U+202E) and no bidi isolate (U+2066–U+2069); `\n` only where the table allows it |
| the encoded reply | at most 32 768 bytes |

A reply that breaks a rule is refused with one **reason**. When it breaks
several, the reason is the first of these, in this order:

`control_chars`, `text_length`, `heading_level`, `progress_range`, `lang`,
`link`, `button_action`, `buttons`, `list_items`, `empty_element`,
`elements`, `empty_card`, `tutor_note_length`, `reply_too_large`.

The first eleven are the names Lingara's clamp gives the same rules.
[`vectors/card-limits.json`](vectors/card-limits.json) holds a boundary pair
for every limit and at least one refusal for every reason, and every kit's
validator MUST agree with every vector.

## AK4 — fail closed and quiet

- A developer function that raises, or a reply that fails AK3's validation,
  is **`500`** with exactly `{"error":"handler_failed"}`.
- No error body ever carries a message, a stack trace, a secret or a
  signature. A kit logs the failure through its language's standard logging
  facility, never into the response.

## AK5 — rotate

- A kit accepts one or more secrets (two in practice). During a rotation a
  request signed by either passes. A kit does not enforce a maximum.

## §F — the fixture app

A case cannot tell a developer's function what to do, so every kit
implements this one app in `<lang>/conformance/`, with its own kit, and every
reply below is exact:

- **render** returns `{type: heading, text: <slot>, level: 1}`, then
  `{type: text, text: <kind>}` for each slice received, in the order
  received. On the slot `home.side`, it also returns
  `tutor_note: "fixture note"`.
- **action `inc`** returns `{type: progress, value: 0.5, label: "inc"}` and
  `{type: text, text: <card_etag as received>}`.
- **action `boom`** raises.
- **action `overflow`** returns one `list` of 21 `text` items, `"1"` to
  `"21"` (AK4: `list_items`).
- **action `huge`** returns 24 `text` elements of 600 `漢` each. Each passes
  the card rules, but the reply is over 32 768 bytes (AK4:
  `reply_too_large`).

The fixture reads its secrets from `LINGARA_APPS_CONFORMANCE_SECRETS`
(`primary` and `secondary`, comma-separated; the order carries no meaning)
and its port from `LINGARA_APPS_CONFORMANCE_PORT` (`0`: any free port). It
listens on `127.0.0.1` and prints `listening <port>` as its **first** line on
standard output.
