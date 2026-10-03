use std::path::PathBuf;
use std::time::Duration;

use serde_json::{Value, json};

use crate::case::Case;
use crate::exchange::Reply;
use crate::validate::{Schemas, judge};

pub fn view() -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../spec/generator/apps.3.1.json");
    serde_json::from_str(&std::fs::read_to_string(path).expect("the committed view")).unwrap()
}

fn case(expect: &str) -> Case {
    let yaml = format!(
        "id: ak3.judged\nbehaviours: [AK3]\nrequest:\n  operation: app.render\n  body: {{ json: {{}} }}\nexpect:\n{expect}\n"
    );
    serde_yaml::from_str(&yaml).unwrap()
}

fn reply(status: u16, content_type: &str, body: Vec<u8>, elapsed_ms: u64) -> Reply {
    let headers = vec![("content-type".to_string(), content_type.to_string())];
    Reply { status, headers, body, elapsed: Duration::from_millis(elapsed_ms) }
}

fn card() -> Value {
    json!({ "card": { "elements": [{ "type": "heading", "text": "home.side", "level": 1 }] } })
}

fn json_reply(v: &Value) -> Reply {
    reply(200, "application/json; charset=utf-8", serde_json::to_vec(v).unwrap(), 10)
}

fn misses(case: &Case, reply: &Reply) -> Vec<String> {
    let schemas = Schemas::from_view(&view()).expect("the view compiles");
    let out = judge(case, &schemas, reply, Duration::from_secs(3));
    assert!(out.iter().all(|m| m.starts_with("ak3.judged: ")), "every mismatch names its case: {out:?}");
    out
}

fn one_containing(out: &[String], needle: &str) {
    assert!(out.iter().any(|m| m.contains(needle)), "no mismatch mentions {needle:?}: {out:?}");
}

/// ADR 30.9.26al AC11: each of these is a mismatch naming the case: a reply
/// outside the reply schema; a wrong status; a `200` with another
/// content type or over 32 KiB; a `200` whose card `validate_reply`
/// refuses; a reply slower than its operation's budget; and an AK4 body
/// that carries anything but `{"error":"handler_failed"}`.
#[test]
fn every_mismatch_names_its_case() {
    let schema = case("  status: 200\n  body: { matches_schema: reply }");
    assert!(misses(&schema, &json_reply(&card())).is_empty(), "a good reply passes");

    let mut extra = card();
    extra["outcome"] = json!("ok");
    one_containing(&misses(&schema, &json_reply(&extra)), "outside the reply schema");

    let mut wrong = json_reply(&card());
    wrong.status = 201;
    one_containing(&misses(&schema, &wrong), "status 201 where 200 was expected");

    let plain = reply(200, "text/plain", serde_json::to_vec(&card()).unwrap(), 10);
    one_containing(&misses(&schema, &plain), "not application/json");

    let mut padded = serde_json::to_vec(&card()).unwrap();
    padded.resize(32769, b' ');
    one_containing(&misses(&schema, &reply(200, "application/json", padded, 10)), "over 32768");

    let mut clamped = card();
    clamped["card"]["elements"][0]["text"] = json!("a".repeat(81));
    one_containing(&misses(&schema, &json_reply(&clamped)), "would clamp: text_length");

    let mut slow = json_reply(&card());
    slow.elapsed = Duration::from_millis(3001);
    one_containing(&misses(&schema, &slow), "over the relay's 3000 ms budget");

    let quiet = case("  status: 500\n  body: { json: { error: handler_failed } }");
    let good = reply(500, "application/json", br#"{"error":"handler_failed"}"#.to_vec(), 10);
    assert!(misses(&quiet, &good).is_empty(), "the fixed body passes");
    let chatty = reply(500, "application/json", br#"{"error":"handler_failed","message":"boom"}"#.to_vec(), 10);
    one_containing(&misses(&quiet, &chatty), "must be exactly");
    let null_extra = reply(500, "application/json", br#"{"error":"handler_failed","stack":null}"#.to_vec(), 10);
    one_containing(&misses(&quiet, &null_extra), "must be exactly");
}
