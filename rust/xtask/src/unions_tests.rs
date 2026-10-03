//! The emitter over the committed view (ADR 30.9.26am D6).

use serde_json::{Value, json};

fn committed_view() -> Value {
    crate::read_json(&crate::workspace_root().join(crate::VIEW)).expect("the committed view reads")
}

fn file<'a>(files: &'a [(&str, String)], name: &str) -> &'a str {
    files.iter().find(|(n, _)| *n == name).map(|(_, c)| c.as_str()).unwrap_or_else(|| panic!("no {name}"))
}

/// The item typify defines under `name`, attributes included, as source
/// text: from its "///`Name`" doc line to the next item's.
fn item<'a>(code: &'a str, name: &str) -> &'a str {
    let start = code.find(&format!("///`{name}`\n")).unwrap_or_else(|| panic!("no item {name}"));
    let end = code[start + 1..].find("\n///").map_or(code.len(), |i| start + 1 + i);
    &code[start..end]
}

// 30.9.26am AC16: over the committed view, unions.rs writes CardElement,
// ListItem and ContextSlice with S1 D4's arm names, the four slice kinds and
// the app.render / app.action discriminators; no request-side generated type
// carries deny_unknown_fields; and a component named after a type this file
// writes fails codegen.
#[test]
fn the_emitter_writes_the_three_unions_and_the_operations() {
    let files = crate::generate(&committed_view()).expect("the committed view generates");
    let unions = file(&files, "unions.rs");
    for needle in [
        "#[serde(tag = \"type\")]\npub enum CardElement {",
        "#[serde(rename = \"heading\")]\n    Heading(super::models::CardElementHeading),",
        "#[serde(rename = \"list\")]\n    List(super::models::CardElementList),",
        "#[serde(rename = \"link\")]\n    Link(super::models::CardElementLink),",
        "#[serde(tag = \"type\")]\npub enum ListItem {",
        "Text(super::models::ListItemText),",
        "Term(super::models::ListItemTerm),",
        "#[serde(tag = \"kind\")]\npub enum ContextSlice {",
        "Languages(super::models::ContextSliceLanguages),",
        "pub const ALL: [ContextSliceKind; 4]",
        "ContextSliceKind::Languages => \"languages\",",
        "ContextSliceKind::PlanSummary => \"plan_summary\",",
        "ContextSliceKind::ReviewDue => \"review_due\",",
        "ContextSliceKind::TutorTopic => \"tutor_topic\",",
        "pub enum Operation {\n    AppRender,\n    AppAction,\n}",
        "Operation::AppRender => \"app.render\",",
        "Operation::AppAction => \"app.action\",",
        "Operation::AppRender => \"app.card\",",
    ] {
        assert!(unions.contains(needle), "unions.rs is missing {needle:?}");
    }

    let models = file(&files, "models.rs");
    for union in ["CardElement", "ListItem", "ContextSlice"] {
        assert!(!models.contains(&format!("pub struct {union} ")), "typify defined the union {union}");
        assert!(models.contains(&format!("super::unions::{union}")), "models.rs does not reach the union {union}");
    }
    for request_side in ["AppRenderRequest", "AppActionRequest", "ContextSliceLanguages", "ContextSlicePlanSummary", "ContextSliceReviewDue", "ContextSliceTutorTopic"] {
        assert!(!item(models, request_side).contains("deny_unknown_fields"), "{request_side} still denies unknown fields");
    }
    assert!(item(models, "CardElementHeading").contains("deny_unknown_fields"), "the reply side is left as typify wrote it");
    assert!(!item(models, "CardElementHeading").contains("type_"), "an arm keeps its tag property");

    for reserved in ["Operation", "ContextSliceKind", "CardElementType"] {
        let mut view = committed_view();
        view["components"]["schemas"][reserved] = json!({ "type": "string", "enum": ["x"] });
        let refused = crate::generate(&view).expect_err("a component named after a written type is refused");
        assert!(refused.contains(reserved), "{refused}");
    }
}

/// Two runs give the same bytes, which is what check-codegen relies on.
#[test]
fn codegen_is_deterministic() {
    let view = committed_view();
    assert_eq!(crate::generate(&view).unwrap(), crate::generate(&view).unwrap());
}

/// An operation or a union arm naming a missing component is refused.
#[test]
fn a_dangling_reference_is_refused() {
    let mut view = committed_view();
    view["x-lingara-app-operations"][0]["request"] = json!("#/components/schemas/Nope");
    assert!(crate::view::read(&view).is_err_and(|e| e.contains("Nope")));
}
