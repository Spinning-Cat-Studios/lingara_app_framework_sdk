package snippets;

// lingara:begin context
import com.getlingara.apps.Card;
import com.getlingara.apps.Reply;
import com.getlingara.apps.model.AppRenderRequest;
import com.getlingara.apps.model.ContextSlice;
import com.getlingara.apps.model.ContextSliceLanguages;
import com.getlingara.apps.model.ContextSlicePlanSummary;
import com.getlingara.client.LingaraClient;
import com.getlingara.client.model.LessonPlan;

// lingara:end

/** The documentation site's context example. */
public final class ContextSnippet {
  private ContextSnippet() {}

  // lingara:begin context
  // The app's own client-credentials client speaks for its owner; for any other learner,
  // call with that learner's token instead.
  static final LingaraClient CLIENT =
      LingaraClient.builder()
          .clientCredentials(
              System.getenv("LINGARA_CLIENT_ID"), System.getenv("LINGARA_CLIENT_SECRET"))
          .build();

  static Reply render(AppRenderRequest request) {
    Card.Builder card = Card.card();
    // A slice is present only when the learner agreed to share it; the context is read-only.
    for (ContextSlice slice : request.getContext()) {
      if (slice instanceof ContextSliceLanguages languages) {
        card.heading("Learning " + languages.targetLang(), 1);
      } else if (slice instanceof ContextSlicePlanSummary summary) {
        LessonPlan plan = CLIENT.getLessonPlan(summary.planId()).body();
        card.text(plan.getTitle() + ": " + summary.setsCompleted() + " of " + summary.setCount());
      }
    }
    return card.heading("Today", 2).build();
  }
  // lingara:end
}
