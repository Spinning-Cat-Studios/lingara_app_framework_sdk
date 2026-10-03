//! What a text or term element and a list item hold (ADR 30.9.26am D5): the
//! closed field set of the generated arms `CardElementText`, `ListItemText`,
//! `CardElementTerm` and `ListItemTerm`, each built from a bare string or
//! with its optional fields named.

/// A text element or list item: `text`, and an optional BCP 47 `lang`.
#[derive(Clone, Debug)]
pub struct Text {
    pub(crate) text: String,
    pub(crate) lang: Option<String>,
}

impl Text {
    pub fn new(text: impl Into<String>) -> Self {
        Text { text: text.into(), lang: None }
    }

    #[must_use]
    pub fn lang(mut self, lang: impl Into<String>) -> Self {
        self.lang = Some(lang.into());
        self
    }
}

impl From<&str> for Text {
    fn from(text: &str) -> Self {
        Text::new(text)
    }
}

impl From<String> for Text {
    fn from(text: String) -> Self {
        Text::new(text)
    }
}

/// A term element or list item: `word`, and optional `reading`, `gloss` and
/// `lang`.
#[derive(Clone, Debug)]
pub struct Term {
    pub(crate) word: String,
    pub(crate) reading: Option<String>,
    pub(crate) gloss: Option<String>,
    pub(crate) lang: Option<String>,
}

impl Term {
    pub fn new(word: impl Into<String>) -> Self {
        Term { word: word.into(), reading: None, gloss: None, lang: None }
    }

    #[must_use]
    pub fn reading(mut self, reading: impl Into<String>) -> Self {
        self.reading = Some(reading.into());
        self
    }

    #[must_use]
    pub fn gloss(mut self, gloss: impl Into<String>) -> Self {
        self.gloss = Some(gloss.into());
        self
    }

    #[must_use]
    pub fn lang(mut self, lang: impl Into<String>) -> Self {
        self.lang = Some(lang.into());
        self
    }
}

impl From<&str> for Term {
    fn from(word: &str) -> Self {
        Term::new(word)
    }
}

impl From<String> for Term {
    fn from(word: String) -> Self {
        Term::new(word)
    }
}

/// List items: `item::text(…)` and `item::term(…)`.
pub mod item {
    use super::{Term, Text};
    use crate::generated::models::{ListItemTerm, ListItemText};
    use crate::generated::unions::ListItem;

    pub fn text(text: impl Into<Text>) -> ListItem {
        let Text { text, lang } = text.into();
        ListItem::Text(ListItemText { text, lang })
    }

    pub fn term(term: impl Into<Term>) -> ListItem {
        let Term { word, reading, gloss, lang } = term.into();
        ListItem::Term(ListItemTerm { word, reading, gloss, lang })
    }
}

