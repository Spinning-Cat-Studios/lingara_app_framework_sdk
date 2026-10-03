//! The request, as the relay builds it (ADR 30.9.26al D6; the relay's own
//! signer is the Backend's `signed_request`). A `json` body gets its `type`
//! and a fresh `id`, is serialised once, and those bytes are signed and
//! sent. Exactly five headers: `webhook-id` (the body's `id`),
//! `webhook-timestamp`, `webhook-signature` (one `v1,<base64>` per secret,
//! newest first, space-separated), `content-type` and `user-agent`.

use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use hmac::{Hmac, Mac};
use serde_json::Value;
use sha2::Sha256;

use crate::case::{Body, Case, Method, SecretName, Tamper, WebhookHeader};

pub const USER_AGENT: &str = "Lingara-Apps/1 (+https://getlingara.com/docs/apps/)";
pub const SECRET_PREFIX: &str = "lgr_whsec_";
pub const MESSAGE_ID_PREFIX: &str = "lgr_msg_";

/// The ASCII each conformance secret encodes: 32 visibly fake bytes.
const SECRET_BYTES: [(SecretName, &str); 3] = [
    (SecretName::Primary, "conformance-app-secret-0001!!!!!"),
    (SecretName::Secondary, "conformance-app-secret-0002!!!!!"),
    (SecretName::Unknown, "conformance-app-secret-0003!!!!!"),
];

/// `lgr_whsec_` + base64 of the name's fake bytes.
pub fn secret(name: SecretName) -> String {
    let (_, ascii) = SECRET_BYTES.iter().find(|(n, _)| *n == name).expect("every name has bytes");
    format!("{SECRET_PREFIX}{}", STANDARD.encode(ascii))
}

/// What the fixture app is started with: `primary,secondary`.
pub fn fixture_secrets() -> String {
    format!("{},{}", secret(SecretName::Primary), secret(SecretName::Secondary))
}

/// One `v1,<base64 HMAC-SHA256("id.timestamp.body")>` entry; the key is the
/// base64 after `lgr_whsec_`.
pub fn signature(secret: &str, id: &str, timestamp: i64, body: &[u8]) -> Result<String, String> {
    let encoded = secret.strip_prefix(SECRET_PREFIX).ok_or("a signing secret starts with lgr_whsec_")?;
    let key = STANDARD.decode(encoded).map_err(|e| format!("a signing secret's key is not base64: {e}"))?;
    let mut mac = Hmac::<Sha256>::new_from_slice(&key).map_err(|e| e.to_string())?;
    mac.update(format!("{id}.{timestamp}.").as_bytes());
    mac.update(body);
    Ok(format!("v1,{}", STANDARD.encode(mac.finalize().into_bytes())))
}

/// A fresh `lgr_msg_` + 32 lowercase hex.
pub fn message_id() -> String {
    let half = || {
        let mut h = RandomState::new().build_hasher();
        h.write_u64(std::process::id().into());
        h.finish()
    };
    format!("{MESSAGE_ID_PREFIX}{:016x}{:016x}", half(), half())
}

/// A request ready to send.
#[derive(Debug)]
pub struct Built {
    pub method: Method,
    /// In send order, names lowercase; the exchange applies the case.
    pub headers: Vec<(&'static str, String)>,
    pub body: Vec<u8>,
}

impl Built {
    #[cfg(test)]
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.iter().find(|(n, _)| *n == name).map(|(_, v)| v.as_str())
    }
}

/// The request for `case` at `now` (Unix seconds).
pub fn build(case: &Case, now: i64) -> Result<Built, String> {
    let request = &case.request;
    let id = message_id();
    let mut body = match &request.body {
        Body::Json(json) => json_body(json, request.operation_or_default(), &id)?,
        Body::Raw(raw) => raw.clone().into_bytes(),
        Body::RawPad(pad) => pad.bytes()?,
    };
    let timestamp = now + request.sign.timestamp_offset_s;
    let mut entries = request
        .sign
        .secrets
        .iter()
        .map(|s| signature(&secret(*s), &id, timestamp, &body))
        .collect::<Result<Vec<_>, _>>()?;
    entries.extend(request.sign.extra_entries.iter().cloned());
    if request.tamper == Some(Tamper::Body) {
        let last = body.last_mut().ok_or("tamper: body needs a body to tamper with")?;
        *last ^= 0x01;
    }
    let all = [
        (WebhookHeader::Id, "webhook-id", id),
        (WebhookHeader::Timestamp, "webhook-timestamp", timestamp.to_string()),
        (WebhookHeader::Signature, "webhook-signature", entries.join(" ")),
    ];
    let mut headers: Vec<(&'static str, String)> =
        all.into_iter().filter(|(h, _, _)| !request.omit_headers.contains(h)).map(|(_, n, v)| (n, v)).collect();
    headers.push(("content-type", "application/json".into()));
    headers.push(("user-agent", USER_AGENT.into()));
    Ok(Built { method: request.method, headers, body })
}

/// The relay's body: the case's JSON with `type` and `id` set first, in
/// that order, then the case's own fields.
fn json_body(json: &Value, operation: &str, id: &str) -> Result<Vec<u8>, String> {
    let fields = json.as_object().ok_or("a json body is an object")?;
    let mut body = serde_json::Map::new();
    body.insert("type".into(), Value::from(operation));
    body.insert("id".into(), Value::from(id));
    for (k, v) in fields.iter().filter(|(k, _)| *k != "type" && *k != "id") {
        body.insert(k.clone(), v.clone());
    }
    serde_json::to_vec(&body).map_err(|e| e.to_string())
}
