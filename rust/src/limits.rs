//! The card rules (ADR 30.9.26am D5; the contract's AK3): every limit as a
//! constant in this one file, `validate_reply`, and `truncate`.
//!
//! The relay clamps a reply that breaks a rule; a kit refuses it instead,
//! naming the rule the relay would have applied. A character is a Unicode
//! scalar value (`chars()`), never a grapheme or a UTF-16 unit, and "empty"
//! means empty after trimming Unicode `White_Space` (`str::trim`).

use std::fmt;

use serde_json::Value;

/// The largest reply body the relay reads, in bytes.
pub const REPLY_MAX_BYTES: usize = 32_768;
/// The largest request body a kit reads, in bytes (AK2).
pub const REQUEST_MAX_BYTES: usize = 65_536;
pub const MAX_ELEMENTS: usize = 24;
pub const MAX_BUTTONS: usize = 4;
pub const MAX_LIST_ITEMS: usize = 20;
pub const TUTOR_NOTE_MAX_CHARS: usize = 280;
pub const MAX_URL_BYTES: usize = 2_048;
pub const HEADING_MAX_CHARS: usize = 80;
pub const TEXT_MAX_CHARS: usize = 600;
pub const TERM_WORD_MAX_CHARS: usize = 60;
pub const TERM_READING_MAX_CHARS: usize = 120;
pub const TERM_GLOSS_MAX_CHARS: usize = 160;
pub const PROGRESS_LABEL_MAX_CHARS: usize = 60;
pub const BUTTON_LABEL_MAX_CHARS: usize = 32;
pub const LINK_LABEL_MAX_CHARS: usize = 60;
/// `button.action`, and so a request's `action_id`: `^[A-Za-z0-9_.:-]{1,64}$`.
pub const ACTION_MAX_CHARS: usize = 64;

/// Why a reply is refused. When a reply breaks several rules, the reason is
/// the first one here: the relay's eleven `ClampRule` spellings, then the
/// kit's own three.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
    pub const ALL: [Reason; 14] = [
        Reason::ControlChars,
        Reason::TextLength,
        Reason::HeadingLevel,
        Reason::ProgressRange,
        Reason::Lang,
        Reason::Link,
        Reason::ButtonAction,
        Reason::Buttons,
        Reason::ListItems,
        Reason::EmptyElement,
        Reason::Elements,
        Reason::EmptyCard,
        Reason::TutorNoteLength,
        Reason::ReplyTooLarge,
    ];

    /// The contract's snake_case spelling.
    pub const fn as_str(self) -> &'static str {
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

    pub fn parse(s: &str) -> Option<Reason> {
        Reason::ALL.into_iter().find(|r| r.as_str() == s)
    }
}

impl fmt::Display for Reason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Checks a whole `{card, tutor_note?}` against every rule and returns the
/// first one it breaks, or the bytes the kit sends: the last rule encodes the
/// reply exactly as sent, so the size checked is the size sent.
pub fn validate_reply(reply: &Value) -> Result<Vec<u8>, Reason> {
    let mut found = crate::rules::card(reply.pointer("/card/elements"));
    found.tutor_note(reply.get("tutor_note"));
    if let Some(first) = found.first() {
        return Err(first);
    }
    let bytes = encode(reply);
    if bytes.len() > REPLY_MAX_BYTES {
        return Err(Reason::ReplyTooLarge);
    }
    Ok(bytes)
}

/// Compact JSON, raw UTF-8, `/` unescaped: what `serde_json` writes, and
/// what the relay counts.
pub(crate) fn encode(reply: &Value) -> Vec<u8> {
    // A `Value` holds only strings, numbers, booleans, arrays and maps, none
    // of which can fail to serialise.
    serde_json::to_vec(reply).unwrap_or_default()
}

/// The relay's cut, only when asked for: a string over `limit` scalar values
/// becomes its first `limit − 1` and `…`; anything else is returned as is.
pub fn truncate(text: &str, limit: usize) -> String {
    if text.chars().count() <= limit {
        return text.to_owned();
    }
    let mut cut: String = text.chars().take(limit.saturating_sub(1)).collect();
    cut.push('…');
    cut
}
