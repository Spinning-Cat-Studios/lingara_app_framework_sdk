//! Judging a reply (ADR 30.9.26al D6). Everything that holds for the case is
//! applied, and every mismatch names the case: the status; for a `200`, a
//! JSON content type, at most 32 KiB and a card `validate_reply` accepts
//! (so a kit that sent what the relay would clamp fails here, not only in its
//! own unit tests); the expected body; the fixed error bodies of AK2 and
//! AK4; and the relay's time budget.

use std::collections::HashMap;
use std::time::Duration;

use serde_json::{Value, json};

use crate::case::{Case, ExpectBody};
use crate::exchange::Reply;
use crate::limits::{REPLY_MAX_BYTES, validate_reply};

/// The relay's budgets: 3 s for a render, 5 s for an action.
#[derive(Debug, Clone, Copy)]
pub struct Budgets {
    pub render: Duration,
    pub action: Duration,
}

impl Default for Budgets {
    fn default() -> Self {
        Budgets { render: Duration::from_secs(3), action: Duration::from_secs(5) }
    }
}

impl Budgets {
    pub fn for_operation(&self, operation: &str) -> Duration {
        if operation == "app.action" { self.action } else { self.render }
    }
}

/// The view's request and reply schemas, compiled once, keyed by message.
pub struct Schemas {
    pub request: HashMap<String, jsonschema::Validator>,
    pub reply: HashMap<String, jsonschema::Validator>,
}

impl Schemas {
    pub fn from_view(view: &Value) -> Result<Schemas, String> {
        let root = with_unions_restored(view);
        let (mut request, mut reply) = (HashMap::new(), HashMap::new());
        for op in view["x-lingara-app-operations"].as_array().ok_or("the view has no x-lingara-app-operations")? {
            let message = op["message"].as_str().ok_or("an app operation without a message")?.to_string();
            request.insert(message.clone(), compile(&root, &op["request"])?);
            reply.insert(message, compile(&root, &op["reply"])?);
        }
        Ok(Schemas { request, reply })
    }
}

/// The view with each lifted union turned back into a `oneOf` of its arms,
/// so a schema check sees the real shape rather than `{type: object}`.
pub fn with_unions_restored(view: &Value) -> Value {
    let mut root = json!({ "components": view["components"].clone() });
    for union in view["x-lingara-unions"].as_array().into_iter().flatten() {
        let arms: Vec<Value> = union["arms"].as_array().into_iter().flatten().map(|a| json!({ "$ref": a["schema"] })).collect();
        if let Some(name) = union["name"].as_str() {
            root["components"]["schemas"][name] = json!({ "oneOf": arms });
        }
    }
    root
}

fn compile(root: &Value, reference: &Value) -> Result<jsonschema::Validator, String> {
    let mut schema = root.clone();
    schema["$ref"] = reference.clone();
    jsonschema::validator_for(&schema).map_err(|e| format!("the view's schema at {reference} does not compile: {e}"))
}

/// The fixed body each error status carries.
fn fixed_error_body(status: u16) -> Option<Value> {
    match status {
        400 => Some(json!({ "error": "bad_request" })),
        500 => Some(json!({ "error": "handler_failed" })),
        _ => None,
    }
}

/// Every way `reply` misses `case`, each naming the case.
pub fn judge(case: &Case, schemas: &Schemas, reply: &Reply, budget: Duration) -> Vec<String> {
    let mut out = Vec::new();
    let mut miss = |why: String| out.push(format!("{}: {why}", case.id));
    if reply.status != case.expect.status {
        miss(format!("status {} where {} was expected", reply.status, case.expect.status));
    }
    if reply.elapsed > budget {
        miss(format!("replied in {} ms, over the relay's {} ms budget", reply.elapsed.as_millis(), budget.as_millis()));
    }
    let parsed = serde_json::from_slice::<Value>(&reply.body).ok();
    if reply.status == 200 {
        judge_card(reply, parsed.as_ref()).into_iter().for_each(&mut miss);
    }
    if let Some(fixed) = fixed_error_body(reply.status)
        && parsed.as_ref() != Some(&fixed)
    {
        miss(format!("a {} body must be exactly {fixed}", reply.status));
    }
    if let Some(expected) = &case.expect.body {
        let reply_schema = schemas.reply.get(case.request.operation_or_default());
        if let Some(why) = judge_body(expected, &reply.body, parsed.as_ref(), reply_schema) {
            miss(why);
        }
    }
    out
}

/// A `200` is JSON, within the relay's read cap, and a card the relay would
/// draw untouched.
fn judge_card(reply: &Reply, parsed: Option<&Value>) -> Vec<String> {
    let mut out = Vec::new();
    if !reply.header("content-type").is_some_and(|t| t.starts_with("application/json")) {
        out.push(format!("a 200 with content-type {:?}, not application/json", reply.header("content-type")));
    }
    if reply.body.len() > REPLY_MAX_BYTES {
        out.push(format!("a {}-byte reply, over {REPLY_MAX_BYTES}", reply.body.len()));
    }
    match parsed.map(validate_reply) {
        None => out.push("a 200 whose body is not JSON".into()),
        Some(Err(reason)) => out.push(format!("a card the relay would clamp: {}", reason.as_str())),
        Some(Ok(())) => {}
    }
    out
}

fn judge_body(expected: &ExpectBody, raw: &[u8], parsed: Option<&Value>, schema: Option<&jsonschema::Validator>) -> Option<String> {
    match expected {
        ExpectBody::Empty if raw.is_empty() => None,
        ExpectBody::Empty => Some(format!("a {}-byte body where an empty one was expected", raw.len())),
        ExpectBody::Json(want) => {
            let got = parsed.map(drop_nulls);
            (got.as_ref() != Some(&drop_nulls(want))).then(|| format!("body {} is not {want}", String::from_utf8_lossy(raw)))
        }
        ExpectBody::MatchesSchema(_) => match (parsed, schema) {
            (Some(v), Some(s)) if s.is_valid(v) => None,
            (Some(v), Some(s)) => Some(format!("the reply is outside the reply schema: {}", first_error(s, v))),
            (None, _) => Some("the reply is not JSON".into()),
            (_, None) => Some("the case names no operation with a reply schema".into()),
        },
    }
}

fn first_error(schema: &jsonschema::Validator, v: &Value) -> String {
    schema.iter_errors(v).next().map(|e| format!("{} at {}", e, e.instance_path)).unwrap_or_default()
}

/// `v` with every `null` object member removed, recursively.
pub fn drop_nulls(v: &Value) -> Value {
    match v {
        Value::Object(map) => Value::Object(map.iter().filter(|(_, x)| !x.is_null()).map(|(k, x)| (k.clone(), drop_nulls(x))).collect()),
        Value::Array(list) => Value::Array(list.iter().map(drop_nulls).collect()),
        other => other.clone(),
    }
}
