package snippets

import (
	"net/http"
	"os"

	lingaraapps "github.com/Spinning-Cat-Studios/lingara_app_framework_sdk/go"
)

func serve(app lingaraapps.App) error {
	// lingara:begin serve
	// The app's signing secret, lgr_whsec_…, or both during a rotation.
	handler, err := lingaraapps.NewHandler(app, os.Getenv("LINGARA_APP_SECRET"))
	if err != nil {
		return err // a malformed secret fails here, at startup
	}
	mux := http.NewServeMux()
	mux.Handle("/lingara", handler)
	err = http.ListenAndServe(":8080", mux)
	// lingara:end
	return err
}
