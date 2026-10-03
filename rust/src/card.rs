//! The card builder (ADR 30.9.26am D5). A kit refuses what the
//! relay would clamp: `build` runs the card rules and returns the first one
//! the card breaks, so the relay's clamp never fires on a kit-built card.
//!
//! ```
//! use lingara_apps::{card, item, Term};
//!
//! let card = card()
//!     .heading("Today", 1)
//!     .term(Term::new("雨").reading("yǔ").gloss("rain").lang("zh"))
//!     .list([item::text("one"), item::term("二")])
//!     .button("Next", "next")
//!     .build()?;
//! # Ok::<(), lingara_apps::CardLimitError>(())
//! ```

use crate::generated::models::{
    ButtonStyle, Card, CardElementButton, CardElementDivider, CardElementHeading, CardElementLink, CardElementList, CardElementProgress,
    CardElementTerm, CardElementText,
};
use crate::generated::unions::{CardElement, ListItem};
use crate::content::{Term, Text};
use crate::reply::CardLimitError;
use crate::rules;

/// Starts a card.
pub fn card() -> CardBuilder {
    CardBuilder::default()
}

/// A card, one element per call, in order.
#[derive(Clone, Debug, Default)]
#[must_use]
pub struct CardBuilder {
    elements: Vec<CardElement>,
}

impl CardBuilder {
    pub fn heading(self, text: impl Into<String>, level: u8) -> Self {
        self.push(CardElementHeading { text: text.into(), level })
    }

    pub fn text(self, text: impl Into<Text>) -> Self {
        let Text { text, lang } = text.into();
        self.push(CardElementText { text, lang })
    }

    pub fn term(self, term: impl Into<Term>) -> Self {
        let Term { word, reading, gloss, lang } = term.into();
        self.push(CardElementTerm { word, reading, gloss, lang })
    }

    pub fn list(self, items: impl IntoIterator<Item = ListItem>) -> Self {
        self.push(CardElementList { items: items.into_iter().collect() })
    }

    /// `value` from 0 to 1.
    pub fn progress(self, value: f64, label: impl Into<String>) -> Self {
        self.push(CardElementProgress { value, label: label.into() })
    }

    pub fn divider(self) -> Self {
        self.push(CardElementDivider {})
    }

    /// `action` comes back as the request's `action_id`, which an action
    /// function is registered under.
    pub fn button(self, label: impl Into<String>, action: impl Into<String>) -> Self {
        self.push(CardElementButton { label: label.into(), action: action.into(), style: None })
    }

    pub fn styled_button(self, label: impl Into<String>, action: impl Into<String>, style: ButtonStyle) -> Self {
        self.push(CardElementButton { label: label.into(), action: action.into(), style: Some(style) })
    }

    /// An `https` URL on a named host, without user information.
    pub fn link(self, label: impl Into<String>, url: impl Into<String>) -> Self {
        self.push(CardElementLink { label: label.into(), url: url.into() })
    }

    fn push(mut self, element: impl Into<CardElement>) -> Self {
        self.elements.push(element.into());
        self
    }

    /// The card, or the first card rule it breaks.
    pub fn build(self) -> Result<Card, CardLimitError> {
        let card = Card { elements: self.elements };
        let value = serde_json::to_value(&card).unwrap_or_default();
        match rules::card(value.get("elements")).first() {
            Some(reason) => Err(CardLimitError::new(reason)),
            None => Ok(card),
        }
    }
}

