package com.getlingara.apps;

import com.fasterxml.jackson.databind.JsonNode;
import com.getlingara.apps.Limits.Reason;
import java.util.EnumSet;

/**
 * The rules a reply breaks, every one of them (ADR 30.9.26am D5): the contract's reference
 * validator, read rule for rule. It reads the generic JSON tree, so a vector, a builder-made card
 * and a developer's reply are checked by one walk. {@link Limits} picks the first.
 */
final class ReplyCheck {
  /**
   * One text field's rule: its limit, whether {@code \n} is legal in it, whether it is required.
   */
  private record Field(int limit, boolean newline, boolean required) {}

  private static final Field HEADING = new Field(Limits.HEADING_MAX_CHARS, false, true);
  private static final Field TEXT = new Field(Limits.TEXT_MAX_CHARS, true, true);
  private static final Field WORD = new Field(Limits.TERM_WORD_MAX_CHARS, false, true);
  private static final Field READING = new Field(Limits.TERM_READING_MAX_CHARS, false, false);
  private static final Field GLOSS = new Field(Limits.TERM_GLOSS_MAX_CHARS, false, false);
  private static final Field PROGRESS_LABEL =
      new Field(Limits.PROGRESS_LABEL_MAX_CHARS, false, true);
  private static final Field BUTTON_LABEL = new Field(Limits.BUTTON_LABEL_MAX_CHARS, false, true);
  private static final Field LINK_LABEL = new Field(Limits.LINK_LABEL_MAX_CHARS, false, true);

  /** {@code lang}, {@code button.action} and {@code link.url}: no length limit of their own. */
  private static final Field RAW = new Field(Integer.MAX_VALUE, false, false);

  private final EnumSet<Reason> found = EnumSet.noneOf(Reason.class);

  private ReplyCheck() {}

  /** Every card rule and the tutor-note rule {@code reply} breaks; never the size rule. */
  static EnumSet<Reason> reply(JsonNode reply) {
    ReplyCheck check = new ReplyCheck();
    check.elements(reply.path("card").path("elements"));
    check.tutorNote(reply.path("tutor_note"));
    return check.found;
  }

  /** Every card rule {@code card}, a {@code {elements}} object, breaks. */
  static EnumSet<Reason> card(JsonNode card) {
    ReplyCheck check = new ReplyCheck();
    check.elements(card.path("elements"));
    return check.found;
  }

  private void elements(JsonNode elements) {
    int count = elements.isArray() ? elements.size() : 0;
    int buttons = 0;
    for (int i = 0; i < count; i++) {
      JsonNode e = elements.get(i);
      element(e);
      buttons += typeOf(e).equals("button") ? 1 : 0;
    }
    when(Reason.BUTTONS, buttons > Limits.MAX_BUTTONS);
    when(Reason.ELEMENTS, count > Limits.MAX_ELEMENTS);
    when(Reason.EMPTY_CARD, count == 0);
  }

  private void when(Reason reason, boolean broken) {
    if (broken) {
      found.add(reason);
    }
  }

  /** Checks one text field; returns the string, or null when it is absent or not a string. */
  private String text(JsonNode value, Field rule) {
    if (value == null || !value.isTextual()) {
      when(Reason.EMPTY_ELEMENT, rule.required());
      return null;
    }
    String text = value.asText();
    when(Reason.CONTROL_CHARS, Text.hasControl(text, rule.newline()));
    when(Reason.TEXT_LENGTH, Text.scalars(text) > rule.limit());
    when(Reason.EMPTY_ELEMENT, rule.required() && Text.trim(text).isEmpty());
    return text;
  }

  /** A member, treating JSON {@code null} as absent, as the reference does for optional fields. */
  private static JsonNode optional(JsonNode parent, String name) {
    JsonNode value = parent.get(name);
    return value == null || value.isNull() ? null : value;
  }

  private void lang(JsonNode value) {
    String tag = text(value, RAW);
    if (tag != null) {
      when(Reason.LANG, !Text.isLangTag(tag));
    }
  }

  private void item(JsonNode item) {
    switch (typeOf(item)) {
      case "text" -> {
        text(item.get("text"), TEXT);
        lang(optional(item, "lang"));
      }
      case "term" -> {
        text(item.get("word"), WORD);
        text(optional(item, "reading"), READING);
        text(optional(item, "gloss"), GLOSS);
        lang(optional(item, "lang"));
      }
      default -> {
        // Not an item this rule set knows: the reference ignores it too.
      }
    }
  }

  private static String typeOf(JsonNode node) {
    JsonNode type = node.path("type");
    return type.isTextual() ? type.asText() : "";
  }

  private void element(JsonNode e) {
    switch (typeOf(e)) {
      case "heading" -> heading(e);
      case "text", "term" -> item(e);
      case "list" -> list(e.path("items"));
      case "progress" -> progress(e);
      case "button" -> button(e);
      case "link" -> link(e);
      default -> {
        // A divider, or a type this rule set does not know: nothing to check.
      }
    }
  }

  private void heading(JsonNode e) {
    text(e.get("text"), HEADING);
    JsonNode level = e.path("level");
    when(Reason.HEADING_LEVEL, !(level.isIntegralNumber() && isOneOrTwo(level)));
  }

  private void progress(JsonNode e) {
    JsonNode value = e.path("value");
    when(Reason.PROGRESS_RANGE, !(value.isNumber() && inUnitRange(value.asDouble())));
    text(e.get("label"), PROGRESS_LABEL);
  }

  private void button(JsonNode e) {
    text(e.get("label"), BUTTON_LABEL);
    String action = text(e.get("action"), RAW);
    when(Reason.BUTTON_ACTION, !Text.isActionId(action == null ? "" : action));
  }

  private void link(JsonNode e) {
    text(e.get("label"), LINK_LABEL);
    String url = text(e.get("url"), RAW);
    when(Reason.LINK, !Text.isSafeLink(url == null ? "" : url));
  }

  private void list(JsonNode items) {
    int count = items.isArray() ? items.size() : 0;
    for (int i = 0; i < count; i++) {
      item(items.get(i));
    }
    when(Reason.LIST_ITEMS, count == 0 || count > Limits.MAX_LIST_ITEMS);
  }

  private static boolean isOneOrTwo(JsonNode level) {
    return level.canConvertToLong() && (level.asLong() == 1 || level.asLong() == 2);
  }

  private static boolean inUnitRange(double v) {
    return v >= 0.0 && v <= 1.0;
  }

  private void tutorNote(JsonNode note) {
    if (!note.isTextual()) {
      return;
    }
    String text = note.asText();
    when(Reason.CONTROL_CHARS, Text.hasControl(text, true));
    // The relay turns \n into a space and trims before it counts.
    String line = Text.trim(text.replace('\n', ' '));
    when(Reason.TUTOR_NOTE_LENGTH, Text.scalars(line) > Limits.TUTOR_NOTE_MAX_CHARS);
  }
}
