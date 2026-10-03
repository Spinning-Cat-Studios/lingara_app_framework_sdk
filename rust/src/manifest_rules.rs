//! `validate_manifest` (ADR 30.9.26am D5): what Backend A1's `plan_manifest`
//! would refuse or normalise away, where a kit can know it. The rules and
//! their order are `conformance/vectors/manifest.json`'s description; a
//! refusal names the first broken one.

use std::collections::BTreeSet;
use std::fmt;

use serde_json::{Value, json};

use crate::generated::models::AppSlotName;
use crate::generated::unions::ContextSliceKind;

pub const NAME_MAX_CHARS: usize = 40;
pub const DESCRIPTION_MAX_CHARS: usize = 280;
/// A1's seven stored keys, serialised.
pub const MANIFEST_MAX_BYTES: usize = 65_536;

/// The manifest rules, in the order a refusal is chosen.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ManifestRule {
    ManifestVersion,
    DefaultLocale,
    Name,
    Description,
    RenderUrl,
    Slots,
    Context,
    Duplicate,
    TooLarge,
}

impl ManifestRule {
    pub const fn as_str(self) -> &'static str {
        match self {
            ManifestRule::ManifestVersion => "manifest_version",
            ManifestRule::DefaultLocale => "default_locale",
            ManifestRule::Name => "name",
            ManifestRule::Description => "description",
            ManifestRule::RenderUrl => "render_url",
            ManifestRule::Slots => "slots",
            ManifestRule::Context => "context",
            ManifestRule::Duplicate => "duplicate",
            ManifestRule::TooLarge => "too_large",
        }
    }
}

/// A manifest broke `rule`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ManifestError {
    rule: ManifestRule,
}

impl ManifestError {
    pub fn rule(&self) -> ManifestRule {
        self.rule
    }
}

impl fmt::Display for ManifestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "the manifest breaks the {} rule", self.rule.as_str())
    }
}

impl std::error::Error for ManifestError {}

type Check = fn(&Value) -> bool;

/// Checks A1 §2's wire form and names the first rule it breaks.
pub fn validate_manifest(manifest: &Value) -> Result<(), ManifestError> {
    let checks: [(ManifestRule, Check); 9] = [
        (ManifestRule::ManifestVersion, |m| m["manifest_version"].as_u64() == Some(1)),
        (ManifestRule::DefaultLocale, default_locale_ok),
        (ManifestRule::Name, |m| localized_ok(&m["name"], NAME_MAX_CHARS)),
        (ManifestRule::Description, |m| localized_ok(&m["description"], DESCRIPTION_MAX_CHARS)),
        (ManifestRule::RenderUrl, render_url_ok),
        (ManifestRule::Slots, slots_ok),
        (ManifestRule::Context, context_ok),
        (ManifestRule::Duplicate, |m| ["slots", "context", "scopes"].iter().all(|k| distinct(&m[*k]))),
        (ManifestRule::TooLarge, |m| stored_bytes(m) <= MANIFEST_MAX_BYTES),
    ];
    match checks.into_iter().find(|(_, ok)| !ok(manifest)) {
        Some((rule, _)) => Err(ManifestError { rule }),
        None => Ok(()),
    }
}

fn default_locale_ok(m: &Value) -> bool {
    let Some(locale) = m["default_locale"].as_str().filter(|l| !l.is_empty()) else { return false };
    ["name", "description"].iter().all(|k| m[*k].as_object().is_none_or(|map| map.contains_key(locale)))
}

/// A non-empty locale map whose every value, trimmed, is 1–`max` scalar
/// values free of control, bidi and zero-width characters.
fn localized_ok(value: &Value, max: usize) -> bool {
    let forbidden = |c: char| crate::rules::stripped(c) || ('\u{200B}'..='\u{200F}').contains(&c) || c == '\u{FEFF}';
    let text_ok = |v: &Value| {
        v.as_str().map(str::trim).is_some_and(|t| (1..=max).contains(&t.chars().count()) && !t.chars().any(forbidden))
    };
    value.as_object().is_some_and(|map| !map.is_empty() && map.values().all(text_ok))
}

/// `https://` (scheme in any case) and then something other than `/`, `?`
/// or `#`.
fn render_url_ok(m: &Value) -> bool {
    let Some(url) = m["render_url"].as_str() else { return false };
    let scheme = url.get(..8).is_some_and(|s| s.eq_ignore_ascii_case("https://"));
    scheme && url[8..].chars().next().is_some_and(|c| !"/?#".contains(c))
}

fn slots_ok(m: &Value) -> bool {
    let slot = |v: &Value| v.as_str().is_some_and(|s| s.parse::<AppSlotName>().is_ok());
    m["slots"].as_array().is_some_and(|slots| !slots.is_empty() && slots.iter().all(slot))
}

fn context_ok(m: &Value) -> bool {
    let kind = |v: &Value| v.as_str().and_then(ContextSliceKind::parse).is_some();
    match m.get("context") {
        None => true,
        Some(Value::Array(kinds)) => kinds.iter().all(kind),
        Some(_) => false,
    }
}

fn distinct(list: &Value) -> bool {
    let Some(items) = list.as_array() else { return true };
    let unique: BTreeSet<String> = items.iter().map(Value::to_string).collect();
    unique.len() == items.len()
}

/// The compact JSON of A1's seven stored keys, with their defaults.
fn stored_bytes(m: &Value) -> usize {
    let get = |key: &str, default: Value| m.get(key).cloned().unwrap_or(default);
    let stored = json!({
        "default_locale": get("default_locale", Value::Null),
        "name": get("name", Value::Null),
        "description": get("description", Value::Null),
        "slots": get("slots", Value::Null),
        "context": get("context", json!([])),
        "scopes": get("scopes", json!([])),
        "tutor_note": get("tutor_note", json!(false)),
    });
    crate::limits::encode(&stored).len()
}
