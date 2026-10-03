//! The builders and validators over S1's card rules and the manifest rules
//! (ADR 30.9.26am D5, D9). The vector files themselves are walked by
//! `rust/tests/vectors_tests.rs`, outside the packaged crate.

use serde_json::{Value, json};

use crate::{
    AppSlotName, ContextSliceKind, Reason, Term, item, manifest, reply, validate_manifest, validate_reply,
};

/// On success `validate_reply` returns the bytes the core sends: compact,
/// raw UTF-8, `/` unescaped, an absent tutor note omitted.
#[test]
fn validate_reply_returns_the_bytes_it_measured() {
    let card = crate::card().text("漢/字").link("Why?", "https://apps.example.com/why").build().unwrap();
    let bytes = validate_reply(&reply(card).to_value()).unwrap();
    let text = String::from_utf8(bytes).unwrap();
    assert!(text.contains("漢/字") && text.contains("https://apps.example.com/why"), "{text}");
    assert!(!text.contains("tutor_note") && !text.contains("null") && !text.contains(' '), "{text}");
}

/// The builder refuses with the same reasons the vectors name.
#[test]
fn the_card_builder_refuses_what_the_relay_would_clamp() {
    let refused = |b: crate::CardBuilder| b.build().unwrap_err().reason();
    assert_eq!(refused(crate::card()), Reason::EmptyCard);
    assert_eq!(refused(crate::card().heading("x".repeat(81), 1)), Reason::TextLength);
    assert_eq!(refused(crate::card().heading("Three", 3)), Reason::HeadingLevel);
    assert_eq!(refused(crate::card().progress(f64::NAN, "nan")), Reason::ProgressRange);
    assert_eq!(refused(crate::card().text(crate::Text::new("x").lang("english"))), Reason::Lang);
    assert_eq!(refused(crate::card().link("Why?", "https://127.0.0.1/")), Reason::Link);
    assert_eq!(refused(crate::card().button("Next", "next kanji")), Reason::ButtonAction);
    assert_eq!(refused(crate::card().list((1..=21).map(|n| item::text(n.to_string())))), Reason::ListItems);
    assert_eq!(refused(crate::card().list([])), Reason::ListItems);
    assert_eq!(refused(crate::card().heading("\u{3000}", 1)), Reason::EmptyElement);

    let full = crate::card()
        .heading("Today", 1)
        .term(Term::new("雨").reading("yǔ").gloss("rain").lang("zh"))
        .list([item::text("one\ntwo"), item::term("二")])
        .progress(0.5, "half")
        .divider()
        .styled_button("Next", "next", crate::ButtonStyle::Primary)
        .link("Why?", "https://apps.example.com/why")
        .build()
        .unwrap();
    let value = reply(full).tutor_note("first\nsecond").unwrap().to_value();
    assert_eq!(value["card"]["elements"][1], json!({ "type": "term", "word": "雨", "reading": "yǔ", "gloss": "rain", "lang": "zh" }));
    assert_eq!(value["card"]["elements"][4], json!({ "type": "divider" }));
    assert_eq!(value["tutor_note"], "first\nsecond");

    let card = crate::card().text("x").build().unwrap();
    assert_eq!(reply(card.clone()).tutor_note("a".repeat(281)).unwrap_err().reason(), Reason::TutorNoteLength);
    assert_eq!(reply(card).tutor_note("a\tb").unwrap_err().reason(), Reason::ControlChars);
}

/// The manifest builder writes A1 §2's form, with a bare string as the
/// default locale's and `listed` false unless set.
#[test]
fn the_manifest_builder_writes_the_upload() {
    let built = manifest()
        .default_locale("en")
        .name("Daily five")
        .description([("en", "Five words."), ("zh-Hans", "五个词。")])
        .render_url("https://apps.example.com/lingara/render")
        .slots([AppSlotName::HomeSide, AppSlotName::PlansEmptyDetail])
        .context([ContextSliceKind::Languages])
        .scopes(["plans:read"])
        .tutor_note(true)
        .build()
        .unwrap();
    let json: Value = serde_json::from_str(&built.to_json()).unwrap();
    assert_eq!(json["manifest_version"], 1);
    assert_eq!(json["name"], json!({ "en": "Daily five" }));
    assert_eq!(json["slots"], json!(["home.side", "plans.empty_detail"]));
    assert_eq!(json["context"], json!(["languages"]));
    assert_eq!(json["listed"], false);
    assert_eq!(validate_manifest(&json), Ok(()));

    let rule = |b: crate::ManifestBuilder| b.build().unwrap_err().rule().as_str();
    assert_eq!(rule(manifest().name("x")), "default_locale");
    let base = || manifest().default_locale("en").name("Daily five").description("Five.");
    assert_eq!(rule(base().render_url("http://apps.example.com/")), "render_url");
    assert_eq!(rule(base().render_url("https://apps.example.com/")), "slots");
    let slots = || base().render_url("https://apps.example.com/").slots([AppSlotName::HomeSide]);
    assert_eq!(rule(slots().scopes(["plans:read", "plans:read"])), "duplicate");
}
