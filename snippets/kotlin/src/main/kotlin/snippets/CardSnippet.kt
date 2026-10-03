package snippets

// lingara:begin card
import com.getlingara.apps.kotlin.Card
import com.getlingara.apps.kotlin.card

// lingara:end

/** The documentation site's card example. */
fun cardSnippet(): Card {
    // lingara:begin card
    val c =
        card {
            heading("Today's five", 1)
            term("雨", reading = "yǔ", gloss = "rain", lang = "zh")
            list(item.text("Say it aloud"), item.term("二", reading = "èr", gloss = "two", lang = "zh"))
            button("Next", "next")
        } // a CardLimitException names the first rule the card breaks
    // lingara:end
    return c
}
