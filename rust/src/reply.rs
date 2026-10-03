//! The reply (ADR 30.9.26am D4, D5): a card and an optional tutor note, and
//! the error a card rule raises.

use std::fmt;

use serde_json::Value;

use crate::generated::models::{AppCardReply, Card};
use crate::limits::Reason;
use crate::rules;

/// A card or a reply broke a card rule: `reason` names the first.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CardLimitError {
    reason: Reason,
}

impl CardLimitError {
    pub(crate) fn new(reason: Reason) -> Self {
        CardLimitError { reason }
    }

    pub fn reason(&self) -> Reason {
        self.reason
    }
}

impl fmt::Display for CardLimitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "the card breaks the {} rule", self.reason)
    }
}

impl std::error::Error for CardLimitError {}

/// What a render or action function returns: a card, and optionally a
/// tutor note. A `Card` converts into one.
#[derive(Clone, Debug)]
pub struct Reply {
    card: Card,
    tutor_note: Option<String>,
}

/// A reply carrying `card`.
pub fn reply(card: Card) -> Reply {
    Reply { card, tutor_note: None }
}

impl Reply {
    /// Plain text, at most 280 characters once `\n` becomes a space; no
    /// control or bidi character but `\n`.
    pub fn tutor_note(mut self, note: impl Into<String>) -> Result<Reply, CardLimitError> {
        let note = note.into();
        let mut found = rules::Found::default();
        found.tutor_note(Some(&Value::String(note.clone())));
        if let Some(reason) = found.first() {
            return Err(CardLimitError::new(reason));
        }
        self.tutor_note = Some(note);
        Ok(self)
    }

    /// `{card, tutor_note?}`, as the core validates and sends it.
    pub fn to_value(&self) -> Value {
        let reply = AppCardReply { card: self.card.clone(), tutor_note: self.tutor_note.clone() };
        serde_json::to_value(&reply).unwrap_or_default()
    }
}

impl From<Card> for Reply {
    fn from(card: Card) -> Self {
        reply(card)
    }
}
