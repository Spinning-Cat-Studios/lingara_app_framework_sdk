package snippets

// lingara:begin tutorNote
import com.getlingara.apps.kotlin.Reply
import com.getlingara.apps.kotlin.card
import com.getlingara.apps.kotlin.reply

// lingara:end

/** The documentation site's tutorNote example. */
fun tutorNoteSnippet(): Reply {
    // lingara:begin tutorNote
    val c =
        card {
            heading("Today's five", 1)
            term("雨", reading = "yǔ", gloss = "rain", lang = "zh")
        }
    // Plain text, at most 280 characters; the tutor reads it beside the card.
    return reply(c).tutorNote("The learner is reviewing weather words today.")
    // lingara:end
}
