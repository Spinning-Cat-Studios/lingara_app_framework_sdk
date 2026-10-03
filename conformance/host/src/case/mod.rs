//! The closed case format (ADR 30.9.26al D5). Every struct refuses an
//! unknown key, and every enumerable field is a closed set, so a case that
//! asks for something the host cannot do fails to load rather than passing
//! by doing less.

use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::Value;

/// The contract's five behaviours. Each must have at least one case.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum Behaviour {
    AK1,
    AK2,
    AK3,
    AK4,
    AK5,
}

pub const BEHAVIOURS: [Behaviour; 5] = [Behaviour::AK1, Behaviour::AK2, Behaviour::AK3, Behaviour::AK4, Behaviour::AK5];

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Case {
    pub id: String,
    pub behaviours: Vec<Behaviour>,
    pub request: Request,
    pub expect: Expect,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    /// An `x-lingara-app-operations` `message`. It sets a `json` body's
    /// `type` and chooses the time budget; required for a `json` body,
    /// `app.render` by default otherwise.
    #[serde(default)]
    pub operation: Option<String>,
    // serde_yaml reads an enum as a YAML tag by default; the case format
    // spells one as a single-key map (`body: { json: … }`).
    #[serde(with = "serde_yaml::with::singleton_map")]
    pub body: Body,
    #[serde(default)]
    pub sign: Sign,
    #[serde(default)]
    pub omit_headers: Vec<WebhookHeader>,
    #[serde(default)]
    pub tamper: Option<Tamper>,
    #[serde(default)]
    pub method: Method,
    #[serde(default)]
    pub header_case: HeaderCase,
}

impl Request {
    pub fn operation_or_default(&self) -> &str {
        self.operation.as_deref().unwrap_or("app.render")
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Body {
    /// What the relay would send; the host fills in `type` and `id`.
    Json(Value),
    /// Bytes sent and signed verbatim, never rewritten.
    Raw(String),
    /// `json`, compact, padded with spaces to exactly `to_bytes` bytes.
    RawPad(RawPad),
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawPad {
    pub json: Value,
    pub to_bytes: usize,
}

impl RawPad {
    /// Exactly `to_bytes` bytes of valid JSON, or why not.
    pub fn bytes(&self) -> Result<Vec<u8>, String> {
        let mut out = serde_json::to_vec(&self.json).map_err(|e| e.to_string())?;
        if out.len() > self.to_bytes {
            return Err(format!("raw_pad: the JSON is already {} bytes, over to_bytes {}", out.len(), self.to_bytes));
        }
        out.resize(self.to_bytes, b' ');
        Ok(out)
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Sign {
    /// Newest first, as the relay sends them during a rotation.
    #[serde(default = "primary_only")]
    pub secrets: Vec<SecretName>,
    #[serde(default)]
    pub timestamp_offset_s: i64,
    /// Appended to `webhook-signature` verbatim.
    #[serde(default)]
    pub extra_entries: Vec<String>,
}

impl Default for Sign {
    fn default() -> Self {
        Sign { secrets: primary_only(), timestamp_offset_s: 0, extra_entries: Vec::new() }
    }
}

fn primary_only() -> Vec<SecretName> {
    vec![SecretName::Primary]
}

/// The fixture app holds `primary` and `secondary`, never `unknown`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SecretName {
    Primary,
    Secondary,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum WebhookHeader {
    #[serde(rename = "webhook-id")]
    Id,
    #[serde(rename = "webhook-timestamp")]
    Timestamp,
    #[serde(rename = "webhook-signature")]
    Signature,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Tamper {
    /// One body byte changed after signing.
    Body,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
pub enum Method {
    #[default]
    #[serde(rename = "POST")]
    Post,
    #[serde(rename = "GET")]
    Get,
    #[serde(rename = "PUT")]
    Put,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HeaderCase {
    #[default]
    Lower,
    /// `Webhook-Id`, `Webhook-Signature`, …
    Title,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Expect {
    pub status: u16,
    #[serde(default, with = "serde_yaml::with::singleton_map")]
    pub body: Option<ExpectBody>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ExpectBody {
    Empty,
    /// JSON-equal after dropping nulls.
    Json(Value),
    MatchesSchema(SchemaName),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SchemaName {
    /// The operation's reply schema.
    Reply,
}

/// A case and the file it came from.
#[derive(Debug)]
pub struct Loaded {
    pub path: PathBuf,
    pub case: Case,
}

impl Loaded {
    /// `<group>.<file stem>`, the id the case must carry.
    pub fn path_id(&self) -> String {
        let stem = self.path.file_stem().map(|s| s.to_string_lossy()).unwrap_or_default();
        let group = self.path.parent().and_then(Path::file_name).map(|s| s.to_string_lossy()).unwrap_or_default();
        format!("{group}.{stem}")
    }
}

/// Every `<group>/*.yaml` under `dir`, sorted by path, and every file that
/// did not load, with why.
pub fn load_all(dir: &Path) -> (Vec<Loaded>, Vec<String>) {
    let mut paths: Vec<PathBuf> = fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|g| g.path().is_dir())
        .flat_map(|g| fs::read_dir(g.path()).into_iter().flatten().flatten())
        .map(|f| f.path())
        .filter(|p| p.extension().is_some_and(|e| e == "yaml"))
        .collect();
    paths.sort();
    let (mut loaded, mut errors) = (Vec::new(), Vec::new());
    for path in paths {
        match load(&path) {
            Ok(case) => loaded.push(Loaded { path, case }),
            Err(e) => errors.push(format!("{}: {e}", path.display())),
        }
    }
    (loaded, errors)
}

pub fn load(path: &Path) -> Result<Case, String> {
    let text = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let case: Case = serde_yaml::from_str(&text).map_err(|e| e.to_string())?;
    if let Body::RawPad(pad) = &case.request.body {
        pad.bytes()?;
    }
    Ok(case)
}

#[cfg(test)]
mod mod_tests;
