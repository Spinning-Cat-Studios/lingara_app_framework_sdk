package main

import (
	"bytes"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

const (
	viewPath   = "../../spec/generator/apps.3.0.json"
	modelsPath = "../models_gen.go"
)

// TestEmitterWritesTheThreeUnionsAndTheOperations: 30.9.26am AC17. Over the
// committed view, go/codegen writes CardElement, ListItem and ContextSlice
// as sealed interfaces with S1's lifted arm names, the four slice kinds, and
// the app.render / app.action operations beside the app.card reply; two runs
// are byte-identical; and it refuses a models file that declares a union.
func TestEmitterWritesTheThreeUnionsAndTheOperations(t *testing.T) {
	first, err := generate(viewPath, modelsPath)
	if err != nil {
		t.Fatal(err)
	}
	if second, _ := generate(viewPath, modelsPath); !bytes.Equal(first, second) {
		t.Error("two runs differ")
	}
	src := flat(first)
	for _, want := range []string{
		"type CardElement interface{ isCardElement() }",
		"type ListItem interface{ isListItem() }",
		"type ContextSlice interface{ isContextSlice() }",
		"func (CardElementHeading) isCardElement() {}",
		"func (CardElementList) isCardElement() {}",
		"func (CardElementLink) isCardElement() {}",
		"func (ListItemText) isListItem() {}",
		"func (ListItemTerm) isListItem() {}",
		"func (ContextSliceLanguages) isContextSlice() {}",
		"func (ContextSliceTutorTopic) isContextSlice() {}",
		`ContextSliceKindLanguages ContextSliceKind = "languages"`,
		`ContextSliceKindPlanSummary ContextSliceKind = "plan_summary"`,
		`ContextSliceKindReviewDue ContextSliceKind = "review_due"`,
		`ContextSliceKindTutorTopic ContextSliceKind = "tutor_topic"`,
		`case "plan_summary": var s ContextSlicePlanSummary`,
		`OperationAppRender Operation = "app.render"`,
		`OperationAppAction Operation = "app.action"`,
		"return []Operation{OperationAppRender, OperationAppAction}",
		`const ReplyMessage = "app.card"`,
	} {
		if !strings.Contains(src, want) {
			t.Errorf("apps_gen.go lacks %s", want)
		}
	}
	if n := strings.Count(src, ") isCardElement() {}"); n != 8 {
		t.Errorf("%d CardElement arms, want 8", n)
	}
	checkRefusals(t)
}

// checkRefusals: a models file that declares a name apps_gen.go writes, or
// lacks an arm, is refused.
func checkRefusals(t *testing.T) {
	t.Helper()
	models, err := os.ReadFile(modelsPath)
	if err != nil {
		t.Fatal(err)
	}
	for _, clash := range []string{"type CardElement struct{}", "type ContextSliceKind string"} {
		bad := filepath.Join(t.TempDir(), "models_gen.go")
		if err := os.WriteFile(bad, append(models, []byte("\n"+clash+"\n")...), 0o644); err != nil {
			t.Fatal(err)
		}
		if _, err := generate(viewPath, bad); err == nil || !strings.Contains(err.Error(), "exclude-schemas") {
			t.Errorf("%s: got %v", clash, err)
		}
	}
	noArm := filepath.Join(t.TempDir(), "models_gen.go")
	if err := os.WriteFile(noArm, bytes.ReplaceAll(models, []byte("type ListItemTerm struct"), []byte("type ListItemTermX struct")), 0o644); err != nil {
		t.Fatal(err)
	}
	if _, err := generate(viewPath, noArm); err == nil {
		t.Error("a models file without an arm was accepted")
	}
}

// flat collapses runs of whitespace, so assertions do not depend on
// gofmt's alignment.
func flat(src []byte) string { return strings.Join(strings.Fields(string(src)), " ") }
