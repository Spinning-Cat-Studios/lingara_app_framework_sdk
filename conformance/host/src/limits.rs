//! The reference validator (ADR 30.9.26al D5, D6): the relay's clamp
//! restated as refusals. Where the relay would cut, strip or drop part of a
//! reply, a kit refuses the whole reply instead, naming the rule the relay
//! would have applied. Every vector in `conformance/vectors/card-limits.json`
//! must agree with `validate_reply`, and each kit's own validator is held to
//! those vectors.
//!
//! "Characters" are Unicode scalar values (`chars()`), never graphemes or
//! UTF-16 units. "Empty" means empty after trimming Unicode `White_Space`
//! (`str::trim`). Both are the relay's definitions.

use std::collections::BTreeSet;
use std::net::IpAddr;

use serde_json::Value;

/// The largest reply body the relay reads, in bytes.
pub const REPLY_MAX_BYTES: usize = 32768;
pub const MAX_ELEMENTS: usize = 24;
pub const MAX_BUTTONS: usize = 4;
pub const MAX_LIST_ITEMS: usize = 20;
pub const TUTOR_NOTE_MAX_CHARS: usize = 280;
pub const MAX_URL_BYTES: usize = 2048;

/// Why a reply is refused. The order is the contract's: when a reply breaks
/// several rules, the reason is the first one here. The first eleven are the
/// relay's `ClampRule` spellings; the last three are the kit's own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Reason {
    ControlChars,
    TextLength,
    HeadingLevel,
    ProgressRange,
    Lang,
    Link,
    ButtonAction,
    Buttons,
    ListItems,
    EmptyElement,
    Elements,
    EmptyCard,
    TutorNoteLength,
    ReplyTooLarge,
}

impl Reason {
    #[cfg(test)]
    pub const ALL: [Reason; 14] = [
        Reason::ControlChars, Reason::TextLength, Reason::HeadingLevel, Reason::ProgressRange, Reason::Lang,
        Reason::Link, Reason::ButtonAction, Reason::Buttons, Reason::ListItems, Reason::EmptyElement,
        Reason::Elements, Reason::EmptyCard, Reason::TutorNoteLength, Reason::ReplyTooLarge,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Reason::ControlChars => "control_chars",
            Reason::TextLength => "text_length",
            Reason::HeadingLevel => "heading_level",
            Reason::ProgressRange => "progress_range",
            Reason::Lang => "lang",
            Reason::Link => "link",
            Reason::ButtonAction => "button_action",
            Reason::Buttons => "buttons",
            Reason::ListItems => "list_items",
            Reason::EmptyElement => "empty_element",
            Reason::Elements => "elements",
            Reason::EmptyCard => "empty_card",
            Reason::TutorNoteLength => "tutor_note_length",
            Reason::ReplyTooLarge => "reply_too_large",
        }
    }

    #[cfg(test)]
    pub fn parse(s: &str) -> Option<Reason> {
        Reason::ALL.into_iter().find(|r| r.as_str() == s)
    }
}

/// One text field's rule: its limit, whether `\n` is legal in it, and
/// whether it must be non-empty.
#[derive(Clone, Copy)]
struct Field {
    limit: usize,
    newline: bool,
    required: bool,
}

const fn field(limit: usize, newline: bool, required: bool) -> Field {
    Field { limit, newline, required }
}

const HEADING: Field = field(80, false, true);
const TEXT: Field = field(600, true, true);
const WORD: Field = field(60, false, true);
const READING: Field = field(120, false, false);
const GLOSS: Field = field(160, false, false);
const PROGRESS_LABEL: Field = field(60, false, true);
const BUTTON_LABEL: Field = field(32, false, true);
const LINK_LABEL: Field = field(60, false, true);
/// `lang`, `button.action` and `link.url`: no length limit of their own here.
const RAW: Field = field(usize::MAX, false, false);

/// The rules a reply breaks.
#[derive(Default)]
struct Found(BTreeSet<Reason>);

impl Found {
    fn when(&mut self, reason: Reason, broken: bool) {
        if broken {
            self.0.insert(reason);
        }
    }

    fn text(&mut self, value: Option<&Value>, rule: Field) -> Option<String> {
        let Some(text) = value.and_then(Value::as_str) else {
            self.when(Reason::EmptyElement, rule.required);
            return None;
        };
        self.when(Reason::ControlChars, text.chars().any(|c| stripped(c) && !(rule.newline && c == '\n')));
        self.when(Reason::TextLength, text.chars().count() > rule.limit);
        self.when(Reason::EmptyElement, rule.required && text.trim().is_empty());
        Some(text.to_string())
    }

    fn lang(&mut self, value: Option<&Value>) {
        if let Some(tag) = self.text(value.filter(|v| !v.is_null()), RAW) {
            self.when(Reason::Lang, !is_lang_tag(&tag));
        }
    }

    fn item(&mut self, item: &Value) {
        match item["type"].as_str() {
            Some("text") => {
                self.text(item.get("text"), TEXT);
                self.lang(item.get("lang"));
            }
            Some("term") => {
                self.text(item.get("word"), WORD);
                self.text(item.get("reading").filter(|v| !v.is_null()), READING);
                self.text(item.get("gloss").filter(|v| !v.is_null()), GLOSS);
                self.lang(item.get("lang"));
            }
            _ => {}
        }
    }

    fn element(&mut self, e: &Value) {
        match e["type"].as_str() {
            Some("heading") => {
                self.text(e.get("text"), HEADING);
                self.when(Reason::HeadingLevel, !matches!(e["level"].as_u64(), Some(1 | 2)));
            }
            Some("text" | "term") => self.item(e),
            Some("list") => {
                let items = e["items"].as_array().map(Vec::as_slice).unwrap_or_default();
                items.iter().for_each(|i| self.item(i));
                self.when(Reason::ListItems, items.is_empty() || items.len() > MAX_LIST_ITEMS);
            }
            Some("progress") => {
                self.when(Reason::ProgressRange, !e["value"].as_f64().is_some_and(|v| (0.0..=1.0).contains(&v)));
                self.text(e.get("label"), PROGRESS_LABEL);
            }
            Some("button") => {
                self.text(e.get("label"), BUTTON_LABEL);
                let action = self.text(e.get("action"), RAW).unwrap_or_default();
                self.when(Reason::ButtonAction, !is_action_id(&action));
            }
            Some("link") => {
                self.text(e.get("label"), LINK_LABEL);
                let url = self.text(e.get("url"), RAW).unwrap_or_default();
                self.when(Reason::Link, !is_safe_link(&url));
            }
            _ => {}
        }
    }

    fn tutor_note(&mut self, note: Option<&Value>) {
        let Some(note) = note.and_then(Value::as_str) else { return };
        self.when(Reason::ControlChars, note.chars().any(|c| stripped(c) && c != '\n'));
        // The relay turns `\n` into a space and trims before it counts.
        let line = note.replace('\n', " ");
        self.when(Reason::TutorNoteLength, line.trim().chars().count() > TUTOR_NOTE_MAX_CHARS);
    }
}

/// The first rule `reply` breaks, in `Reason` order, or `Ok`.
pub fn validate_reply(reply: &Value) -> Result<(), Reason> {
    let mut found = Found::default();
    let elements = reply.pointer("/card/elements").and_then(Value::as_array).map(Vec::as_slice).unwrap_or_default();
    elements.iter().for_each(|e| found.element(e));
    let buttons = elements.iter().filter(|e| e["type"] == "button").count();
    found.when(Reason::Buttons, buttons > MAX_BUTTONS);
    found.when(Reason::Elements, elements.len() > MAX_ELEMENTS);
    found.when(Reason::EmptyCard, elements.is_empty());
    found.tutor_note(reply.get("tutor_note"));
    found.when(Reason::ReplyTooLarge, encoded_len(reply) > REPLY_MAX_BYTES);
    found.0.first().map_or(Ok(()), |r| Err(*r))
}

/// The bytes a kit sends: compact JSON, raw UTF-8, no escaping beyond JSON's
/// own — what `serde_json` writes.
pub fn encoded_len(reply: &Value) -> usize {
    serde_json::to_vec(reply).map(|b| b.len()).unwrap_or(usize::MAX)
}

/// C0 and C1 controls, and the bidi overrides and isolates.
pub fn stripped(c: char) -> bool {
    c.is_control() || ('\u{202A}'..='\u{202E}').contains(&c) || ('\u{2066}'..='\u{2069}').contains(&c)
}

/// The whole string matches `^[A-Za-z]{2,3}(-[A-Za-z0-9]{2,8}){0,3}$`.
pub fn is_lang_tag(tag: &str) -> bool {
    let mut parts = tag.split('-');
    let primary = parts.next().unwrap_or_default();
    let subtags: Vec<&str> = parts.collect();
    (2..=3).contains(&primary.len())
        && primary.chars().all(|c| c.is_ascii_alphabetic())
        && subtags.len() <= 3
        && subtags.iter().all(|s| (2..=8).contains(&s.len()) && s.chars().all(|c| c.is_ascii_alphanumeric()))
}

/// The whole string matches `^[A-Za-z0-9_.:-]{1,64}$`.
pub fn is_action_id(id: &str) -> bool {
    (1..=64).contains(&id.len()) && id.chars().all(|c| c.is_ascii_alphanumeric() || "_.:-".contains(c))
}

/// `https`, no userinfo, a named host (never an IP literal), at most
/// `MAX_URL_BYTES`.
pub fn is_safe_link(raw: &str) -> bool {
    if raw.len() > MAX_URL_BYTES {
        return false;
    }
    let Ok(url) = url::Url::parse(raw) else { return false };
    let named = url.host_str().is_some_and(|h| h.trim_matches(['[', ']']).parse::<IpAddr>().is_err());
    url.scheme() == "https" && url.username().is_empty() && url.password().is_none() && named
}
