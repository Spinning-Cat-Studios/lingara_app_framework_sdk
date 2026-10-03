//! The axum adapter over the core (ADR 30.9.26am D4, D9). Requests are
//! signed here as the relay signs them: Standard Webhooks, HMAC-SHA256 over
//! `id.timestamp.body`, keyed on the base64 after `lgr_whsec_`. Nothing
//! leaves the process: each request goes to the router through `oneshot`.

use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use ::axum::body::{Body, to_bytes};
use ::axum::http::{Request, StatusCode};
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use hmac::{Hmac, Mac};
use serde_json::{Value, json};
use sha2::Sha256;
use tower::ServiceExt as _;

use crate::{App, AppActionRequest, AppRenderRequest, AppSlotName, BoxError, ContextSlice, card};

const KEY: &[u8; 32] = b"axum-adapter-test-secret-0001!!!";

fn secret() -> String {
    format!("lgr_whsec_{}", STANDARD.encode(KEY))
}

fn signed(body: &str, tamper: bool) -> Request<Body> {
    let (id, now) = ("lgr_msg_00000000000000000000000000000001", SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs());
    let mut mac = Hmac::<Sha256>::new_from_slice(KEY).unwrap();
    mac.update(format!("{id}.{now}.{body}").as_bytes());
    let signature = format!("v1,{}", STANDARD.encode(mac.finalize().into_bytes()));
    let sent = if tamper { body.replacen("lgr_sub_", "lgr_suX", 1) } else { body.to_owned() };
    Request::post("/")
        .header("webhook-id", id)
        .header("webhook-timestamp", now.to_string())
        .header("webhook-signature", signature)
        .header("content-type", "application/json")
        .body(Body::from(sent))
        .unwrap()
}

fn request(kind: &str, extra: Value) -> String {
    let mut body = json!({
        "type": kind, "id": "lgr_msg_00000000000000000000000000000001",
        "install_id": "7c1d2e3f-4a5b-4c6d-8e7f-9a0b1c2d3e4f", "subject": "lgr_sub_learner", "slot": "home.side", "locale": "ja",
        "context": [
            { "kind": "languages", "source_lang": "en", "target_lang": "ja", "level": 3 },
            { "kind": "weather", "sky": "clear" },
            { "kind": "review_due", "due": 12, "learned": 340 },
        ],
    });
    body.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
    body.to_string()
}

/// An app that records what each function received.
fn recording_app(renders: Arc<Mutex<Vec<AppRenderRequest>>>, actions: Arc<Mutex<Vec<AppActionRequest>>>) -> App {
    let app = App::new([secret()], move |r: AppRenderRequest| {
        renders.lock().unwrap().push(r);
        async { Ok::<_, BoxError>(card().heading("seen", 1).build()?) }
    });
    app.unwrap().action("inc", move |a: AppActionRequest| {
        actions.lock().unwrap().push(a);
        async { Ok::<_, BoxError>(card().text("pressed").build()?) }
    })
}

async fn send(app: &App, request: Request<Body>) -> (StatusCode, Option<String>, Vec<u8>) {
    let response = crate::axum::router(app.clone()).oneshot(request).await.unwrap();
    let content_type = response.headers().get("content-type").map(|v| v.to_str().unwrap().to_owned());
    let status = response.status();
    (status, content_type, to_bytes(response.into_body(), usize::MAX).await.unwrap().to_vec())
}

// 30.9.26am AC9: through the axum router, a tampered body is 401 and never
// reaches the render function; a valid render reaches it with the decoded
// subject, slot and slices, an unknown slice kind skipped; an action's
// card_etag reaches its function byte for byte.
#[tokio::test]
async fn the_axum_router_verifies_the_raw_body_before_dispatch() {
    let (renders, actions) = (Arc::default(), Arc::default());
    let app = recording_app(Arc::clone(&renders), Arc::clone(&actions));

    let (status, content_type, body) = send(&app, signed(&request("app.render", json!({})), true)).await;
    assert_eq!((status, content_type, body.len()), (StatusCode::UNAUTHORIZED, None, 0));
    assert!(renders.lock().unwrap().is_empty(), "a tampered body never reaches the render function");

    let (status, content_type, body) = send(&app, signed(&request("app.render", json!({ "future": true })), false)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(content_type.as_deref(), Some("application/json"));
    assert_eq!(serde_json::from_slice::<Value>(&body).unwrap(), json!({ "card": { "elements": [{ "type": "heading", "text": "seen", "level": 1 }] } }));
    let rendered = renders.lock().unwrap().pop().expect("the render function ran");
    assert_eq!((rendered.subject.as_str(), rendered.slot), ("lgr_sub_learner", AppSlotName::HomeSide));
    assert!(matches!(rendered.context.as_slice(), [ContextSlice::Languages(l), ContextSlice::ReviewDue(r)] if l.level == Some(3) && r.due == 12));

    let etag = "c1_AbC-xyz_09+/=\\u00e9 é";
    let action = request("app.action", json!({ "action_id": "inc", "card_etag": etag }));
    let (status, _, _) = send(&app, signed(&action, false)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(actions.lock().unwrap().pop().expect("the action function ran").card_etag, etag);
}

/// AK2 and AK4 through the adapter: the method gate, the size cap, the
/// fixed error bodies.
#[tokio::test]
async fn the_axum_router_answers_the_fixed_failures() {
    let app = recording_app(Arc::default(), Arc::default())
        .action("boom", |_: AppActionRequest| async { Err::<crate::Reply, BoxError>("boom".into()) })
        .action("panic", |_: AppActionRequest| async { panic!("the function panicked") as Result<crate::Reply, BoxError> });

    let get = Request::get("/").body(Body::empty()).unwrap();
    assert_eq!(send(&app, get).await.0, StatusCode::METHOD_NOT_ALLOWED);
    let big = signed(&" ".repeat(crate::limits::REQUEST_MAX_BYTES + 1), false);
    assert_eq!(send(&app, big).await.0, StatusCode::PAYLOAD_TOO_LARGE);

    let bad_request = (StatusCode::BAD_REQUEST, Some("application/json".to_owned()), br#"{"error":"bad_request"}"#.to_vec());
    assert_eq!(send(&app, signed("not json", false)).await, bad_request);
    assert_eq!(send(&app, signed(&request("app.unknown", json!({})), false)).await, bad_request);
    let unregistered = request("app.action", json!({ "action_id": "nope", "card_etag": "c1_x" }));
    assert_eq!(send(&app, signed(&unregistered, false)).await, bad_request);

    let failed = (StatusCode::INTERNAL_SERVER_ERROR, Some("application/json".to_owned()), br#"{"error":"handler_failed"}"#.to_vec());
    for id in ["boom", "panic"] {
        let action = request("app.action", json!({ "action_id": id, "card_etag": "c1_x" }));
        assert_eq!(send(&app, signed(&action, false)).await, failed, "{id}");
    }
}

/// A malformed secret fails when the app is built, not on a request.
#[test]
fn a_malformed_secret_fails_at_construction() {
    let render = |_: AppRenderRequest| async { Ok::<_, BoxError>(card().text("x").build()?) };
    assert!(App::new(["not-a-secret"], render).is_err());
    assert!(App::new(Vec::<String>::new(), render).is_err());
}

/// Nested at the render path, as the README shows, the router answers that
/// path.
#[tokio::test]
async fn the_router_nests_at_the_render_path() {
    let app = recording_app(Arc::default(), Arc::default());
    let routes = ::axum::Router::new().nest("/lingara/render", crate::axum::router(app));
    let mut request = signed(&request("app.render", json!({})), false);
    *request.uri_mut() = "/lingara/render".parse().unwrap();
    assert_eq!(routes.oneshot(request).await.unwrap().status(), StatusCode::OK);
}
