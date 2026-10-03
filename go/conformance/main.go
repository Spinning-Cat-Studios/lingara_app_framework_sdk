// Command conformance is the Go kit's fixture app (conformance/CONTRACT.md
// §F; ADR 30.9.26am D9). It is built on the kit's public API and NewHandler
// only, on net/http, so the host's cases judge exactly what an app gets.
package main

import (
	"context"
	"errors"
	"fmt"
	"net"
	"net/http"
	"os"
	"strconv"
	"strings"

	lingaraapps "github.com/Spinning-Cat-Studios/lingara_app_framework_sdk/go"
)

func main() {
	if err := run(); err != nil {
		fmt.Fprintln(os.Stderr, "✗ fixture:", err)
		os.Exit(1)
	}
}

func run() error {
	secrets := strings.Split(os.Getenv("LINGARA_APPS_CONFORMANCE_SECRETS"), ",")
	handler, err := lingaraapps.NewHandler(app(), secrets...)
	if err != nil {
		return err
	}
	port := os.Getenv("LINGARA_APPS_CONFORMANCE_PORT")
	if port == "" {
		port = "0"
	}
	listener, err := net.Listen("tcp", net.JoinHostPort("127.0.0.1", port))
	if err != nil {
		return err
	}
	// The host waits for this as the first line of standard output.
	fmt.Printf("listening %d\n", listener.Addr().(*net.TCPAddr).Port)
	return http.Serve(listener, handler)
}

func app() lingaraapps.App {
	return lingaraapps.App{
		Render: render,
		Actions: map[string]lingaraapps.ActionFunc{
			"inc": inc,
			"boom": func(context.Context, lingaraapps.AppActionRequest) (lingaraapps.Replier, error) {
				return nil, errors.New("boom")
			},
			"overflow": overflow,
			"huge":     huge,
		},
	}
}

// render: a heading naming the slot, then one text per slice received, in
// order; on home.side, a tutor note too.
func render(_ context.Context, req lingaraapps.AppRenderRequest) (lingaraapps.Replier, error) {
	b := lingaraapps.NewCard().Heading(string(req.Slot), 1)
	for _, slice := range req.Context {
		b.Text(kind(slice))
	}
	card, err := b.Build()
	if err != nil {
		return nil, err
	}
	if req.Slot == lingaraapps.AppSlotNameHomeSide {
		return lingaraapps.Reply(card).TutorNote("fixture note"), nil
	}
	return card, nil
}

func kind(slice lingaraapps.ContextSlice) string {
	switch s := slice.(type) {
	case lingaraapps.ContextSliceLanguages:
		return string(s.Kind)
	case lingaraapps.ContextSlicePlanSummary:
		return string(s.Kind)
	case lingaraapps.ContextSliceReviewDue:
		return string(s.Kind)
	case lingaraapps.ContextSliceTutorTopic:
		return string(s.Kind)
	}
	return "unknown"
}

func inc(_ context.Context, req lingaraapps.AppActionRequest) (lingaraapps.Replier, error) {
	return lingaraapps.NewCard().Progress(0.5, "inc").Text(req.CardEtag).Build()
}

// overflow: 21 list items, which the card builder refuses (list_items).
func overflow(context.Context, lingaraapps.AppActionRequest) (lingaraapps.Replier, error) {
	items := make([]lingaraapps.ListItem, 21)
	for i := range items {
		items[i] = lingaraapps.Item.Text(strconv.Itoa(i + 1))
	}
	return lingaraapps.NewCard().List(items...).Build()
}

// huge: 24 legal elements whose reply is over 32 768 bytes (reply_too_large).
func huge(context.Context, lingaraapps.AppActionRequest) (lingaraapps.Replier, error) {
	b := lingaraapps.NewCard()
	for range 24 {
		b.Text(strings.Repeat("漢", 600))
	}
	return b.Build()
}
