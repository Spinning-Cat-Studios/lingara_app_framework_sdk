use std::path::PathBuf;

use serde_json::Value;

use crate::limits::{Reason, validate_reply};

/// Every C6 limit, by the vector-name prefix of its boundary pair.
const LIMITS: [&str; 21] = [
    "heading", "text", "term-word", "term-reading", "term-gloss", "progress-label", "button-label", "link-label",
    "list-item-text", "list-item-word", "tutor-note", "heading-astral", "heading-combining", "heading-level",
    "progress-value", "buttons", "list-items", "elements", "button-action-length", "link-url-bytes", "reply-size",
];

pub fn vectors() -> Vec<Value> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../vectors/card-limits.json");
    let doc: Value = serde_json::from_str(&std::fs::read_to_string(path).expect("the vector file")).unwrap();
    doc["vectors"].as_array().expect("a vectors array").clone()
}

fn expected(v: &Value) -> Result<(), Reason> {
    match (&v["expect"]["ok"], v["expect"]["refused"].as_str()) {
        (Value::Bool(true), None) => Ok(()),
        (Value::Null, Some(reason)) => Err(Reason::parse(reason).unwrap_or_else(|| panic!("{}: unknown reason {reason}", v["name"]))),
        _ => panic!("{}: expect is neither {{ok: true}} nor {{refused}}", v["name"]),
    }
}

fn named<'a>(vectors: &'a [Value], name: &str) -> &'a Value {
    vectors.iter().find(|v| v["name"] == name).unwrap_or_else(|| panic!("no vector named {name}"))
}

/// ADR 30.9.26al AC14: every vector's `expect` equals `validate_reply`'s
/// answer; every reason has a refused vector; every C6 limit has an ok
/// vector at the limit and a refused one past it; the astral-plane and
/// combining vectors count scalar values, and a U+3000-only heading is
/// `empty_element`.
#[test]
fn every_vector_agrees_with_the_reference() {
    let vectors = vectors();
    let mut names = std::collections::BTreeSet::new();
    for v in &vectors {
        assert!(names.insert(v["name"].as_str().unwrap()), "{} is named twice", v["name"]);
        assert_eq!(validate_reply(&v["reply"]), expected(v), "{}", v["name"]);
    }

    for reason in Reason::ALL {
        let refused = vectors.iter().any(|v| v["expect"]["refused"] == reason.as_str());
        assert!(refused, "no vector is refused as {}", reason.as_str());
    }

    for limit in LIMITS {
        assert_eq!(expected(named(&vectors, &format!("{limit}-at-limit"))), Ok(()), "{limit}-at-limit");
        assert!(expected(named(&vectors, &format!("{limit}-over-limit"))).is_err(), "{limit}-over-limit");
    }

    let astral = named(&vectors, "heading-astral-at-limit")["reply"]["card"]["elements"][0]["text"].as_str().unwrap();
    assert_eq!((astral.chars().count(), astral.encode_utf16().count()), (80, 160));
    let combining = named(&vectors, "heading-combining-at-limit")["reply"]["card"]["elements"][0]["text"].as_str().unwrap();
    assert!(combining.ends_with("e\u{301}") && combining.chars().count() == 80);
    assert_eq!(expected(named(&vectors, "heading-ideographic-space-only")), Err(Reason::EmptyElement));
}
