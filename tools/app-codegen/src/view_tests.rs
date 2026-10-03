use std::fs;
use std::path::PathBuf;

use serde_json::Value;

use crate::build_view;
use crate::cli::current_snapshot;

fn spec() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../spec")
}

fn view() -> String {
    let (id, snapshot) = current_snapshot(&spec().join("versions.toml")).expect("the vendored registry has a current");
    let catalogue: Value = serde_json::from_str(&fs::read_to_string(snapshot).unwrap()).unwrap();
    let source = fs::read_to_string(spec().join("SOURCE")).unwrap();
    spec_codegen::render(&build_view(&catalogue, &id, source.trim()).expect("the vendored catalogue builds").v31)
}

/// ADR 30.9.26al AC8: over the vendored catalogue the view has exactly two
/// app operations, both replying `app.card` → `AppCardReply`, and exactly
/// three unions: `CardElement` on `type` with eight arms including `list`,
/// `ListItem` on `type` with two, and `ContextSlice` on `kind` with four.
/// Two runs are byte-identical, and equal to the committed view.
#[test]
fn the_vendored_catalogue_yields_two_app_operations() {
    let text = view();
    assert_eq!(text, view(), "two runs differ");
    assert_eq!(text, fs::read_to_string(spec().join("generator/apps.3.1.json")).unwrap(), "the committed view is stale");
    let v: Value = serde_json::from_str(&text).unwrap();

    let ops: Vec<_> = v["x-lingara-app-operations"]
        .as_array()
        .unwrap()
        .iter()
        .map(|o| (o["operation"].as_str().unwrap(), o["message"].as_str().unwrap()))
        .collect();
    assert_eq!(ops, [("sendAppRender", "app.render"), ("sendAppAction", "app.action")]);
    for op in v["x-lingara-app-operations"].as_array().unwrap() {
        assert_eq!(op["reply_message"], "app.card");
        assert_eq!(op["reply"], "#/components/schemas/AppCardReply");
    }

    let unions: Vec<_> = v["x-lingara-unions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|u| (u["name"].as_str().unwrap(), u["tag"].as_str().unwrap(), u["arms"].as_array().unwrap().len()))
        .collect();
    assert_eq!(unions.len(), 3);
    for expected in [("CardElement", "type", 8), ("ListItem", "type", 2), ("ContextSlice", "kind", 4)] {
        assert!(unions.contains(&expected), "missing {expected:?} in {unions:?}");
    }
    let card = v["x-lingara-unions"].as_array().unwrap().iter().find(|u| u["name"] == "CardElement").unwrap();
    assert!(card["arms"].as_array().unwrap().iter().any(|a| a["value"] == "list"));
}
