use serde_json::{Map, Value, json};

use crate::reach::copy_reachable;
use crate::test_support::{catalogue, refusal, tagged};
use crate::unions::{lift, pascal};

fn copied(doc: &Value) -> Map<String, Value> {
    let roots = ["AppRenderRequest", "AppCardReply", "AppActionRequest"].map(String::from);
    copy_reachable(doc, &roots).expect("the fixture copies")
}

/// ADR 30.9.26al AC4: a tagged `oneOf` of inline members becomes one
/// `x-lingara-unions` entry with its tag and its arms in member order; each
/// inline arm is lifted as `<Union><Tag>`, a `$ref` member is used as it is,
/// and the union's component becomes `{type: object}`.
#[test]
fn a_tagged_one_of_becomes_a_recorded_union() {
    assert_eq!(pascal("plan_summary"), "PlanSummary");
    let mut schemas = copied(&catalogue());
    let records = lift(&mut schemas).expect("the fixture lifts");
    let names: Vec<_> = records.iter().map(|r| r["name"].as_str().unwrap()).collect();
    assert_eq!(names, ["CardElement", "ContextSlice", "ListItem"], "document order");

    let slice = &records[1];
    assert_eq!(slice["tag"], "kind");
    assert_eq!(
        slice["arms"],
        json!([
            { "value": "languages", "schema": "#/components/schemas/ContextSliceLanguages" },
            { "value": "plan_summary", "schema": "#/components/schemas/ContextSlicePlanSummary" }
        ])
    );
    assert_eq!(schemas["ContextSlice"], json!({ "type": "object" }));
    assert_eq!(schemas["ContextSlicePlanSummary"]["properties"]["kind"]["enum"], json!(["plan_summary"]));
    let keys: Vec<_> = schemas.keys().map(String::as_str).collect();
    let at = keys.iter().position(|k| *k == "ContextSlice").unwrap();
    assert_eq!(keys[at + 1..at + 3], ["ContextSliceLanguages", "ContextSlicePlanSummary"], "arms follow their union");

    let mut with_ref = catalogue();
    let schemas_in = with_ref["components"]["schemas"].as_object_mut().unwrap();
    schemas_in.insert("TermItem".into(), tagged("type", "term", json!({ "word": { "type": "string" } })));
    with_ref["components"]["schemas"]["ListItem"]["oneOf"][1] = json!({ "$ref": "#/components/schemas/TermItem" });
    let mut schemas = copied(&with_ref);
    let records = lift(&mut schemas).expect("a $ref member lifts");
    assert_eq!(records[2]["arms"][1], json!({ "value": "term", "schema": "#/components/schemas/TermItem" }));
    assert!(!schemas.contains_key("ListItemTerm"), "a $ref member is not lifted");
}

/// ADR 30.9.26al AC5: a `oneOf` whose members disagree on the tag name,
/// repeat a tag value or lack a one-value tag is refused, and so is a lifted
/// arm name that collides with an existing component.
#[test]
fn an_untagged_one_of_is_refused() {
    let union = |members: Value| {
        let mut doc = catalogue();
        doc["components"]["schemas"]["ListItem"] = json!({ "oneOf": members });
        let mut schemas = copied(&doc);
        refusal(lift(&mut schemas))
    };
    let text = tagged("type", "text", json!({}));
    let why = union(json!([text, tagged("kind", "term", json!({}))]));
    assert!(why.starts_with("/components/schemas/ListItem/oneOf:") && why.contains("no one-value tag"), "{why}");
    let why = union(json!([text, tagged("type", "text", json!({ "x": {} }))]));
    assert!(why.contains("the tag value text repeats"), "{why}");
    let why = union(json!([text, { "type": "object", "properties": { "type": { "type": "string" } } }]));
    assert!(why.contains("no one-value tag"), "{why}");

    let mut collide = catalogue();
    collide["components"]["schemas"]["ListItemText"] = json!({ "type": "string" });
    collide["components"]["schemas"]["AppSlotName"] = json!({ "$ref": "#/components/schemas/ListItemText" });
    let mut schemas = copied(&collide);
    let why = refusal(lift(&mut schemas));
    assert!(why.starts_with("/components/schemas/ListItemText:") && why.contains("collides"), "{why}");

    let mut nested = catalogue();
    nested["components"]["schemas"]["Card"]["properties"]["elements"] = json!({ "oneOf": [{}, {}] });
    let mut schemas = copied(&nested);
    assert!(refusal(lift(&mut schemas)).starts_with("/components/schemas/Card/properties/elements/oneOf:"));
}
