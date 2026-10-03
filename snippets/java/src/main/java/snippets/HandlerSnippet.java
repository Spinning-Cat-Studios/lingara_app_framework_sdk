package snippets;

// lingara:begin handler
import com.getlingara.apps.Card;
import com.getlingara.apps.LingaraApp;
import java.util.Map;
import java.util.concurrent.ConcurrentHashMap;

// lingara:end

/** The documentation site's handler example. */
public final class HandlerSnippet {
  private HandlerSnippet() {}

  static LingaraApp app() {
    // lingara:begin handler
    // Per-learner state is keyed on subject, the value the app's events carry too.
    Map<String, Integer> seen = new ConcurrentHashMap<>();
    LingaraApp app =
        LingaraApp.builder()
            // The app's signing secret (lgr_whsec_…); pass two during a rotation.
            .secrets(System.getenv("LINGARA_APP_SECRET"))
            .render(
                request -> {
                  int count = seen.getOrDefault(request.getSubject(), 0);
                  return Card.card()
                      .heading("Today's five", 1)
                      .text("Reviewed " + count + " so far.")
                      .button("Next", "next")
                      .build();
                })
            // The button's action comes back as the request's action_id. It may arrive
            // twice, so make it safe to repeat.
            .action(
                "next",
                request -> {
                  int count = seen.merge(request.getSubject(), 1, Integer::sum);
                  return Card.card().progress(Math.min(1.0, count / 5.0), count + " of 5").build();
                })
            .build();
    // lingara:end
    return app;
  }
}
