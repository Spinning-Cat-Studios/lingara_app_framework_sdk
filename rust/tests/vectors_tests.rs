//! S1's card-limit vectors and the manifest vectors through this kit's
//! validators (ADR 30.9.26am D5, D9). An integration test, not a unit test:
//! it reads `conformance/vectors/` beside the crate, which the packaged crate
//! does not carry, so it runs in the repository (`make test-rust`) and not in
//! release.yml's packaged-crate step.

use lingara_apps::{Reason, truncate, validate_manifest, validate_reply};
use serde_json::{Value, json};

fn vectors(file: &str) -> Vec<Value> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../conformance/vectors").join(file);
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let parsed: Value = serde_json::from_str(&text).expect("the vector file is JSON");
    parsed["vectors"].as_array().expect("a vectors list").clone()
}

fn answer<E>(result: Result<(), E>, spell: impl Fn(E) -> &'static str) -> Value {
    match result {
        Ok(()) => json!({ "ok": true }),
        Err(e) => json!({ "refused": spell(e) }),
    }
}

// 30.9.26am AC2: every card-limits.json vector's expect equals
// validate_reply's answer; every manifest.json vector's equals
// validate_manifest's; truncate of an 81-scalar astral heading at 80 is 79
// scalars plus "…".
#[test]
fn every_card_and_manifest_vector_gives_its_expected_answer() {
    let cards = vectors("card-limits.json");
    assert!(cards.len() > 50, "the card vectors are all there");
    for v in &cards {
        let got = answer(validate_reply(&v["reply"]).map(drop), Reason::as_str);
        assert_eq!(got, v["expect"], "card vector {}", v["name"]);
    }
    let manifests = vectors("manifest.json");
    assert!(manifests.len() > 30, "the manifest vectors are all there");
    for v in &manifests {
        let got = answer(validate_manifest(&v["manifest"]), |e| e.rule().as_str());
        assert_eq!(got, v["expect"], "manifest vector {}", v["name"]);
    }

    let astral = "𝄞".repeat(81);
    let cut = truncate(&astral, 80);
    assert_eq!(cut.chars().count(), 80);
    assert_eq!(cut, format!("{}…", "𝄞".repeat(79)));
    assert_eq!(truncate(&"𝄞".repeat(80), 80), "𝄞".repeat(80), "at the limit nothing is cut");
    assert_eq!(truncate("e\u{301}", 1), "…", "a combining mark is its own scalar");
}
