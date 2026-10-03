//! The manifest builder (ADR 30.9.26am D5), hand-written against Backend
//! A1 §2's wire form, which the console accepts as an upload. Its closed
//! values come from the view: `slots` are the generated `AppSlotName`, and
//! `context` the generated `ContextSliceKind`.
//!
//! ```
//! use lingara_apps::{manifest, AppSlotName, ContextSliceKind};
//!
//! let manifest = manifest()
//!     .default_locale("en")
//!     .name("Daily five")
//!     .description([("en", "Five words to review."), ("zh-Hans", "复习五个词。")])
//!     .render_url("https://apps.example.com/lingara/render")
//!     .slots([AppSlotName::HomeSide])
//!     .context([ContextSliceKind::Languages, ContextSliceKind::PlanSummary])
//!     .build()?;
//! std::fs::write(std::env::temp_dir().join("manifest.json"), manifest.to_json())?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

use std::collections::BTreeMap;

use serde::Serialize;

use crate::generated::models::AppSlotName;
use crate::generated::unions::ContextSliceKind;
use crate::manifest_rules::{ManifestError, validate_manifest};

/// A name or description: one string, which is the `default_locale`'s, or a
/// map from locale to text.
#[derive(Clone, Debug)]
pub enum Localized {
    Default(String),
    Locales(BTreeMap<String, String>),
}

impl From<&str> for Localized {
    fn from(text: &str) -> Self {
        Localized::Default(text.to_owned())
    }
}

impl From<String> for Localized {
    fn from(text: String) -> Self {
        Localized::Default(text)
    }
}

impl<K: Into<String>, V: Into<String>, const N: usize> From<[(K, V); N]> for Localized {
    fn from(pairs: [(K, V); N]) -> Self {
        Localized::Locales(pairs.into_iter().map(|(k, v)| (k.into(), v.into())).collect())
    }
}

/// A validated manifest. `to_json` writes the upload.
#[derive(Clone, Debug, Serialize)]
pub struct Manifest {
    manifest_version: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    default_locale: Option<String>,
    name: BTreeMap<String, String>,
    description: BTreeMap<String, String>,
    render_url: String,
    slots: Vec<AppSlotName>,
    context: Vec<ContextSliceKind>,
    scopes: Vec<String>,
    tutor_note: bool,
    listed: bool,
}

impl Manifest {
    /// The manifest as the console takes it, pretty-printed.
    pub fn to_json(&self) -> String {
        // Strings, maps, lists and booleans: nothing here fails to serialise.
        serde_json::to_string_pretty(self).unwrap_or_default()
    }
}

/// Starts a manifest; `listed` and `tutor_note` default to `false`.
pub fn manifest() -> ManifestBuilder {
    ManifestBuilder::default()
}

#[derive(Clone, Debug, Default)]
#[must_use]
pub struct ManifestBuilder {
    default_locale: Option<String>,
    name: Option<Localized>,
    description: Option<Localized>,
    render_url: String,
    slots: Vec<AppSlotName>,
    context: Vec<ContextSliceKind>,
    scopes: Vec<String>,
    tutor_note: bool,
    listed: bool,
}

impl ManifestBuilder {
    pub fn default_locale(mut self, locale: impl Into<String>) -> Self {
        self.default_locale = Some(locale.into());
        self
    }

    /// 1–40 characters per locale.
    pub fn name(mut self, name: impl Into<Localized>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// 1–280 characters per locale.
    pub fn description(mut self, description: impl Into<Localized>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn render_url(mut self, url: impl Into<String>) -> Self {
        self.render_url = url.into();
        self
    }

    pub fn slots(mut self, slots: impl IntoIterator<Item = AppSlotName>) -> Self {
        self.slots = slots.into_iter().collect();
        self
    }

    /// The slices the app asks a learner to share.
    pub fn context(mut self, kinds: impl IntoIterator<Item = ContextSliceKind>) -> Self {
        self.context = kinds.into_iter().collect();
        self
    }

    pub fn scopes<S: Into<String>>(mut self, scopes: impl IntoIterator<Item = S>) -> Self {
        self.scopes = scopes.into_iter().map(Into::into).collect();
        self
    }

    pub fn tutor_note(mut self, tutor_note: bool) -> Self {
        self.tutor_note = tutor_note;
        self
    }

    pub fn listed(mut self, listed: bool) -> Self {
        self.listed = listed;
        self
    }

    /// The manifest, or the first rule it breaks.
    pub fn build(self) -> Result<Manifest, ManifestError> {
        let locale = self.default_locale.clone().unwrap_or_default();
        let resolve = |text: Option<Localized>| match text {
            Some(Localized::Default(text)) => BTreeMap::from([(locale.clone(), text)]),
            Some(Localized::Locales(map)) => map,
            None => BTreeMap::new(),
        };
        let manifest = Manifest {
            manifest_version: 1,
            name: resolve(self.name),
            description: resolve(self.description),
            default_locale: self.default_locale,
            render_url: self.render_url,
            slots: self.slots,
            context: self.context,
            scopes: self.scopes,
            tutor_note: self.tutor_note,
            listed: self.listed,
        };
        validate_manifest(&serde_json::to_value(&manifest).unwrap_or_default())?;
        Ok(manifest)
    }
}
