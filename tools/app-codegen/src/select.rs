//! The app half of the catalogue (ADR 30.9.26al D4): the one channel whose
//! `x-lingara-transport` is `app`, the operations on it, and the messages
//! they reach. Every other channel and operation is ignored, never judged:
//! the events belong to the client libraries. Inside the app half the subset
//! is closed, and a key outside it is refused by its JSON pointer.

use serde_json::{Map, Value, json};
use spec_codegen::walk::escape;

use crate::{Refusal, refuse, schema_ref};

/// Documentation keys, dropped wherever the app half carries them.
pub const DOC_KEYS: [&str; 7] = ["title", "summary", "description", "tags", "externalDocs", "x-i18n", "examples"];

/// One app operation: its request message's `name` (the request's `type`
/// discriminator) and schema, and its reply's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppOperation {
    pub operation: String,
    pub message: String,
    pub request: String,
    pub reply_message: String,
    pub reply: String,
}

#[derive(Debug)]
pub struct Selection {
    pub operations: Vec<AppOperation>,
}

impl Selection {
    /// Every request and reply schema, first appearance first.
    pub fn payload_roots(&self) -> Vec<String> {
        let mut roots: Vec<String> = Vec::new();
        for op in &self.operations {
            for name in [&op.request, &op.reply] {
                if !roots.contains(name) {
                    roots.push(name.clone());
                }
            }
        }
        roots
    }

    /// `x-lingara-app-operations`, in document order.
    pub fn records(&self) -> Value {
        let entry = |op: &AppOperation| {
            json!({
                "operation": op.operation,
                "message": op.message,
                "request": schema_ref(&op.request),
                "reply_message": op.reply_message,
                "reply": schema_ref(&op.reply),
            })
        };
        Value::Array(self.operations.iter().map(entry).collect())
    }
}

/// The app channel's key and object, for resolving its message map.
struct AppChannel<'a> {
    key: &'a str,
    pointer: String,
    messages: &'a Map<String, Value>,
}

pub fn select(doc: &Value) -> Result<Selection, Refusal> {
    check_root(doc)?;
    let channel = app_channel(doc)?;
    let mut operations = Vec::new();
    let empty = Map::new();
    let all = doc.get("operations").and_then(Value::as_object).unwrap_or(&empty);
    for (name, op) in all {
        if op.pointer("/channel/$ref").and_then(Value::as_str) == Some(channel_ref(&channel).as_str()) {
            operations.push(app_operation(doc, &channel, name, op)?);
        }
    }
    if operations.is_empty() {
        return refuse(&channel.pointer, "the app channel has no operation");
    }
    Ok(Selection { operations })
}

fn check_root(doc: &Value) -> Result<(), Refusal> {
    let version = doc.get("asyncapi").and_then(Value::as_str).unwrap_or_default();
    if !version.starts_with("3.0.") {
        return refuse("/asyncapi", "the app view reads AsyncAPI 3.0.x only");
    }
    match doc.get("defaultContentType") {
        None => Ok(()),
        Some(v) if v == "application/json" => Ok(()),
        Some(_) => refuse("/defaultContentType", "must be application/json or absent"),
    }
}

fn app_channel(doc: &Value) -> Result<AppChannel<'_>, Refusal> {
    let Some(channels) = doc.get("channels").and_then(Value::as_object) else {
        return refuse("/channels", "no channel has x-lingara-transport: app");
    };
    let mut found = channels.iter().filter(|(_, c)| c.get("x-lingara-transport") == Some(&json!("app")));
    let Some((key, channel)) = found.next() else {
        return refuse("/channels", "no channel has x-lingara-transport: app");
    };
    if let Some((second, _)) = found.next() {
        return refuse(&format!("/channels/{}", escape(second)), "a second app channel");
    }
    let pointer = format!("/channels/{}", escape(key));
    let obj = channel.as_object().expect("filtered on a key, so an object");
    for (k, v) in obj {
        match k.as_str() {
            "address" if v.is_null() => {}
            "address" => return refuse(&format!("{pointer}/address"), "the app channel's address must be null"),
            "messages" | "x-lingara-transport" | "servers" => {}
            k if DOC_KEYS.contains(&k) => {}
            k => return refuse(&format!("{pointer}/{}", escape(k)), "outside the app channel's subset"),
        }
    }
    let messages = obj.get("messages").and_then(Value::as_object);
    let messages = messages.ok_or_else(|| Refusal(format!("{pointer}/messages: the app channel has no message map")))?;
    Ok(AppChannel { key, pointer, messages })
}

fn channel_ref(channel: &AppChannel) -> String {
    format!("#/channels/{}", escape(channel.key))
}

fn app_operation(doc: &Value, channel: &AppChannel, name: &str, op: &Value) -> Result<AppOperation, Refusal> {
    let ptr = format!("/operations/{}", escape(name));
    for k in op.as_object().into_iter().flat_map(Map::keys) {
        match k.as_str() {
            "action" | "channel" | "messages" | "reply" | "security" => {}
            k if DOC_KEYS.contains(&k) => {}
            k => return refuse(&format!("{ptr}/{}", escape(k)), "outside the app operation's subset"),
        }
    }
    if op.get("action") != Some(&json!("send")) {
        return refuse(&format!("{ptr}/action"), "an app operation is action: send");
    }
    let reply = op.get("reply").ok_or_else(|| Refusal(format!("{ptr}/reply: an app operation must reply")))?;
    let (message, request) = one_message(doc, channel, op, &ptr)?;
    let reply_ptr = format!("{ptr}/reply");
    for k in reply.as_object().into_iter().flat_map(Map::keys) {
        if k != "channel" && k != "messages" {
            return refuse(&format!("{reply_ptr}/{}", escape(k)), "outside the app reply's subset");
        }
    }
    if reply.pointer("/channel/$ref").and_then(Value::as_str) != Some(channel_ref(channel).as_str()) {
        return refuse(&format!("{reply_ptr}/channel"), "an app reply goes on the app channel");
    }
    let (reply_message, reply) = one_message(doc, channel, reply, &reply_ptr)?;
    Ok(AppOperation { operation: name.to_string(), message, request, reply_message, reply })
}

/// The single message `holder.messages` names, through the channel's map:
/// its `name` and its payload's schema name.
fn one_message(doc: &Value, channel: &AppChannel, holder: &Value, ptr: &str) -> Result<(String, String), Refusal> {
    let list = holder.get("messages").and_then(Value::as_array);
    let Some([only]) = list.map(Vec::as_slice) else {
        return refuse(&format!("{ptr}/messages"), "exactly one message $ref");
    };
    let target = only.get("$ref").and_then(Value::as_str).unwrap_or_default();
    let prefix = format!("{}/messages/", channel_ref(channel));
    let key = target.strip_prefix(&prefix).filter(|k| channel.messages.contains_key(*k));
    let Some(key) = key else {
        return refuse(&format!("{ptr}/messages/0"), "not a message of the app channel's map");
    };
    let component = channel.messages[key].get("$ref").and_then(Value::as_str).unwrap_or_default();
    let Some(name) = component.strip_prefix("#/components/messages/") else {
        return refuse(&format!("{}/messages/{}", channel.pointer, escape(key)), "not a #/components/messages $ref");
    };
    message(doc, name)
}

/// A message's `name` and payload schema, under the closed subset.
fn message(doc: &Value, name: &str) -> Result<(String, String), Refusal> {
    let ptr = format!("/components/messages/{name}");
    let msg = doc.pointer(&ptr).and_then(Value::as_object);
    let msg = msg.ok_or_else(|| Refusal(format!("{ptr}: the message does not exist")))?;
    for (k, v) in msg {
        match k.as_str() {
            "name" | "payload" => {}
            "contentType" if v == "application/json" => {}
            "contentType" => return refuse(&format!("{ptr}/contentType"), "must be application/json or absent"),
            k if DOC_KEYS.contains(&k) => {}
            k => return refuse(&format!("{ptr}/{}", escape(k)), "outside the app message's subset"),
        }
    }
    let wire_name = msg.get("name").and_then(Value::as_str);
    let wire_name = wire_name.ok_or_else(|| Refusal(format!("{ptr}/name: an app message needs a name")))?;
    let payload = msg.get("payload").and_then(Value::as_object);
    let schema = payload
        .filter(|p| p.len() == 1)
        .and_then(|p| p.get("$ref")?.as_str()?.strip_prefix("#/components/schemas/"));
    let Some(schema) = schema else {
        return refuse(&format!("{ptr}/payload"), "an app payload is a #/components/schemas $ref, never inline");
    };
    Ok((wire_name.to_string(), schema.to_string()))
}
