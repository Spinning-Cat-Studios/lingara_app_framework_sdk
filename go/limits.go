package lingaraapps

import (
	"bytes"
	"encoding/json"
	"net"
	"net/url"
	"strconv"
	"strings"
	"unicode"
	"unicode/utf8"
)

// The card limits (ADR 30.9.26am D5; conformance/CONTRACT.md AK3). They are
// the relay's post-parse clamps, which the view carries no keyword for, so
// they live here, in one file. A character is a Unicode scalar value
// (utf8.RuneCountInString), never a byte or a grapheme.
const (
	// MaxRequestBytes is the largest request body the handler reads (AK2).
	MaxRequestBytes = 65536
	// MaxReplyBytes is the largest reply body Lingara reads (AK3).
	MaxReplyBytes      = 32768
	MaxElements        = 24
	MaxButtons         = 4
	MaxListItems       = 20
	MaxTutorNoteChars  = 280
	MaxURLBytes        = 2048
	MaxHeadingChars    = 80
	MaxTextChars       = 600
	MaxWordChars       = 60
	MaxReadingChars    = 120
	MaxGlossChars      = 160
	MaxProgressLabel   = 60
	MaxButtonLabel     = 32
	MaxLinkLabel       = 60
	MaxActionIDBytes   = 64
	maxUnboundedLength = int(^uint(0) >> 1)
)

// Reason is why a reply is refused: CONTRACT.md's fourteen, in its order.
// When a reply breaks several rules, the reason is the earliest of them.
type Reason string

// The fourteen reasons. The first eleven are the relay's clamp rules; the
// last three are the kit's own.
const (
	ReasonControlChars    Reason = "control_chars"
	ReasonTextLength      Reason = "text_length"
	ReasonHeadingLevel    Reason = "heading_level"
	ReasonProgressRange   Reason = "progress_range"
	ReasonLang            Reason = "lang"
	ReasonLink            Reason = "link"
	ReasonButtonAction    Reason = "button_action"
	ReasonButtons         Reason = "buttons"
	ReasonListItems       Reason = "list_items"
	ReasonEmptyElement    Reason = "empty_element"
	ReasonElements        Reason = "elements"
	ReasonEmptyCard       Reason = "empty_card"
	ReasonTutorNoteLength Reason = "tutor_note_length"
	ReasonReplyTooLarge   Reason = "reply_too_large"
)

// reasonOrder is the contract's order, earliest first.
var reasonOrder = []Reason{
	ReasonControlChars, ReasonTextLength, ReasonHeadingLevel, ReasonProgressRange, ReasonLang,
	ReasonLink, ReasonButtonAction, ReasonButtons, ReasonListItems, ReasonEmptyElement,
	ReasonElements, ReasonEmptyCard, ReasonTutorNoteLength, ReasonReplyTooLarge,
}

// CardLimitError is a card or reply that breaks a card rule. The kit refuses
// such a reply rather than cutting it, so Lingara's clamp never fires on it.
type CardLimitError struct {
	Reason Reason
}

func (e *CardLimitError) Error() string {
	return "lingaraapps: the card breaks a card limit: " + string(e.Reason)
}

// ValidateReply checks a whole {card, tutor_note?} against every card rule
// and returns the bytes the handler sends: compact JSON, raw UTF-8, no HTML
// or `/` escaping, an absent tutor note omitted. The last rule measures
// those bytes, so the size checked is the size sent. A refusal is a
// *CardLimitError naming the first broken rule.
//
// reply is an AppCardReply, a Card, a *ReplyBuilder, or any value that
// encodes to the reply's JSON shape (a decoded map, say).
func ValidateReply(reply any) ([]byte, error) {
	if r, ok := reply.(Replier); ok {
		built, err := r.appCardReply()
		if err != nil {
			return nil, err
		}
		reply = built
	}
	encoded, tree, err := encodeTree(reply)
	if err != nil {
		return nil, err
	}
	var f found
	f.card(tree)
	f.tutorNote(tree)
	f.when(ReasonReplyTooLarge, len(encoded) > MaxReplyBytes)
	if r := f.first(); r != "" {
		return nil, &CardLimitError{Reason: r}
	}
	return encoded, nil
}

// Truncate returns text cut to at most limit characters the way Lingara's
// clamp cuts it: over the limit, its first limit−1 characters and "…";
// otherwise unchanged. The kit never calls it for you.
func Truncate(text string, limit int) string {
	if limit < 1 || utf8.RuneCountInString(text) <= limit {
		return text
	}
	cut, n := 0, 0
	for i := range text {
		if n == limit-1 {
			cut = i
			break
		}
		n++
	}
	return text[:cut] + "…"
}

// encode writes v as the handler sends it. json.Marshal escapes <, > and &
// and Encode appends a newline; neither is in the bytes Lingara counts.
func encode(v any) ([]byte, error) {
	var buf bytes.Buffer
	enc := json.NewEncoder(&buf)
	enc.SetEscapeHTML(false)
	if err := enc.Encode(v); err != nil {
		return nil, err
	}
	return bytes.TrimSuffix(buf.Bytes(), []byte("\n")), nil
}

// encodeTree encodes v and decodes the bytes back as a generic tree, with
// numbers kept as written, so the rules read exactly what is sent.
func encodeTree(v any) ([]byte, any, error) {
	encoded, err := encode(v)
	if err != nil {
		return nil, nil, err
	}
	dec := json.NewDecoder(bytes.NewReader(encoded))
	dec.UseNumber()
	var tree any
	if err := dec.Decode(&tree); err != nil {
		return nil, nil, err
	}
	return encoded, tree, nil
}

// field is one text field's rule: its limit, whether `\n` is legal in it,
// and whether it must be present and non-empty.
type field struct {
	limit    int
	newline  bool
	required bool
}

var (
	headingField       = field{MaxHeadingChars, false, true}
	textField          = field{MaxTextChars, true, true}
	wordField          = field{MaxWordChars, false, true}
	readingField       = field{MaxReadingChars, false, false}
	glossField         = field{MaxGlossChars, false, false}
	progressLabelField = field{MaxProgressLabel, false, true}
	buttonLabelField   = field{MaxButtonLabel, false, true}
	linkLabelField     = field{MaxLinkLabel, false, true}
	// rawField is lang, button.action and link.url: no length rule here.
	rawField = field{maxUnboundedLength, false, false}
)

// found collects the rules a reply breaks.
type found map[Reason]bool

func (f *found) when(r Reason, broken bool) {
	if !broken {
		return
	}
	if *f == nil {
		*f = found{}
	}
	(*f)[r] = true
}

// first is the earliest broken rule in the contract's order, or "".
func (f found) first() Reason {
	for _, r := range reasonOrder {
		if f[r] {
			return r
		}
	}
	return ""
}

// member reads a JSON object's member: nil when absent, null or not an
// object.
func member(v any, key string) any {
	if m, ok := v.(map[string]any); ok {
		return m[key]
	}
	return nil
}

func (f *found) text(value any, rule field) (string, bool) {
	s, ok := value.(string)
	if !ok {
		f.when(ReasonEmptyElement, rule.required)
		return "", false
	}
	f.when(ReasonControlChars, strings.IndexFunc(s, func(r rune) bool { return stripped(r) && (!rule.newline || r != '\n') }) >= 0)
	f.when(ReasonTextLength, utf8.RuneCountInString(s) > rule.limit)
	f.when(ReasonEmptyElement, rule.required && strings.TrimFunc(s, unicode.IsSpace) == "")
	return s, true
}

func (f *found) lang(value any) {
	if tag, ok := f.text(value, rawField); ok {
		f.when(ReasonLang, !isLangTag(tag))
	}
}

func (f *found) item(item any) {
	switch member(item, "type") {
	case "text":
		f.text(member(item, "text"), textField)
		f.lang(member(item, "lang"))
	case "term":
		f.text(member(item, "word"), wordField)
		f.text(member(item, "reading"), readingField)
		f.text(member(item, "gloss"), glossField)
		f.lang(member(item, "lang"))
	}
}

func (f *found) element(e any) {
	switch member(e, "type") {
	case "heading":
		f.text(member(e, "text"), headingField)
		f.when(ReasonHeadingLevel, !isLevel(member(e, "level")))
	case "text", "term":
		f.item(e)
	case "list":
		items, _ := member(e, "items").([]any)
		for _, i := range items {
			f.item(i)
		}
		f.when(ReasonListItems, len(items) == 0 || len(items) > MaxListItems)
	case "progress":
		f.when(ReasonProgressRange, !isUnit(member(e, "value")))
		f.text(member(e, "label"), progressLabelField)
	case "button":
		f.text(member(e, "label"), buttonLabelField)
		action, _ := f.text(member(e, "action"), rawField)
		f.when(ReasonButtonAction, !isActionID(action))
	case "link":
		f.text(member(e, "label"), linkLabelField)
		u, _ := f.text(member(e, "url"), rawField)
		f.when(ReasonLink, !isSafeLink(u))
	}
}

// card runs the card rules, reasons control_chars to empty_card.
func (f *found) card(reply any) {
	elements, _ := member(member(reply, "card"), "elements").([]any)
	buttons := 0
	for _, e := range elements {
		f.element(e)
		if member(e, "type") == "button" {
			buttons++
		}
	}
	f.when(ReasonButtons, buttons > MaxButtons)
	f.when(ReasonElements, len(elements) > MaxElements)
	f.when(ReasonEmptyCard, len(elements) == 0)
}

// tutorNote: `\n` is legal, because the relay turns it into a space and
// trims before it counts.
func (f *found) tutorNote(reply any) {
	note, ok := member(reply, "tutor_note").(string)
	if !ok {
		return
	}
	f.when(ReasonControlChars, strings.IndexFunc(note, func(r rune) bool { return stripped(r) && r != '\n' }) >= 0)
	line := strings.TrimFunc(strings.ReplaceAll(note, "\n", " "), unicode.IsSpace)
	f.when(ReasonTutorNoteLength, utf8.RuneCountInString(line) > MaxTutorNoteChars)
}

// stripped is a C0 or C1 control, a bidi override or a bidi isolate.
func stripped(r rune) bool {
	return unicode.IsControl(r) || (r >= 0x202A && r <= 0x202E) || (r >= 0x2066 && r <= 0x2069)
}

// isLevel: an integer, 1 or 2.
func isLevel(v any) bool {
	n, ok := v.(json.Number)
	if !ok {
		return false
	}
	level, err := strconv.ParseUint(string(n), 10, 64)
	return err == nil && (level == 1 || level == 2)
}

// isUnit: a number from 0 to 1 inclusive.
func isUnit(v any) bool {
	n, ok := v.(json.Number)
	if !ok {
		return false
	}
	x, err := n.Float64()
	return err == nil && x >= 0 && x <= 1
}

// isLangTag: the whole string matches ^[A-Za-z]{2,3}(-[A-Za-z0-9]{2,8}){0,3}$.
func isLangTag(tag string) bool {
	parts := strings.Split(tag, "-")
	primary, subtags := parts[0], parts[1:]
	if len(primary) < 2 || len(primary) > 3 || !allASCII(primary, false) || len(subtags) > 3 {
		return false
	}
	for _, s := range subtags {
		if len(s) < 2 || len(s) > 8 || !allASCII(s, true) {
			return false
		}
	}
	return true
}

// allASCII: every byte is an ASCII letter, or a letter or digit.
func allASCII(s string, digits bool) bool {
	for i := 0; i < len(s); i++ {
		c := s[i] | 0x20
		letter := c >= 'a' && c <= 'z'
		digit := digits && s[i] >= '0' && s[i] <= '9'
		if !letter && !digit {
			return false
		}
	}
	return true
}

// isActionID: the whole string matches ^[A-Za-z0-9_.:-]{1,64}$.
func isActionID(id string) bool {
	if len(id) < 1 || len(id) > MaxActionIDBytes {
		return false
	}
	return strings.Trim(id, "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_.:-") == ""
}

// isSafeLink: https, no user information, a host name (never an IP
// address), at most MaxURLBytes. A host whose last label is a number is an
// IPv4 address to a WHATWG parser (127.1, 0x7f.1), as it is to the relay.
func isSafeLink(raw string) bool {
	if len(raw) > MaxURLBytes {
		return false
	}
	u, err := url.Parse(raw)
	if err != nil || !strings.EqualFold(u.Scheme, "https") || u.Opaque != "" {
		return false
	}
	if u.User != nil && (u.User.Username() != "" || hasPassword(u.User)) {
		return false
	}
	host := u.Hostname()
	return host != "" && net.ParseIP(host) == nil && !endsInNumber(host)
}

func hasPassword(u *url.Userinfo) bool {
	_, set := u.Password()
	return set
}

// endsInNumber is the WHATWG host parser's test for an IPv4 host: the last
// label (a trailing dot ignored) is decimal digits, or 0x and hex digits.
func endsInNumber(host string) bool {
	labels := strings.Split(strings.TrimSuffix(host, "."), ".")
	last := strings.ToLower(labels[len(labels)-1])
	if last == "" {
		return false
	}
	if hex, ok := strings.CutPrefix(last, "0x"); ok {
		return strings.Trim(hex, "0123456789abcdef") == ""
	}
	return strings.Trim(last, "0123456789") == ""
}
