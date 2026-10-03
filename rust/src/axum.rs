//! The axum adapter (ADR 30.9.26am D2), behind the `axum` feature: a router
//! answering `/` that hands the raw body to the core. Nest it at the path
//! your manifest's `render_url` names.
//!
//! ```no_run
//! # async fn run(app: lingara_apps::App) -> std::io::Result<()> {
//! let routes = axum::Router::new().nest("/lingara/render", lingara_apps::axum::router(app));
//! let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
//! axum::serve(listener, routes).await
//! # }
//! ```

use std::sync::Arc;

use ::axum::Router;
use ::axum::body::{Body, to_bytes};
use ::axum::extract::Request;
use ::axum::http::{HeaderValue, StatusCode, header};
use ::axum::routing::any;

use crate::core::App;
use crate::limits::REQUEST_MAX_BYTES;
use crate::response::Response;

/// `app` at `/`, every method: the core answers anything but `POST` with
/// `405` before the body is read.
pub fn router(app: App) -> Router {
    let app = Arc::new(app);
    Router::new().route("/", any(move |request: Request| serve(Arc::clone(&app), request)))
}

async fn serve(app: Arc<App>, request: Request) -> ::axum::response::Response {
    if let Err(refused) = App::check_method(request.method().as_str()) {
        return write(refused);
    }
    let (parts, body) = request.into_parts();
    let declared = parts.headers.get(header::CONTENT_LENGTH).and_then(|v| v.to_str().ok()?.parse::<u64>().ok());
    if declared.is_some_and(|length| length > REQUEST_MAX_BYTES as u64) {
        return write(Response::too_large());
    }
    // Reads at most the limit: a longer body stops the read with an error,
    // never a larger buffer.
    let Ok(body) = to_bytes(body, REQUEST_MAX_BYTES).await else {
        return write(Response::too_large());
    };
    write(app.handle(&parts.headers, &body).await)
}

fn write(response: Response) -> ::axum::response::Response {
    let mut written = ::axum::response::Response::new(Body::from(response.body));
    *written.status_mut() = StatusCode::from_u16(response.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    if let Some(content_type) = response.content_type {
        written.headers_mut().insert(header::CONTENT_TYPE, HeaderValue::from_static(content_type));
    }
    written
}
