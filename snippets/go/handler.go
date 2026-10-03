package snippets

import (
	"context"
	"sync"

	lingaraapps "github.com/Spinning-Cat-Studios/lingara_app_framework_sdk/go"
)

func newApp() lingaraapps.App {
	// lingara:begin handler
	// Per-learner state is keyed on Subject, the lgr_sub_… value the app's
	// events carry too.
	var mu sync.Mutex
	streaks := map[string]int{}

	render := func(ctx context.Context, req lingaraapps.AppRenderRequest) (lingaraapps.Replier, error) {
		mu.Lock()
		days := streaks[req.Subject]
		mu.Unlock()
		return lingaraapps.NewCard().
			Heading("Daily five", 1).
			Progress(float64(days%5)/5, "This week").
			Button("Done today", "done").
			Build()
	}
	// An action may arrive twice: make it safe to run twice.
	done := func(ctx context.Context, req lingaraapps.AppActionRequest) (lingaraapps.Replier, error) {
		mu.Lock()
		streaks[req.Subject]++
		mu.Unlock()
		return lingaraapps.NewCard().Heading("Nice work", 1).Text("See you tomorrow.").Build()
	}
	app := lingaraapps.App{
		Render:  render,
		Actions: map[string]lingaraapps.ActionFunc{"done": done},
	}
	// lingara:end
	return app
}
