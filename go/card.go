package lingaraapps

// The card and reply builders (ADR 30.9.26am D5). They refuse what Lingara's
// clamp would otherwise cut, strip or drop: Build runs the card rules and
// returns a *CardLimitError naming the first broken one.

// Term is a term element's or list item's fields. Word is required; an
// empty Reading, Gloss or Lang is left out of the card.
type Term struct {
	Word    string
	Reading string
	Gloss   string
	Lang    string
}

// CardBuilder builds a Card, one element per call, in order. Start one with
// NewCard; nothing is checked until Build.
type CardBuilder struct {
	elements []CardElement
}

// NewCard starts an empty card.
func NewCard() *CardBuilder { return &CardBuilder{} }

// Heading adds a heading of level 1 or 2.
func (b *CardBuilder) Heading(text string, level uint8) *CardBuilder {
	return b.add(CardElementHeading{Type: CardElementHeadingTypeHeading, Text: text, Level: level})
}

// Text adds a paragraph. `\n` is allowed in it.
func (b *CardBuilder) Text(text string) *CardBuilder {
	return b.add(CardElementText{Type: CardElementTextTypeText, Text: text})
}

// TextIn adds a paragraph in the language lang (a BCP 47 tag such as "zh").
func (b *CardBuilder) TextIn(lang, text string) *CardBuilder {
	return b.add(CardElementText{Type: CardElementTextTypeText, Text: text, Lang: optional(lang)})
}

// Term adds a word with its optional reading, gloss and language.
func (b *CardBuilder) Term(t Term) *CardBuilder {
	return b.add(CardElementTerm{
		Type: CardElementTermTypeTerm, Word: t.Word,
		Reading: optional(t.Reading), Gloss: optional(t.Gloss), Lang: optional(t.Lang),
	})
}

// List adds a list of 1 to 20 items, each made with Item.Text, Item.TextIn
// or Item.Term.
func (b *CardBuilder) List(items ...ListItem) *CardBuilder {
	return b.add(CardElementList{Type: CardElementListTypeList, Items: append([]ListItem{}, items...)})
}

// Progress adds a progress bar: value from 0 to 1, and its label.
func (b *CardBuilder) Progress(value float64, label string) *CardBuilder {
	return b.add(CardElementProgress{Type: CardElementProgressTypeProgress, Value: value, Label: label})
}

// Divider adds a rule between elements.
func (b *CardBuilder) Divider() *CardBuilder {
	return b.add(CardElementDivider{Type: CardElementDividerTypeDivider})
}

// Button adds a button. action is what comes back as the request's
// ActionID when it is pressed: 1 to 64 of A–Z a–z 0–9 _ . : -.
func (b *CardBuilder) Button(label, action string) *CardBuilder {
	return b.add(CardElementButton{Type: CardElementButtonTypeButton, Label: label, Action: action})
}

// StyledButton adds a button with a style.
func (b *CardBuilder) StyledButton(label, action string, style ButtonStyle) *CardBuilder {
	return b.add(CardElementButton{Type: CardElementButtonTypeButton, Label: label, Action: action, Style: &style})
}

// Link adds a link: an https URL with a host name, at most 2048 bytes.
func (b *CardBuilder) Link(label, url string) *CardBuilder {
	return b.add(CardElementLink{Type: CardElementLinkTypeLink, Label: label, URL: url})
}

func (b *CardBuilder) add(e CardElement) *CardBuilder {
	b.elements = append(b.elements, e)
	return b
}

// Build runs the card rules and returns the card, or a *CardLimitError.
func (b *CardBuilder) Build() (Card, error) {
	card := Card{Elements: append([]CardElement{}, b.elements...)}
	_, tree, err := encodeTree(AppCardReply{Card: card})
	if err != nil {
		return Card{}, err
	}
	var f found
	f.card(tree)
	if r := f.first(); r != "" {
		return Card{}, &CardLimitError{Reason: r}
	}
	return card, nil
}

// Item makes list items: Item.Text("one"), Item.Term(Term{Word: "二"}).
var Item itemMaker

type itemMaker struct{}

// Text is a list item of text. `\n` is allowed in it.
func (itemMaker) Text(text string) ListItem {
	return ListItemText{Type: ListItemTextTypeText, Text: text}
}

// TextIn is a list item of text in the language lang.
func (itemMaker) TextIn(lang, text string) ListItem {
	return ListItemText{Type: ListItemTextTypeText, Text: text, Lang: optional(lang)}
}

// Term is a list item of a word with its optional reading, gloss and language.
func (itemMaker) Term(t Term) ListItem {
	return ListItemTerm{
		Type: ListItemTermTypeTerm, Word: t.Word,
		Reading: optional(t.Reading), Gloss: optional(t.Gloss), Lang: optional(t.Lang),
	}
}

// optional leaves an empty string out of the card rather than sending "".
func optional(s string) *string {
	if s == "" {
		return nil
	}
	return &s
}

// Replier is what a render or action function returns: a Card, a
// *ReplyBuilder (a card and a tutor note), or an AppCardReply. The handler
// runs ValidateReply on it before anything is sent.
type Replier interface {
	appCardReply() (AppCardReply, error)
}

func (c Card) appCardReply() (AppCardReply, error) { return AppCardReply{Card: c}, nil }

func (r AppCardReply) appCardReply() (AppCardReply, error) { return r, nil }

// ReplyBuilder is a card with a tutor note: a line of plain text Lingara's
// tutor may use. Start one with Reply.
type ReplyBuilder struct {
	card Card
	note *string
}

// Reply starts a reply carrying card.
func Reply(card Card) *ReplyBuilder { return &ReplyBuilder{card: card} }

// TutorNote sets the tutor note: at most 280 characters once `\n` becomes a
// space and the ends are trimmed, with no control or bidi character but
// `\n`. It is plain text, never markup.
func (r *ReplyBuilder) TutorNote(text string) *ReplyBuilder {
	r.note = &text
	return r
}

// Build checks the tutor note and returns the reply, or a *CardLimitError.
func (r *ReplyBuilder) Build() (AppCardReply, error) {
	reply := AppCardReply{Card: r.card, TutorNote: r.note}
	var f found
	f.tutorNote(map[string]any{"tutor_note": derefOr(r.note)})
	if reason := f.first(); reason != "" {
		return AppCardReply{}, &CardLimitError{Reason: reason}
	}
	return reply, nil
}

func (r *ReplyBuilder) appCardReply() (AppCardReply, error) { return r.Build() }

// derefOr is *s, or nil when s is nil, so an unset note is absent.
func derefOr(s *string) any {
	if s == nil {
		return nil
	}
	return *s
}
