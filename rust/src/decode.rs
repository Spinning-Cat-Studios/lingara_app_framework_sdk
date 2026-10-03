//! Lenient request decoding (ADR 30.9.26am D4, D6; AK2's forward
//! compatibility). The body's `type` names the operation; `context` is read
//! element by element before the generated union parser runs, so a slice of
//! a kind this crate does not know is skipped with a log line; and the
//! generated request types ignore unknown fields (the codegen drops their
//! `deny_unknown_fields`).

use serde_json::Value;

use crate::generated::models::{AppActionRequest, AppRenderRequest};
use crate::generated::unions::{ContextSliceKind, Operation};

/// A verified request, decoded.
pub(crate) enum Request {
    Render(AppRenderRequest),
    Action(AppActionRequest),
}

/// `None` for anything that is not a request this crate knows: not JSON, an
/// unknown `type`, or a body its generated type does not decode.
pub(crate) fn decode(body: &[u8]) -> Option<Request> {
    let mut value: Value = serde_json::from_slice(body).ok()?;
    let operation = value.get("type").and_then(Value::as_str).and_then(Operation::from_message)?;
    if let Some(context) = value.get_mut("context").and_then(Value::as_array_mut) {
        context.retain(known_kind);
    }
    // Exhaustive: a third operation in the view fails the build here until
    // the core handles it (D6).
    match operation {
        Operation::AppRender => serde_json::from_value(value).map(Request::Render).ok(),
        Operation::AppAction => serde_json::from_value(value).map(Request::Action).ok(),
    }
}

/// Keeps a slice unless its `kind` is a string this crate does not know. A
/// slice with no string `kind` is kept, so the union parser refuses it.
fn known_kind(slice: &Value) -> bool {
    match slice.get("kind").and_then(Value::as_str) {
        Some(kind) if ContextSliceKind::parse(kind).is_none() => {
            log::info!(target: "lingara_apps", "skipped a context slice of unknown kind {kind:?}");
            false
        }
        _ => true,
    }
}
