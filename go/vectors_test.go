package lingaraapps

import (
	"bytes"
	"encoding/json"
	"errors"
	"os"
	"strings"
	"testing"
	"unicode/utf8"
)

const (
	cardVectorsPath     = "../conformance/vectors/card-limits.json"
	manifestVectorsPath = "../conformance/vectors/manifest.json"
)

type expectation struct {
	OK      bool   `json:"ok"`
	Refused string `json:"refused"`
}

// readVectors decodes a vector file with numbers kept as written, so a
// reply re-encodes to the bytes the vector's author measured.
func readVectors(t *testing.T, path string, into any) {
	t.Helper()
	raw, err := os.ReadFile(path)
	if err != nil {
		t.Fatal(err)
	}
	dec := json.NewDecoder(bytes.NewReader(raw))
	dec.UseNumber()
	if err := dec.Decode(into); err != nil {
		t.Fatal(err)
	}
}

// TestEveryCardAndManifestVectorGivesItsExpectedAnswer: 30.9.26am AC3.
// Every card-limits.json vector through ValidateReply and every
// manifest.json vector through ValidateManifest gives its expected answer,
// and Truncate cuts an 81-scalar astral heading at 80 to 79 scalars and "…".
func TestEveryCardAndManifestVectorGivesItsExpectedAnswer(t *testing.T) {
	var cards struct {
		Vectors []struct {
			Name   string      `json:"name"`
			Reply  any         `json:"reply"`
			Expect expectation `json:"expect"`
		} `json:"vectors"`
	}
	readVectors(t, cardVectorsPath, &cards)
	if len(cards.Vectors) == 0 {
		t.Fatal("no card vectors")
	}
	for _, v := range cards.Vectors {
		_, err := ValidateReply(v.Reply)
		var limit *CardLimitError
		if errors.As(err, &limit) {
			check(t, v.Name, v.Expect, string(limit.Reason))
		} else {
			check(t, v.Name, v.Expect, errString(err))
		}
	}
	var manifests struct {
		Vectors []struct {
			Name     string      `json:"name"`
			Manifest any         `json:"manifest"`
			Expect   expectation `json:"expect"`
		} `json:"vectors"`
	}
	readVectors(t, manifestVectorsPath, &manifests)
	if len(manifests.Vectors) == 0 {
		t.Fatal("no manifest vectors")
	}
	for _, v := range manifests.Vectors {
		err := ValidateManifest(v.Manifest)
		var rule *ManifestError
		if errors.As(err, &rule) {
			check(t, v.Name, v.Expect, string(rule.Rule))
		} else {
			check(t, v.Name, v.Expect, errString(err))
		}
	}
	checkTruncate(t)
}

// check compares one answer, "" for accepted, with a vector's expectation.
func check(t *testing.T, name string, want expectation, got string) {
	t.Helper()
	switch {
	case want.OK && got != "":
		t.Errorf("%s: want ok, got %s", name, got)
	case !want.OK && got != want.Refused:
		t.Errorf("%s: want refused %s, got %q", name, want.Refused, got)
	}
}

func errString(err error) string {
	if err == nil {
		return ""
	}
	return "error: " + err.Error()
}

func checkTruncate(t *testing.T) {
	t.Helper()
	heading := strings.Repeat("𝄞", 81)
	cut := Truncate(heading, 80)
	if utf8.RuneCountInString(cut) != 80 || cut != strings.Repeat("𝄞", 79)+"…" {
		t.Errorf("Truncate(81 × 𝄞, 80) = %d scalars", utf8.RuneCountInString(cut))
	}
	if at := strings.Repeat("𝄞", 80); Truncate(at, 80) != at {
		t.Error("Truncate changed a heading at the limit")
	}
	if Truncate("é", 1) != "…" || Truncate("é", 2) != "é" {
		t.Error("Truncate counted something other than scalar values")
	}
}

// TestTheBuildersRefuseWhatTheClampWouldCut: the card builder runs the card
// rules and refuses, never cuts.
func TestTheBuildersRefuseWhatTheClampWouldCut(t *testing.T) {
	items := make([]ListItem, 21)
	for i := range items {
		items[i] = Item.Text("x")
	}
	if _, err := NewCard().List(items...).Build(); !isReason(err, ReasonListItems) {
		t.Errorf("21 items: %v", err)
	}
	if _, err := NewCard().Link("Docs", "http://example.com").Build(); !isReason(err, ReasonLink) {
		t.Errorf("http link: %v", err)
	}
	if _, err := NewCard().Button("Go", "has space").Build(); !isReason(err, ReasonButtonAction) {
		t.Errorf("bad action: %v", err)
	}
	if _, err := NewCard().Build(); !isReason(err, ReasonEmptyCard) {
		t.Errorf("empty card: %v", err)
	}
}

// TestTheReplyBuilder: the reply builder checks the tutor note, and the
// encoded reply carries it as sent.
func TestTheReplyBuilder(t *testing.T) {
	card, err := NewCard().Heading("Today", 1).Term(Term{Word: "雨", Reading: "yǔ", Gloss: "rain", Lang: "zh"}).
		List(Item.Text("one"), Item.Term(Term{Word: "二"})).Divider().Progress(0.5, "half").
		StyledButton("Next", "next", ButtonStylePrimary).Link("Docs", "https://getlingara.com/docs/").Build()
	if err != nil {
		t.Fatal(err)
	}
	if _, err := Reply(card).TutorNote(strings.Repeat("a", 281)).Build(); !isReason(err, ReasonTutorNoteLength) {
		t.Errorf("long note: %v", err)
	}
	body, err := ValidateReply(Reply(card).TutorNote("a\nb"))
	if err != nil || !bytes.Contains(body, []byte(`"tutor_note":"a\nb"`)) || bytes.Contains(body, []byte("null")) {
		t.Errorf("reply: %s %v", body, err)
	}
}

// TestTheManifestBuilder: Build runs ValidateManifest, a bare-string name is
// {default_locale: name}, and listed defaults to false.
func TestTheManifestBuilder(t *testing.T) {
	m, err := NewManifest().DefaultLocale("en").Name("Daily five").Description("Five words.").
		RenderURL("https://apps.example.com/render").Slots(AppSlotNameHomeSide).Context(ContextSliceKindLanguages).Build()
	if err != nil || m.Name["en"] != "Daily five" || m.Listed || m.Scopes == nil {
		t.Errorf("manifest: %+v %v", m, err)
	}
	_, err = NewManifest().DefaultLocale("en").Name("Daily five").Description("Five words.").
		RenderURL("https://apps.example.com/render").Build()
	var rule *ManifestError
	if !errors.As(err, &rule) || rule.Rule != RuleSlots {
		t.Errorf("no slots: %v", err)
	}
}

func isReason(err error, want Reason) bool {
	var limit *CardLimitError
	return errors.As(err, &limit) && limit.Reason == want
}

// TestTheReplyIsEncodedAsSent: compact, raw UTF-8, `/` and HTML unescaped,
// and no trailing newline.
func TestTheReplyIsEncodedAsSent(t *testing.T) {
	card, err := NewCard().Text("<a> & 漢 /").Build()
	if err != nil {
		t.Fatal(err)
	}
	body, err := ValidateReply(card)
	want := `{"card":{"elements":[{"text":"<a> & 漢 /","type":"text"}]}}`
	if err != nil || string(body) != want {
		t.Errorf("got %s %v, want %s", body, err, want)
	}
}
