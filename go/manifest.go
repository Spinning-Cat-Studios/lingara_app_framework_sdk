package lingaraapps

import (
	"encoding/json"
	"strconv"
	"strings"
	"unicode"
	"unicode/utf8"
)

// The manifest builder (ADR 30.9.26am D5). The manifest is the form an app
// is registered with, uploaded in the console. It is in no spec the kit is
// generated from, so this is written by hand against Lingara's manifest
// format; its closed values (slots, context kinds) are the generated ones.

// Manifest limits.
const (
	MaxNameChars        = 40
	MaxDescriptionChars = 280
	MaxManifestBytes    = 65536
)

// ManifestRule is the rule a manifest breaks, as ValidateManifest names it.
type ManifestRule string

// The rules, in the order they are checked: a refusal names the first one
// broken.
const (
	RuleManifestVersion ManifestRule = "manifest_version"
	RuleDefaultLocale   ManifestRule = "default_locale"
	RuleName            ManifestRule = "name"
	RuleDescription     ManifestRule = "description"
	RuleRenderURL       ManifestRule = "render_url"
	RuleSlots           ManifestRule = "slots"
	RuleContext         ManifestRule = "context"
	RuleDuplicate       ManifestRule = "duplicate"
	RuleTooLarge        ManifestRule = "too_large"
)

// ManifestError is a manifest Lingara would refuse, or would change on
// upload, for a reason the kit can know. The supported-locale list, the rest
// of the URL policy and the scopes your client may ask for stay the
// upload's to check.
type ManifestError struct {
	Rule ManifestRule
}

func (e *ManifestError) Error() string {
	return "lingaraapps: the manifest breaks a rule: " + string(e.Rule)
}

// Manifest is an app's manifest, in the JSON form the console takes.
type Manifest struct {
	ManifestVersion int                `json:"manifest_version"`
	DefaultLocale   string             `json:"default_locale"`
	Name            map[string]string  `json:"name"`
	Description     map[string]string  `json:"description"`
	RenderURL       string             `json:"render_url"`
	Slots           []AppSlotName      `json:"slots"`
	Context         []ContextSliceKind `json:"context"`
	Scopes          []string           `json:"scopes"`
	TutorNote       bool               `json:"tutor_note"`
	Listed          bool               `json:"listed"`
}

// ToJSON writes the manifest as compact JSON, ready to save as manifest.json.
func (m Manifest) ToJSON() ([]byte, error) { return encode(m) }

// ManifestBuilder builds a Manifest. Start one with NewManifest.
type ManifestBuilder struct {
	m                 Manifest
	name, description *string
}

// NewManifest starts a manifest: version 1, unlisted, no context, no
// scopes, no tutor note.
func NewManifest() *ManifestBuilder {
	return &ManifestBuilder{m: Manifest{ManifestVersion: 1}}
}

// DefaultLocale is the locale the name and description always carry.
func (b *ManifestBuilder) DefaultLocale(locale string) *ManifestBuilder {
	b.m.DefaultLocale = locale
	return b
}

// Name is the app's name in the default locale alone.
func (b *ManifestBuilder) Name(name string) *ManifestBuilder {
	b.name, b.m.Name = &name, nil
	return b
}

// Names is the app's name per locale, the default locale's included.
func (b *ManifestBuilder) Names(byLocale map[string]string) *ManifestBuilder {
	b.name, b.m.Name = nil, byLocale
	return b
}

// Description is the app's description in the default locale alone.
func (b *ManifestBuilder) Description(description string) *ManifestBuilder {
	b.description, b.m.Description = &description, nil
	return b
}

// Descriptions is the app's description per locale.
func (b *ManifestBuilder) Descriptions(byLocale map[string]string) *ManifestBuilder {
	b.description, b.m.Description = nil, byLocale
	return b
}

// RenderURL is where Lingara sends the app's signed requests: https.
func (b *ManifestBuilder) RenderURL(u string) *ManifestBuilder {
	b.m.RenderURL = u
	return b
}

// Slots are where in Lingara the app's card may appear: at least one.
func (b *ManifestBuilder) Slots(slots ...AppSlotName) *ManifestBuilder {
	b.m.Slots = slots
	return b
}

// Context is the slices the app asks to read, each shared only when the
// learner agrees.
func (b *ManifestBuilder) Context(kinds ...ContextSliceKind) *ManifestBuilder {
	b.m.Context = kinds
	return b
}

// Scopes are the API scopes the app's client asks for.
func (b *ManifestBuilder) Scopes(scopes ...string) *ManifestBuilder {
	b.m.Scopes = scopes
	return b
}

// TutorNote declares that the app's replies may carry a tutor note.
func (b *ManifestBuilder) TutorNote(on bool) *ManifestBuilder {
	b.m.TutorNote = on
	return b
}

// Listed asks for the app to be listed for every learner, not only its
// owner. It is false unless set.
func (b *ManifestBuilder) Listed(on bool) *ManifestBuilder {
	b.m.Listed = on
	return b
}

// Build runs ValidateManifest and returns the manifest, or a *ManifestError.
func (b *ManifestBuilder) Build() (Manifest, error) {
	m := b.m
	if b.name != nil {
		m.Name = map[string]string{m.DefaultLocale: *b.name}
	}
	if b.description != nil {
		m.Description = map[string]string{m.DefaultLocale: *b.description}
	}
	m.Slots = append([]AppSlotName{}, m.Slots...)
	m.Context = append([]ContextSliceKind{}, m.Context...)
	m.Scopes = append([]string{}, m.Scopes...)
	if err := ValidateManifest(m); err != nil {
		return Manifest{}, err
	}
	return m, nil
}

// ValidateManifest checks a manifest (a Manifest, or anything that encodes
// to its JSON) against every rule the kit can know, in order, and returns a
// *ManifestError naming the first broken one.
func ValidateManifest(manifest any) error {
	_, tree, err := encodeTree(manifest)
	if err != nil {
		return err
	}
	m, _ := tree.(map[string]any)
	for _, check := range manifestChecks {
		if !check.ok(m) {
			return &ManifestError{Rule: check.rule}
		}
	}
	return nil
}

type manifestCheck struct {
	rule ManifestRule
	ok   func(map[string]any) bool
}

var manifestChecks = []manifestCheck{
	{RuleManifestVersion, func(m map[string]any) bool { return isInteger(m["manifest_version"], 1) }},
	{RuleDefaultLocale, defaultLocaleOK},
	{RuleName, func(m map[string]any) bool { return localeMapOK(m["name"], MaxNameChars) }},
	{RuleDescription, func(m map[string]any) bool { return localeMapOK(m["description"], MaxDescriptionChars) }},
	{RuleRenderURL, func(m map[string]any) bool { return renderURLOK(m["render_url"]) }},
	{RuleSlots, slotsOK},
	{RuleContext, contextOK},
	{RuleDuplicate, func(m map[string]any) bool {
		return distinct(m["slots"]) && distinct(m["context"]) && distinct(m["scopes"])
	}},
	{RuleTooLarge, storedSizeOK},
}

func isInteger(v any, want int64) bool {
	n, ok := v.(json.Number)
	if !ok {
		return false
	}
	got, err := strconv.ParseInt(string(n), 10, 64)
	return err == nil && got == want
}

// defaultLocaleOK: a non-empty string, and a key of name and of description
// wherever either is an object.
func defaultLocaleOK(m map[string]any) bool {
	locale, ok := m["default_locale"].(string)
	if !ok || locale == "" {
		return false
	}
	for _, key := range []string{"name", "description"} {
		if byLocale, isMap := m[key].(map[string]any); isMap {
			if _, has := byLocale[locale]; !has {
				return false
			}
		}
	}
	return true
}

// localeMapOK: a non-empty object whose every value, trimmed, is 1 to limit
// characters with no control, bidi or zero-width character.
func localeMapOK(v any, limit int) bool {
	byLocale, ok := v.(map[string]any)
	if !ok || len(byLocale) == 0 {
		return false
	}
	for _, value := range byLocale {
		s, ok := value.(string)
		if !ok {
			return false
		}
		trimmed := strings.TrimFunc(s, unicode.IsSpace)
		n := utf8.RuneCountInString(trimmed)
		if n < 1 || n > limit || strings.IndexFunc(trimmed, invisible) >= 0 {
			return false
		}
	}
	return true
}

// invisible is a control, a bidi override or isolate, or a zero-width
// character: none may be in a name or a description.
func invisible(r rune) bool {
	return stripped(r) || (r >= 0x200B && r <= 0x200F) || r == 0xFEFF
}

// renderURLOK: https:// (any case) and at least one character before the
// first /, ? or #.
func renderURLOK(v any) bool {
	s, ok := v.(string)
	const scheme = "https://"
	if !ok || len(s) <= len(scheme) || !strings.EqualFold(s[:len(scheme)], scheme) {
		return false
	}
	return !strings.ContainsRune("/?#", rune(s[len(scheme)]))
}

func slotsOK(m map[string]any) bool {
	slots, ok := m["slots"].([]any)
	if !ok || len(slots) == 0 {
		return false
	}
	for _, s := range slots {
		name, ok := s.(string)
		if !ok || !AppSlotName(name).Valid() {
			return false
		}
	}
	return true
}

func contextOK(m map[string]any) bool {
	v, present := m["context"]
	if !present {
		return true
	}
	kinds, ok := v.([]any)
	if !ok {
		return false
	}
	for _, k := range kinds {
		kind, ok := k.(string)
		if !ok || !isSliceKind(kind) {
			return false
		}
	}
	return true
}

func isSliceKind(kind string) bool {
	for _, known := range ContextSliceKinds() {
		if string(known) == kind {
			return true
		}
	}
	return false
}

// distinct: no value repeats in a list. Anything else has nothing to repeat.
func distinct(v any) bool {
	list, _ := v.([]any)
	seen := map[string]bool{}
	for _, item := range list {
		key, err := encode(item)
		if err != nil || seen[string(key)] {
			return false
		}
		seen[string(key)] = true
	}
	return true
}

// storedSizeOK: the seven keys Lingara stores, as compact JSON, fit in
// MaxManifestBytes. context and scopes default to [], tutor_note to false.
func storedSizeOK(m map[string]any) bool {
	stored := map[string]any{"context": []any{}, "scopes": []any{}, "tutor_note": false}
	for _, key := range []string{"default_locale", "name", "description", "slots", "context", "scopes", "tutor_note"} {
		if v, ok := m[key]; ok {
			stored[key] = v
		}
	}
	encoded, err := encode(stored)
	return err == nil && len(encoded) <= MaxManifestBytes
}
