//! The dependency rule (ADR 30.9.26am D1), read from the manifest Cargo
//! publishes. An integration test, not a unit test: it reads `LIBRARY_FLOOR`
//! beside the crate, which the packaged crate does not carry.

use std::collections::BTreeSet;

fn manifest() -> toml::Table {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    std::fs::read_to_string(&path).unwrap().parse().expect("Cargo.toml parses")
}

fn is_optional(spec: &toml::Value) -> bool {
    spec.get("optional").and_then(toml::Value::as_bool).unwrap_or(false)
}

// 30.9.26am AC23: the non-optional dependencies are lingara, serde and
// serde_json, beside log (the library's own logging facade, which it already
// carries, so a kit failure lands where the library's lines do); axum is
// optional and not in `default`; and lingara is at LIBRARY_FLOOR.
#[test]
fn only_the_library_and_its_serde_are_required() {
    let manifest = manifest();
    let dependencies = manifest["dependencies"].as_table().expect("a [dependencies] table");
    let required: BTreeSet<&str> = dependencies.iter().filter(|(_, spec)| !is_optional(spec)).map(|(name, _)| name.as_str()).collect();
    assert_eq!(required, BTreeSet::from(["lingara", "log", "serde", "serde_json"]));

    assert!(is_optional(&dependencies["axum"]), "axum is optional");
    let default = manifest["features"]["default"].as_array().expect("a default feature list");
    assert!(default.is_empty(), "nothing is on by default: {default:?}");

    let floor_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../LIBRARY_FLOOR");
    let floor = std::fs::read_to_string(floor_path).unwrap();
    assert_eq!(dependencies["lingara"].as_str(), Some(floor.trim()), "lingara is at the library floor, Cargo's default caret");
    assert!(manifest["dev-dependencies"].get("lingara").is_none(), "no second spelling of the library");
}
