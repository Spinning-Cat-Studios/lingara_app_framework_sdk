package snippets

// lingara:begin context
import com.getlingara.apps.kotlin.Reply
import com.getlingara.apps.kotlin.card
import com.getlingara.apps.kotlin.model.AppRenderRequest
import com.getlingara.apps.kotlin.model.ContextSliceLanguages
import com.getlingara.apps.kotlin.model.ContextSlicePlanSummary
import com.getlingara.kotlin.LingaraClient

// lingara:end

// lingara:begin context
// The app's own client-credentials client speaks for its owner; for any other learner, call
// with that learner's token instead.
val client =
    LingaraClient {
        clientCredentials(System.getenv("LINGARA_CLIENT_ID"), System.getenv("LINGARA_CLIENT_SECRET"))
    }

suspend fun renderWithContext(request: AppRenderRequest): Reply {
    // A slice is present only when the learner agreed to share it; the context is read-only.
    val languages = request.context.filterIsInstance<ContextSliceLanguages>().firstOrNull()
    val summary = request.context.filterIsInstance<ContextSlicePlanSummary>().firstOrNull()
    val plan = summary?.let { client.getLessonPlan(it.planId).body }
    return card {
        heading("Learning ${languages?.targetLang ?: "today"}", 1)
        if (summary != null && plan != null) {
            text("${plan.title}: ${summary.setsCompleted} of ${summary.setCount}")
        }
    }
}
// lingara:end
