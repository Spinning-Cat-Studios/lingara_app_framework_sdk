//! What the core answers (ADR 30.9.26am D4; AK2–AK4): a status, a body, and
//! `application/json` on every body. The error bodies are fixed.

const JSON: &str = "application/json";

/// A framework-neutral reply for an adapter to write as is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Response {
    pub status: u16,
    /// `Some("application/json")` exactly when there is a body.
    pub content_type: Option<&'static str>,
    pub body: Vec<u8>,
}

impl Response {
    fn empty(status: u16) -> Self {
        Response { status, content_type: None, body: Vec::new() }
    }

    fn json(status: u16, body: &[u8]) -> Self {
        Response { status, content_type: Some(JSON), body: body.to_vec() }
    }

    /// `200` with the validated reply bytes.
    pub(crate) fn ok(body: Vec<u8>) -> Self {
        Response { status: 200, content_type: Some(JSON), body }
    }

    /// `400 {"error":"bad_request"}`: not JSON, an unknown `type`, an
    /// undecodable request or an unregistered `action_id`.
    pub fn bad_request() -> Self {
        Response::json(400, br#"{"error":"bad_request"}"#)
    }

    /// `401`, empty: the signature did not verify.
    pub fn unauthorized() -> Self {
        Response::empty(401)
    }

    /// `405`, empty: not a `POST`.
    pub fn method_not_allowed() -> Self {
        Response::empty(405)
    }

    /// `413`, empty: a body over 65 536 bytes.
    pub fn too_large() -> Self {
        Response::empty(413)
    }

    /// `500 {"error":"handler_failed"}`: a function failed or its reply broke
    /// a rule.
    pub fn handler_failed() -> Self {
        Response::json(500, br#"{"error":"handler_failed"}"#)
    }
}
