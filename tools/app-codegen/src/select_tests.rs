use serde_json::json;

use crate::build_view;
use crate::select::select;
use crate::test_support::{all_keys, catalogue, refusal};

/// ADR 30.9.26al AC1: only operations on the `app`-transport channel enter
/// the view; event channels and operations, a `reply` included, are ignored,
/// never refused. An app operation with no reply, a reply on another channel
/// and an app channel with an address are each refused by pointer.
#[test]
fn only_app_operations_enter_the_view() {
    let selection = select(&catalogue()).expect("the fixture selects");
    let names: Vec<_> = selection.operations.iter().map(|o| (o.operation.as_str(), o.message.as_str())).collect();
    assert_eq!(names, [("sendAppRender", "app.render"), ("sendAppAction", "app.action")]);
    assert!(selection.operations.iter().all(|o| o.reply_message == "app.card" && o.reply == "AppCardReply"));
    assert_eq!(selection.payload_roots(), ["AppRenderRequest", "AppCardReply", "AppActionRequest"]);

    let mut no_reply = catalogue();
    no_reply["operations"]["sendAppRender"].as_object_mut().unwrap().remove("reply");
    assert!(refusal(select(&no_reply)).starts_with("/operations/sendAppRender/reply:"));

    let mut elsewhere = catalogue();
    elsewhere["operations"]["sendAppAction"]["reply"]["channel"] = json!({ "$ref": "#/channels/lessonPlans" });
    assert!(refusal(select(&elsewhere)).starts_with("/operations/sendAppAction/reply/channel:"));

    let mut addressed = catalogue();
    addressed["channels"]["app"]["address"] = json!("apps");
    assert!(refusal(select(&addressed)).starts_with("/channels/app/address:"));

    let mut none = catalogue();
    none["channels"]["app"]["x-lingara-transport"] = json!("webhook");
    assert!(refusal(select(&none)).starts_with("/channels:"));
}

/// ADR 30.9.26al AC2: keys outside the subset, an inline payload, and
/// `x-lingara-status` on the app channel, an app operation or an app message
/// are each refused; documentation keys, `info`, `servers`, `security` and
/// `examples` are dropped.
#[test]
fn keys_outside_the_subset_are_refused() {
    let cases = [
        ("/channels/app/bindings", "/channels/app/bindings"),
        ("/operations/sendAppRender/traits", "/operations/sendAppRender/traits"),
        ("/channels/app/x-lingara-status", "/channels/app/x-lingara-status"),
        ("/operations/sendAppAction/x-lingara-status", "/operations/sendAppAction/x-lingara-status"),
        ("/components/messages/AppCard/x-lingara-status", "/components/messages/AppCard/x-lingara-status"),
        ("/components/messages/AppRender/headers", "/components/messages/AppRender/headers"),
    ];
    for (at, pointer) in cases {
        let mut doc = catalogue();
        let (parent, key) = at.rsplit_once('/').unwrap();
        doc.pointer_mut(parent).unwrap()[key] = json!({});
        let why = refusal(build_view(&doc, "v", "s").map(|_| ()));
        assert!(why.starts_with(&format!("{pointer}:")), "{at}: {why}");
    }

    let mut inline = catalogue();
    inline["components"]["messages"]["AppCard"]["payload"] = json!({ "type": "object" });
    assert!(refusal(select(&inline)).starts_with("/components/messages/AppCard/payload:"));

    let view = build_view(&catalogue(), "v", "s").expect("the fixture builds").v31;
    let mut keys = Vec::new();
    all_keys(&view["components"], &mut keys);
    for gone in ["x-i18n", "examples", "servers", "security", "bindings"] {
        assert!(!keys.iter().any(|k| k == gone), "{gone} survived");
    }
    assert_eq!(view["info"]["title"], "Lingara apps", "the catalogue's info is not carried");
}
