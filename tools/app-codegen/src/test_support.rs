//! Shared fixtures for the view tests: a small catalogue in the frozen
//! catalogue's shape (ADR 30.9.26al Step 0), with one event channel and
//! operation beside the app half so the tests can prove they are ignored.

use serde_json::{Value, json};

/// The app half of A2's catalogue in miniature, plus one event channel.
pub fn catalogue() -> Value {
    json!({
        "asyncapi": "3.0.0",
        "info": { "title": "Lingara", "version": "2026-09-test" },
        "defaultContentType": "application/json",
        "servers": { "receiver": { "host": "example.com", "protocol": "https" } },
        "channels": {
            "lessonPlans": {
                "address": "lesson_plan.ready",
                "x-lingara-transport": "webhook",
                "bindings": { "http": {} },
                "messages": { "ready": { "$ref": "#/components/messages/LessonPlanReady" } }
            },
            "app": {
                "address": null,
                "x-lingara-transport": "app",
                "x-i18n": "asyncapi.channels.app",
                "description": "Requests Lingara sends to your app.",
                "servers": [{ "$ref": "#/servers/receiver" }],
                "messages": {
                    "appRender": { "$ref": "#/components/messages/AppRender" },
                    "appAction": { "$ref": "#/components/messages/AppAction" },
                    "appCard": { "$ref": "#/components/messages/AppCard" }
                }
            }
        },
        "operations": {
            "receiveLessonPlan": {
                "action": "receive",
                "channel": { "$ref": "#/channels/lessonPlans" },
                "traits": [{}],
                "reply": { "channel": { "$ref": "#/channels/lessonPlans" } }
            },
            "sendAppRender": app_operation("appRender"),
            "sendAppAction": app_operation("appAction")
        },
        "components": { "messages": messages(), "schemas": schemas() }
    })
}

fn app_operation(message: &str) -> Value {
    json!({
        "action": "send",
        "channel": { "$ref": "#/channels/app" },
        "security": [],
        "messages": [{ "$ref": format!("#/channels/app/messages/{message}") }],
        "reply": {
            "channel": { "$ref": "#/channels/app" },
            "messages": [{ "$ref": "#/channels/app/messages/appCard" }]
        }
    })
}

fn messages() -> Value {
    let message = |name: &str, schema: &str| {
        json!({
            "name": name,
            "x-i18n": format!("asyncapi.messages.{name}"),
            "description": "A message.",
            "contentType": "application/json",
            "payload": { "$ref": format!("#/components/schemas/{schema}") },
            "examples": [{ "payload": {} }]
        })
    };
    json!({
        "LessonPlanReady": message("lesson_plan.ready", "LessonPlanReadyData"),
        "AppRender": message("app.render", "AppRenderRequest"),
        "AppAction": message("app.action", "AppActionRequest"),
        "AppCard": message("app.card", "AppCardReply")
    })
}

fn schemas() -> Value {
    let request = |extra: Value| {
        let mut props = json!({
            "type": { "type": "string" },
            "id": { "type": "string" },
            "slot": { "$ref": "#/components/schemas/AppSlotName" },
            "context": { "type": "array", "items": { "$ref": "#/components/schemas/ContextSlice" } }
        });
        props.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
        json!({ "type": "object", "properties": props, "required": ["type", "id", "slot", "context"] })
    };
    json!({
        "AppActionRequest": request(json!({ "action_id": { "type": "string" } })),
        "AppCardReply": {
            "type": "object", "additionalProperties": false,
            "properties": { "card": { "$ref": "#/components/schemas/Card" }, "tutor_note": { "type": ["string", "null"] } },
            "required": ["card"]
        },
        "AppRenderRequest": request(json!({})),
        "AppSlotName": { "type": "string", "enum": ["plans.empty_detail", "home.side"] },
        "ButtonStyle": { "type": "string", "enum": ["primary", "secondary"] },
        "Card": {
            "type": "object", "additionalProperties": false,
            "properties": { "elements": { "type": "array", "items": { "$ref": "#/components/schemas/CardElement" } } },
            "required": ["elements"]
        },
        "CardElement": { "oneOf": [
            tagged("type", "heading", json!({ "text": { "type": "string" }, "level": { "type": "integer", "format": "uint8" } })),
            tagged("type", "list", json!({ "items": { "type": "array", "items": { "$ref": "#/components/schemas/ListItem" } } })),
            tagged("type", "button", json!({ "label": { "type": "string" },
                "style": { "anyOf": [{ "$ref": "#/components/schemas/ButtonStyle" }, { "type": "null" }] } }))
        ] },
        "ContextSlice": { "oneOf": [
            tagged("kind", "languages", json!({ "target_lang": { "type": "string" } })),
            tagged("kind", "plan_summary", json!({ "title": { "type": ["string", "null"], "description": "kept: a property" } }))
        ] },
        "LessonPlanReadyData": { "type": "object", "properties": { "status": { "const": "ready" } } },
        "ListItem": { "oneOf": [
            tagged("type", "text", json!({ "text": { "type": "string" }, "lang": { "type": ["string", "null"] } })),
            tagged("type", "term", json!({ "word": { "type": "string" } }))
        ] }
    })
}

/// An inline union member: `props` plus the tag, last, as a `const`.
pub fn tagged(tag: &str, value: &str, props: Value) -> Value {
    let mut props = props;
    props[tag] = json!({ "type": "string", "const": value });
    json!({ "type": "object", "additionalProperties": false, "properties": props, "required": [tag] })
}

/// The refusal message `result` carries, or a panic naming what came back.
pub fn refusal<T: std::fmt::Debug>(result: Result<T, crate::Refusal>) -> String {
    match result {
        Err(r) => r.0,
        Ok(v) => panic!("expected a refusal, got {v:?}"),
    }
}

/// Every object key anywhere in `v`, for asserting a key is gone.
pub fn all_keys(v: &Value, out: &mut Vec<String>) {
    match v {
        Value::Object(map) => {
            for (k, child) in map {
                out.push(k.clone());
                all_keys(child, out);
            }
        }
        Value::Array(list) => list.iter().for_each(|c| all_keys(c, out)),
        _ => {}
    }
}
