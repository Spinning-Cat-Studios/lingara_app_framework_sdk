package snippets

// lingara:begin manifest
import com.getlingara.apps.kotlin.manifest
import com.getlingara.apps.kotlin.model.AppSlotName
import com.getlingara.apps.kotlin.model.ContextSliceKind
import java.io.File

// lingara:end

/** The documentation site's manifest example. */
fun manifestSnippet() {
    // lingara:begin manifest
    val json =
        manifest {
            defaultLocale = "en"
            name("Daily five")
            description("Five words to review, picked from your plan.")
            renderUrl = "https://apps.example.com/lingara/render"
            slots(AppSlotName.HOME_PERIOD_SIDE)
            context(ContextSliceKind.LANGUAGES, ContextSliceKind.PLAN_SUMMARY)
            // `scopes(…)` lists the API scopes your client uses, for the learner's consent page.
            // This app uses none.
            tutorNote = true
        }.toJson() // refuses what the upload would, naming the rule
    // Upload manifest.json in the console, beside the app's icon.
    File("manifest.json").writeText(json)
    // lingara:end
}
