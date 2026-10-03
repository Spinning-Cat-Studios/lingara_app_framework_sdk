# Conformance

[`CONTRACT.md`](CONTRACT.md) is the rule; this directory makes it
executable.

- `cases/<group>/*.yaml` — one request and its expected reply each. The
  groups are `op` (the operations' happy paths) and `ak1`–`ak5` (the
  contract's behaviours). Adding a case needs no other change.
- `vectors/card-limits.json` — the card rules as data, for each kit's unit
  tests. No wire is involved.
- `host/` — the host. It plays Lingara's part: it starts a kit's fixture
  app, signs each case's request as Lingara does, and judges the reply.

## Running the host

```sh
# Every case against one kit's fixture app.
make conformance-<lang>

# The same, by hand, against any app that implements §F of CONTRACT.md.
cargo run -p lingara-apps-conformance-host -- run --lang mine -- <command that starts the app>

# Every app operation and AK1–AK5 has a case, and every case loads.
make check-conformance-coverage
```

`run` starts the app with `LINGARA_APPS_CONFORMANCE_SECRETS` and
`LINGARA_APPS_CONFORMANCE_PORT=0`, waits up to 30 s for `listening <port>`
on its standard output, sends one unjudged warm-up render (so a cold start
is not charged to the first case), then sends **every** case and prints one
line per case. It exits non-zero when any case fails. `--only <id,…>` runs a
subset for local debugging and is refused when `CI` is set.

A kit whose fixture runs on several stacks sets
`CONFORMANCE_VARIANTS_<lang> := a b` and one
`CONFORMANCE_APP_CMD_<lang>_<variant>` per variant; `make
conformance-<lang>` then runs the host once per variant.

## The case format

```yaml
id: ak1.stale-timestamp-refused      # <group>.<file name>
behaviours: [AK1]                    # AK1–AK5
request:
  operation: app.render              # an x-lingara-app-operations `message`
  body: { json: { … } }              # or { raw: "…" } or { raw_pad: { json: { … }, to_bytes: 65537 } }
  sign:
    secrets: [primary]               # newest first: primary · secondary · unknown
    timestamp_offset_s: -310         # default 0
    extra_entries: ["v2,AAAA"]       # appended to webhook-signature verbatim
  omit_headers: [webhook-signature]  # any of the three webhook-* headers
  tamper: body                       # one body byte changed after signing
  method: POST                       # default
  header_case: lower                 # default; title sends Webhook-Id, …
expect:
  status: 401
  body: empty                        # or { json: … } or { matches_schema: reply }
```

Every key is closed: an unknown key, secret name, header or value fails to
load.

- A **`json`** body is what Lingara would send. The host sets `type` from
  `operation` and a fresh `id`, serialises the body once, and signs and sends
  those bytes. `operation` is required. A `json` body that expects a `200`
  must be a valid request for its operation, or `check-coverage` fails.
- A **`raw`** body is sent and signed byte for byte, for anything Lingara
  would not send: malformed JSON, an unknown `type`, an unknown field or
  slice kind, a foreign etag. Its `webhook-id` is still fresh and need not
  match any `id` inside the bytes. On a `raw` or `raw_pad` case `operation`
  only chooses the time budget, and defaults to `app.render`.
- **`raw_pad`** pads compact JSON with spaces to exactly `to_bytes`, so a
  64 KiB body never has to be written out.
- `{ json: … }` in `expect` is JSON-equal after dropping `null` members.
  `{ matches_schema: reply }` checks the body against the operation's reply
  schema.

The secrets are `lgr_whsec_` followed by the base64 of 32 visibly fake ASCII
bytes: `conformance-app-secret-0001!!!!!` (`primary`), `…0002!!!!!`
(`secondary`) and `…0003!!!!!` (`unknown`, which the fixture app does not
hold). The cases judge ±290 s as accepted and ±310 s as refused, because a
second can pass between signing and checking; the exact 300 s edge belongs
to the client libraries' verifier vectors, which run against a fixed clock.

## What the host judges

For every case: the status; for a `200`, a `content-type` starting
`application/json`, a body of at most 32 768 bytes, and a card that the
card rules accept; the expected body; the fixed error bodies
(`{"error":"bad_request"}` for every `400`, `{"error":"handler_failed"}` for
every `500`, exactly); and Lingara's time budget, 3 s for a render and 5 s
for an action, timed from the first byte sent to the last byte read.

## The card-limit vectors

`vectors/card-limits.json` is `{description, vectors: [{name, reply,
expect}]}`. `reply` is a full `{card, tutor_note?}`; `expect` is
`{"ok": true}` or `{"refused": "<reason>"}`. A vector named
`<limit>-at-limit` is accepted at exactly the limit, and `<limit>-over-limit`
is refused one past it. The host's own validator,
`host/src/limits.rs`, is held to every vector by its tests, so the vectors a
kit reads are known to agree with each other.
