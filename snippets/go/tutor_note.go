package snippets

import (
	"context"

	lingaraapps "github.com/Spinning-Cat-Studios/lingara_app_framework_sdk/go"
)

func renderWithNote(ctx context.Context, req lingaraapps.AppRenderRequest) (lingaraapps.Replier, error) {
	// lingara:begin tutorNote
	card, err := lingaraapps.NewCard().Heading("Daily five", 1).Text("雨 · 雪 · 风 · 云 · 雷").Build()
	if err != nil {
		return nil, err
	}
	// Plain text, at most 280 characters, for Lingara's tutor to use.
	return lingaraapps.Reply(card).
		TutorNote("The learner is reviewing weather words today."), nil
	// lingara:end
}
