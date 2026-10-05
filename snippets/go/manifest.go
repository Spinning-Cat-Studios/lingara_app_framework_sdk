// Package snippets holds the Go app kit examples the documentation site
// shows. Each marked region is vendored at a released tag, and `go vet` in
// make test-go compiles every one.
package snippets

import (
	"os"

	lingaraapps "github.com/Spinning-Cat-Studios/lingara_app_framework_sdk/go"
)

func writeManifest() error {
	// lingara:begin manifest
	manifest, err := lingaraapps.NewManifest().
		DefaultLocale("en").
		Names(map[string]string{"en": "Daily five", "zh-Hans": "每日五词"}).
		Description("Five words to review, picked from your plan.").
		RenderURL("https://apps.example.com/lingara").
		Slots(lingaraapps.AppSlotNameHomeSide, lingaraapps.AppSlotNamePlansEmptyDetail).
		Context(lingaraapps.ContextSliceKindLanguages, lingaraapps.ContextSliceKindPlanSummary).
		// `.Scopes(…)` lists the API scopes your client uses, for the learner's consent page. This app uses none.
		TutorNote(true).
		Build()
	if err != nil {
		return err // a *lingaraapps.ManifestError names the broken rule
	}
	data, err := manifest.ToJSON()
	if err != nil {
		return err
	}
	// Upload manifest.json in the console.
	err = os.WriteFile("manifest.json", data, 0o644)
	// lingara:end
	return err
}
