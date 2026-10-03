package snippets

// lingara:begin handler
import com.getlingara.apps.kotlin.LingaraApp
import com.getlingara.apps.kotlin.card
import com.getlingara.apps.kotlin.lingaraApp
import java.util.concurrent.ConcurrentHashMap

// lingara:end

/** The documentation site's handler example. */
fun handlerSnippet(): LingaraApp {
    // lingara:begin handler
    // Per-learner state is keyed on subject, the value the app's events carry too.
    val seen = ConcurrentHashMap<String, Int>()
    val app =
        lingaraApp {
            // The app's signing secret (lgr_whsec_…); pass two during a rotation.
            secrets(System.getenv("LINGARA_APP_SECRET"))
            render { request ->
                val count = seen.getOrDefault(request.subject, 0)
                card {
                    heading("Today's five", 1)
                    text("Reviewed $count so far.")
                    button("Next", "next")
                }
            }
            // The button's action comes back as the request's action_id. It may arrive twice,
            // so make it safe to repeat.
            action("next") { request ->
                val count = seen.merge(request.subject, 1, Int::plus) ?: 1
                card { progress(minOf(1.0, count / 5.0), "$count of 5") }
            }
        }
    // lingara:end
    return app
}
