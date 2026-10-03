package snippets;

// lingara:begin tutorNote
import com.getlingara.apps.Card;
import com.getlingara.apps.Reply;

// lingara:end

/** The documentation site's tutorNote example. */
public final class TutorNoteSnippet {
  private TutorNoteSnippet() {}

  static Reply reply() {
    // lingara:begin tutorNote
    Card card = Card.card().heading("Today's five", 1).term("雨", "yǔ", "rain", "zh").build();
    // Plain text, at most 280 characters; the tutor reads it beside the card.
    return Reply.reply(card).tutorNote("The learner is reviewing weather words today.");
    // lingara:end
  }
}
