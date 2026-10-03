use serde_json::json;

use crate::reach::copy_reachable;
use crate::test_support::{catalogue, refusal};

fn roots() -> Vec<String> {
    ["AppRenderRequest", "AppCardReply", "AppActionRequest"].map(String::from).to_vec()
}

/// ADR 30.9.26al AC3: exactly the schemas reachable from the app payloads
/// are copied, in document order; an event schema no payload reaches is
/// absent; a non-internal `$ref` is refused; every `const` becomes a
/// one-value `enum`, and a property named like a documentation key is kept.
#[test]
fn only_reachable_schemas_are_copied() {
    let copied = copy_reachable(&catalogue(), &roots()).expect("the fixture copies");
    let names: Vec<_> = copied.keys().map(String::as_str).collect();
    assert_eq!(
        names,
        ["AppActionRequest", "AppCardReply", "AppRenderRequest", "AppSlotName", "ButtonStyle", "Card", "CardElement",
         "ContextSlice", "ListItem"]
    );
    assert!(!copied.contains_key("LessonPlanReadyData"), "an event-only schema was copied");

    let heading = &copied["CardElement"]["oneOf"][0]["properties"]["type"];
    assert_eq!(heading, &json!({ "type": "string", "enum": ["heading"] }));
    assert!(!serde_json::to_string(&copied).unwrap().contains("\"const\""));

    let title = &copied["ContextSlice"]["oneOf"][1]["properties"]["title"];
    assert_eq!(title, &json!({ "type": ["string", "null"] }), "the property stays; its description goes");

    let mut external = catalogue();
    external["components"]["schemas"]["Card"]["properties"]["elements"]["items"] = json!({ "$ref": "https://example.com/x.json" });
    let why = refusal(copy_reachable(&external, &roots()));
    assert!(why.starts_with("/components/schemas/Card/properties/elements/items/$ref:"), "{why}");

    let mut defs = catalogue();
    defs["components"]["schemas"]["Card"]["$defs"] = json!({ "X": {} });
    assert!(refusal(copy_reachable(&defs, &roots())).starts_with("/components/schemas/Card/$defs:"));

    let mut dangling = catalogue();
    dangling["components"]["schemas"].as_object_mut().unwrap().remove("ListItem");
    assert!(refusal(copy_reachable(&dangling, &roots())).starts_with("/components/schemas/ListItem:"));
}
