package com.getlingara.apps;

import com.fasterxml.jackson.databind.node.ObjectNode;
import java.util.Objects;

/**
 * A card and, optionally, a tutor note: {@code Reply.reply(card).tutorNote("…")} (ADR 30.9.26am
 * D5). The note is plain text, never markup.
 */
public final class CardReply implements Reply {
  private final Card card;
  private final String tutorNote;

  CardReply(Card card, String tutorNote) {
    this.card = Objects.requireNonNull(card, "card");
    this.tutorNote = tutorNote;
  }

  /**
   * This reply with a tutor note. A {@code \n} is allowed, since the relay makes it a space.
   *
   * @param text the note, at most 280 characters once {@code \n} is a space and the ends trimmed
   * @return a new reply carrying the note
   * @throws CardLimitException {@code control_chars} or {@code tutor_note_length}
   */
  public CardReply tutorNote(String text) {
    Objects.requireNonNull(text, "text");
    ObjectNode probe = Json.MAPPER.createObjectNode().put("tutor_note", text);
    Limits.first(ReplyCheck.reply(probe.set("card", card.toTree()))).ifPresent(Limits::refuse);
    return new CardReply(card, text);
  }

  /**
   * The card.
   *
   * @return the card
   */
  public Card card() {
    return card;
  }

  /**
   * The tutor note, or null.
   *
   * @return the note
   */
  public String tutorNote() {
    return tutorNote;
  }

  /** {@code {card, tutor_note?}}, the note omitted when absent. */
  ObjectNode toTree() {
    ObjectNode tree = Json.MAPPER.createObjectNode();
    tree.set("card", card.toTree());
    if (tutorNote != null) {
      tree.put("tutor_note", tutorNote);
    }
    return tree;
  }
}
