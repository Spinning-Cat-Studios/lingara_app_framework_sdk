package snippets

import (
	"context"
	"fmt"

	lingara "github.com/Spinning-Cat-Studios/lingara_api_clients/go"
	lingaraapps "github.com/Spinning-Cat-Studios/lingara_app_framework_sdk/go"
)

// client is the app's own client-credentials client (lingara.New with
// lingara.WithClientCredentials): it speaks for the app's owner.
func contextCard(ctx context.Context, client *lingara.Client, req lingaraapps.AppRenderRequest) (lingaraapps.Replier, error) {
	// lingara:begin context
	// Each slice is present only when the learner agreed to share it.
	card := lingaraapps.NewCard().Heading("Your plan", 1)
	for _, slice := range req.Context {
		switch s := slice.(type) {
		case lingaraapps.ContextSliceLanguages:
			card.Text(fmt.Sprintf("Learning %s from %s", s.TargetLang, s.SourceLang))
		case lingaraapps.ContextSlicePlanSummary:
			// The context is read-only; fetch the plan itself through the
			// lingara library.
			plan, err := client.GetLessonPlan(ctx, s.PlanID)
			if err != nil {
				return nil, err
			}
			card.Text(fmt.Sprintf("%d of %d sets done", s.SetsCompleted, s.SetCount)).
				Text(fmt.Sprintf("Level %d, %s", plan.Value.Level, plan.Value.Status))
		}
	}
	// lingara:end
	return card.Build()
}
