package snippets;

// lingara:begin card
import com.getlingara.apps.Card;
import com.getlingara.apps.Item;

// lingara:end

/** The documentation site's card example. */
public final class CardSnippet {
  private CardSnippet() {}

  static Card card() {
    // lingara:begin card
    Card card =
        Card.card()
            .heading("Today's five", 1)
            .term("雨", "yǔ", "rain", "zh")
            .list(Item.text("Say it aloud"), Item.term("二", "èr", "two", "zh"))
            .button("Next", "next")
            .build(); // a CardLimitException names the first rule the card breaks
    // lingara:end
    return card;
  }
}
