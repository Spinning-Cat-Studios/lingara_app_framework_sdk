package snippets

// lingara:begin context
import com.getlingara.apps.kotlin.Reply
import com.getlingara.apps.kotlin.card
import com.getlingara.apps.kotlin.model.AppRenderRequest
import com.getlingara.apps.kotlin.model.ContextSliceLanguages
import com.getlingara.apps.kotlin.model.ContextSlicePlanSummary

// lingara:end

// lingara:begin context
// The app's own client reads its owner's account, never the learner's. The slices are all a
// render knows about the learner.
fun renderWithContext(request: AppRenderRequest): Reply {
    // A slice is present only when the learner agreed to share it; the context is read-only.
    val languages = request.context.filterIsInstance<ContextSliceLanguages>().firstOrNull()
    val summary = request.context.filterIsInstance<ContextSlicePlanSummary>().firstOrNull()
    return card {
        heading("Learning ${languages?.targetLang ?: "today"}", 1)
        if (summary != null) {
            text("${summary.setsCompleted} of ${summary.setCount} sets done")
        }
    }
}
// lingara:end
