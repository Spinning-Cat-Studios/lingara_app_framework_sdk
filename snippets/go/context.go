package snippets

import (
	"fmt"

	lingaraapps "github.com/Spinning-Cat-Studios/lingara_app_framework_sdk/go"
)

func contextCard(req lingaraapps.AppRenderRequest) (lingaraapps.Replier, error) {
	// lingara:begin context
	// The app's own client reads its owner's account, never the learner's.
	// The slices are all a render knows about the learner.
	//
	// Each slice is present only when the learner agreed to share it.
	card := lingaraapps.NewCard().Heading("Your plan", 1)
	for _, slice := range req.Context {
		switch s := slice.(type) {
		case lingaraapps.ContextSliceLanguages:
			card.Text(fmt.Sprintf("Learning %s from %s", s.TargetLang, s.SourceLang))
		case lingaraapps.ContextSlicePlanSummary:
			card.Text(fmt.Sprintf("%d of %d sets done", s.SetsCompleted, s.SetCount))
		}
	}
	// lingara:end
	return card.Build()
}
