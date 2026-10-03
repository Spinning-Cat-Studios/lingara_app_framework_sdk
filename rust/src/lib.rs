//! The official Lingara app kit for Rust (ADR 30.9.26am).
//!
//! An app is a server that answers two signed requests from Lingara's relay,
//! `app.render` and `app.action`, with a card. This crate verifies each
//! request with the `lingara` library's webhook verifier, decodes it into the
//! generated request type, calls your function, checks the card against the
//! relay's rules and encodes the reply.
//!
//! ```no_run
//! use lingara_apps::{App, AppRenderRequest, BoxError, card};
//!
//! # fn run() -> Result<(), Box<dyn std::error::Error>> {
//! let secret = std::env::var("LINGARA_APP_SECRET")?;
//! let app = App::new([secret], |request: AppRenderRequest| async move {
//!     Ok::<_, BoxError>(card().heading(request.slot.to_string(), 1).build()?)
//! })?;
//! # Ok(())
//! # }
//! ```
//!
//! Every kit keeps one contract, `conformance/CONTRACT.md` in the
//! repository: verification (AK1), dispatch (AK2), the card rules (AK3) and
//! the fixed failure replies (AK4) below are its.

#[cfg(feature = "axum")]
pub mod axum;
mod card;
mod content;
mod core;
mod decode;
mod generated;
pub mod limits;
mod link;
mod manifest;
mod manifest_rules;
mod reply;
mod response;
mod rules;

/// The types generated from the app view: the requests, the card and its
/// elements, the three unions and the operations.
pub mod models {
    pub use crate::generated::models::*;
    pub use crate::generated::unions::*;
}

pub use crate::core::{App, BoxError};
pub use card::{CardBuilder, card};
pub use content::{Term, Text, item};
pub use generated::spec_version::GENERATED_FOR_VERSION;
pub use limits::{Reason, truncate, validate_reply};
/// The library this kit is built on, for `getLessonPlan` and webhook events.
pub use lingara;
pub use manifest::{Localized, Manifest, ManifestBuilder, manifest};
pub use manifest_rules::{ManifestError, ManifestRule, validate_manifest};
pub use models::{
    AppActionRequest, AppRenderRequest, AppSlotName, ButtonStyle, Card, CardElement, ContextSlice, ContextSliceKind, ListItem, Operation,
};
pub use reply::{CardLimitError, Reply, reply};
pub use response::Response;

#[cfg(test)]
mod tests;
