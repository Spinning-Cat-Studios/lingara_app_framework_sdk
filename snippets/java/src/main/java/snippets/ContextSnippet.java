package snippets;

// lingara:begin context
import com.getlingara.apps.Card;
import com.getlingara.apps.Reply;
import com.getlingara.apps.model.AppRenderRequest;
import com.getlingara.apps.model.ContextSlice;
import com.getlingara.apps.model.ContextSliceLanguages;
import com.getlingara.apps.model.ContextSlicePlanSummary;

// lingara:end

/** The documentation site's context example. */
public final class ContextSnippet {
  private ContextSnippet() {}

  // lingara:begin context
  // The app's own client reads its owner's account, never the learner's. The slices are all a
  // render knows about the learner.
  static Reply render(AppRenderRequest request) {
    Card.Builder card = Card.card();
    // A slice is present only when the learner agreed to share it; the context is read-only.
    for (ContextSlice slice : request.getContext()) {
      if (slice instanceof ContextSliceLanguages languages) {
        card.heading("Learning " + languages.targetLang(), 1);
      } else if (slice instanceof ContextSlicePlanSummary summary) {
        card.text(summary.setsCompleted() + " of " + summary.setCount() + " sets done");
      }
    }
    return card.heading("Today", 2).build();
  }
  // lingara:end
}
