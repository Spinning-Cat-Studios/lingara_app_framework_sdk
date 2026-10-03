//! The walk behind `validate_reply` and `Card::build` (ADR 30.9.26am D5):
//! S1's reference validator, ported rule for rule over the generic JSON tree,
//! so the vectors and a kit-built card are judged by one function.

use std::collections::BTreeSet;

use serde_json::Value;

use crate::limits::{self, Reason};

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

const HEADING: Field = field(limits::HEADING_MAX_CHARS, false, true);
const TEXT: Field = field(limits::TEXT_MAX_CHARS, true, true);
const WORD: Field = field(limits::TERM_WORD_MAX_CHARS, false, true);
const READING: Field = field(limits::TERM_READING_MAX_CHARS, false, false);
const GLOSS: Field = field(limits::TERM_GLOSS_MAX_CHARS, false, false);
const PROGRESS_LABEL: Field = field(limits::PROGRESS_LABEL_MAX_CHARS, false, true);
const BUTTON_LABEL: Field = field(limits::BUTTON_LABEL_MAX_CHARS, false, true);
const LINK_LABEL: Field = field(limits::LINK_LABEL_MAX_CHARS, false, true);
/// `lang`, `button.action` and `link.url`: no length limit of their own here.
const RAW: Field = field(usize::MAX, false, false);

/// The rules a reply breaks; the first in `Reason` order is the answer.
#[derive(Default)]
pub(crate) struct Found(BTreeSet<Reason>);

/// Reasons 1–12 over a card's `elements`: everything but the tutor note and
/// the encoded size.
pub(crate) fn card(elements: Option<&Value>) -> Found {
    let mut found = Found::default();
    let elements = elements.and_then(Value::as_array).map(Vec::as_slice).unwrap_or_default();
    elements.iter().for_each(|e| found.element(e));
    let buttons = elements.iter().filter(|e| e["type"] == "button").count();
    found.when(Reason::Buttons, buttons > limits::MAX_BUTTONS);
    found.when(Reason::Elements, elements.len() > limits::MAX_ELEMENTS);
    found.when(Reason::EmptyCard, elements.is_empty());
    found
}

impl Found {
    pub(crate) fn first(&self) -> Option<Reason> {
        self.0.first().copied()
    }

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
        Some(text.to_owned())
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
                self.when(Reason::ListItems, items.is_empty() || items.len() > limits::MAX_LIST_ITEMS);
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
                self.when(Reason::Link, !crate::link::is_safe_link(&url));
            }
            _ => {}
        }
    }

    pub(crate) fn tutor_note(&mut self, note: Option<&Value>) {
        let Some(note) = note.and_then(Value::as_str) else { return };
        self.when(Reason::ControlChars, note.chars().any(|c| stripped(c) && c != '\n'));
        // The relay turns `\n` into a space and trims before it counts.
        let line = note.replace('\n', " ");
        self.when(Reason::TutorNoteLength, line.trim().chars().count() > limits::TUTOR_NOTE_MAX_CHARS);
    }
}

/// C0 and C1 controls, and the bidi overrides and isolates.
pub(crate) fn stripped(c: char) -> bool {
    c.is_control() || ('\u{202A}'..='\u{202E}').contains(&c) || ('\u{2066}'..='\u{2069}').contains(&c)
}

/// The whole string matches `^[A-Za-z]{2,3}(-[A-Za-z0-9]{2,8}){0,3}$`.
pub(crate) fn is_lang_tag(tag: &str) -> bool {
    let mut parts = tag.split('-');
    let primary = parts.next().unwrap_or_default();
    let subtags: Vec<&str> = parts.collect();
    (2..=3).contains(&primary.len())
        && primary.chars().all(|c| c.is_ascii_alphabetic())
        && subtags.len() <= 3
        && subtags.iter().all(|s| (2..=8).contains(&s.len()) && s.chars().all(|c| c.is_ascii_alphanumeric()))
}

/// The whole string matches `^[A-Za-z0-9_.:-]{1,64}$`.
pub(crate) fn is_action_id(id: &str) -> bool {
    (1..=limits::ACTION_MAX_CHARS).contains(&id.len()) && id.chars().all(|c| c.is_ascii_alphanumeric() || "_.:-".contains(c))
}
