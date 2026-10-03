use crate::case::{Body, Case, ExpectBody, HeaderCase, Method, SecretName, Tamper, WebhookHeader};

fn parse(yaml: &str) -> Result<Case, String> {
    serde_yaml::from_str(yaml).map_err(|e| e.to_string())
}

/// D5's example case, every key spelled out.
const EXAMPLE: &str = r#"
id: ak1.stale-timestamp-refused
behaviours: [AK1]
request:
  operation: app.render
  body: { json: { slot: plans.empty_detail } }
  sign:
    secrets: [primary, secondary]
    timestamp_offset_s: -310
    extra_entries: ["v2,AAAA"]
  omit_headers: [webhook-signature]
  tamper: body
  method: POST
  header_case: title
expect:
  status: 401
  body: empty
"#;

/// ADR 30.9.26al AC25: every key in D5's example deserialises; a secret
/// name outside primary · secondary · unknown and an `omit_headers` entry
/// outside the three `webhook-*` names are each refused; an unknown key
/// fails; `raw_pad` yields exactly `to_bytes` bytes of valid JSON and
/// refuses JSON already longer than that.
#[test]
fn the_case_format_is_closed() {
    let case = parse(EXAMPLE).expect("D5's example parses");
    let r = &case.request;
    assert_eq!(r.sign.secrets, [SecretName::Primary, SecretName::Secondary]);
    assert_eq!((r.sign.timestamp_offset_s, r.sign.extra_entries.as_slice()), (-310, ["v2,AAAA".to_string()].as_slice()));
    assert_eq!(r.omit_headers, [WebhookHeader::Signature]);
    assert_eq!((r.tamper, r.method, r.header_case), (Some(Tamper::Body), Method::Post, HeaderCase::Title));
    assert!(matches!(case.expect.body, Some(ExpectBody::Empty)));

    let bad_secret = EXAMPLE.replace("[primary, secondary]", "[primary, tertiary]");
    assert!(parse(&bad_secret).unwrap_err().contains("tertiary"));
    let bad_header = EXAMPLE.replace("[webhook-signature]", "[x-signature]");
    assert!(parse(&bad_header).unwrap_err().contains("x-signature"));
    let unknown = EXAMPLE.replace("  tamper: body\n", "  tamper: body\n  retries: 3\n");
    assert!(parse(&unknown).unwrap_err().contains("retries"));
    let unknown_expect = EXAMPLE.replace("  body: empty", "  body: empty\n  headers: {}");
    assert!(parse(&unknown_expect).unwrap_err().contains("headers"));

    let padded = parse("id: a.b\nbehaviours: [AK2]\nrequest:\n  body:\n    raw_pad: { json: { type: app.render }, to_bytes: 100 }\nexpect: { status: 413 }\n").unwrap();
    let Body::RawPad(pad) = &padded.request.body else { panic!("raw_pad") };
    let bytes = pad.bytes().expect("pads");
    assert_eq!(bytes.len(), 100);
    assert_eq!(serde_json::from_slice::<serde_json::Value>(&bytes).unwrap()["type"], "app.render");
    let short = crate::case::RawPad { json: serde_json::json!({ "type": "app.render" }), to_bytes: 5 };
    assert!(short.bytes().unwrap_err().contains("over to_bytes"));
}
