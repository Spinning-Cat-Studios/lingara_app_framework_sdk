//! The handler core (ADR 30.9.26am D4): one framework-neutral function that
//! verifies, decodes, dispatches, validates and encodes. An adapter reads the
//! method, the headers and at most `REQUEST_MAX_BYTES` + 1 of the body, and
//! is a few lines over it.

use std::collections::HashMap;
use std::future::Future;
use std::panic::AssertUnwindSafe;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use lingara::BuildError;
use lingara::events::{Webhook, WebhookHeaders};

use crate::decode::{Request, decode};
use crate::generated::models::{AppActionRequest, AppRenderRequest};
use crate::generated::unions::Operation;
use crate::limits::{REQUEST_MAX_BYTES, validate_reply};
use crate::reply::Reply;
use crate::response::Response;

/// What a render or action function may fail with.
pub type BoxError = Box<dyn std::error::Error + Send + Sync>;

type ReplyFuture = Pin<Box<dyn Future<Output = Result<Reply, BoxError>> + Send>>;
type Function<R> = Arc<dyn Fn(R) -> ReplyFuture + Send + Sync>;

/// An app: its secrets, one render function and its action functions by
/// `action_id`. Cheap to clone.
#[derive(Clone)]
pub struct App {
    webhook: Webhook,
    render: Function<AppRenderRequest>,
    actions: HashMap<String, Function<AppActionRequest>>,
}

/// Erases a developer's async function. It is called inside the future, so
/// even a panic before its first `.await` is caught as a failure.
fn function<R, F, Fut, T, E>(f: F) -> Function<R>
where
    R: Send + 'static,
    F: Fn(R) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<T, E>> + Send + 'static,
    T: Into<Reply>,
    E: Into<BoxError>,
{
    let f = Arc::new(f);
    Arc::new(move |request| {
        let f = Arc::clone(&f);
        Box::pin(async move { f(request).await.map(Into::into).map_err(Into::into) })
    })
}

impl App {
    /// The app's `lgr_whsec_…` secret, or two during a rotation, and its
    /// render function. A malformed secret fails here, at startup, not on the
    /// first request.
    pub fn new<S, F, Fut, T, E>(secrets: impl IntoIterator<Item = S>, render: F) -> Result<Self, BuildError>
    where
        S: AsRef<str>,
        F: Fn(AppRenderRequest) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<T, E>> + Send + 'static,
        T: Into<Reply>,
        E: Into<BoxError>,
    {
        let webhook = Webhook::with_secrets(secrets)?;
        Ok(App { webhook, render: function(render), actions: HashMap::new() })
    }

    /// The function a button's `action` reaches, as the request's
    /// `action_id`. An action may arrive twice: make it safe to repeat.
    #[must_use]
    pub fn action<F, Fut, T, E>(mut self, action_id: impl Into<String>, f: F) -> Self
    where
        F: Fn(AppActionRequest) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<T, E>> + Send + 'static,
        T: Into<Reply>,
        E: Into<BoxError>,
    {
        self.actions.insert(action_id.into(), function(f));
        self
    }

    /// `405` for anything but `POST`: call it before reading the body.
    pub fn check_method(method: &str) -> Result<(), Response> {
        if method.eq_ignore_ascii_case("POST") { Ok(()) } else { Err(Response::method_not_allowed()) }
    }

    /// Everything after the method: the raw body exactly as received and the
    /// request's headers.
    pub async fn handle(&self, headers: &impl WebhookHeaders, body: &[u8]) -> Response {
        if body.len() > REQUEST_MAX_BYTES {
            return Response::too_large();
        }
        if self.webhook.verify_signature(body, headers).is_err() {
            return Response::unauthorized();
        }
        match decode(body) {
            Some(request) => self.dispatch(request).await,
            None => Response::bad_request(),
        }
    }

    async fn dispatch(&self, request: Request) -> Response {
        let (operation, install_id, future) = match request {
            Request::Render(r) => (Operation::AppRender, r.install_id.clone(), (self.render)(r)),
            Request::Action(a) => match self.actions.get(&a.action_id) {
                Some(f) => (Operation::AppAction, a.install_id.clone(), f(a)),
                None => return Response::bad_request(),
            },
        };
        let failure = match CatchPanic(future).await {
            Some(Ok(reply)) => match validate_reply(&reply.to_value()) {
                Ok(bytes) => return Response::ok(bytes),
                Err(reason) => format!("the reply breaks the {reason} rule"),
            },
            Some(Err(error)) => format!("the function failed: {error}"),
            None => "the function panicked".to_owned(),
        };
        // Never the body, a secret or a signature.
        log::error!(target: "lingara_apps", "{} for install {install_id}: {failure}", operation.message());
        Response::handler_failed()
    }
}

/// A developer function's future, with a panic turned into `None`.
struct CatchPanic(ReplyFuture);

impl Future for CatchPanic {
    type Output = Option<Result<Reply, BoxError>>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let inner = &mut self.0;
        match std::panic::catch_unwind(AssertUnwindSafe(|| inner.as_mut().poll(cx))) {
            Ok(Poll::Pending) => Poll::Pending,
            Ok(Poll::Ready(output)) => Poll::Ready(Some(output)),
            Err(_) => Poll::Ready(None),
        }
    }
}
