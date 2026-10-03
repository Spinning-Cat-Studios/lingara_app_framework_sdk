package com.getlingara.apps;

import com.fasterxml.jackson.core.JsonProcessingException;
import com.fasterxml.jackson.databind.JsonNode;
import java.util.EnumSet;
import java.util.Optional;

/**
 * The card rules, as constants in one place, and {@link #validateReply} (ADR 30.9.26am D5): the
 * relay's clamp restated as refusals, held by {@code conformance/vectors/card-limits.json} to the
 * same answer as every other kit. The relay's bounds are post-parse clamps, not schema keywords, so
 * no generated type carries them.
 */
public final class Limits {
  /** The largest reply body the relay reads, in UTF-8 bytes. */
  public static final int REPLY_MAX_BYTES = 32768;

  /** The largest request body a kit reads, in bytes; one more is {@code 413}. */
  public static final int REQUEST_MAX_BYTES = 65536;

  /** At most this many elements in a card. */
  public static final int MAX_ELEMENTS = 24;

  /** At most this many buttons in a card. */
  public static final int MAX_BUTTONS = 4;

  /** At most this many items in a list (and at least one). */
  public static final int MAX_LIST_ITEMS = 20;

  /** A tutor note's limit, in characters, once {@code \n} is a space and the ends are trimmed. */
  public static final int TUTOR_NOTE_MAX_CHARS = 280;

  /** A link's URL limit, in UTF-8 bytes. */
  public static final int MAX_URL_BYTES = 2048;

  /** A heading's text limit, in characters. */
  public static final int HEADING_MAX_CHARS = 80;

  /** A text element's or text item's limit, in characters. */
  public static final int TEXT_MAX_CHARS = 600;

  /** A term's word limit, in characters. */
  public static final int TERM_WORD_MAX_CHARS = 60;

  /** A term's reading limit, in characters. */
  public static final int TERM_READING_MAX_CHARS = 120;

  /** A term's gloss limit, in characters. */
  public static final int TERM_GLOSS_MAX_CHARS = 160;

  /** A progress label's limit, in characters. */
  public static final int PROGRESS_LABEL_MAX_CHARS = 60;

  /** A button label's limit, in characters. */
  public static final int BUTTON_LABEL_MAX_CHARS = 32;

  /** A link label's limit, in characters. */
  public static final int LINK_LABEL_MAX_CHARS = 60;

  /** A manifest name's limit, in characters, after trimming. */
  public static final int MANIFEST_NAME_MAX_CHARS = 40;

  /** A manifest description's limit, in characters, after trimming. */
  public static final int MANIFEST_DESCRIPTION_MAX_CHARS = 280;

  /** The manifest's stored keys' limit, in bytes of compact UTF-8 JSON. */
  public static final int MANIFEST_MAX_BYTES = 65536;

  /**
   * Why a reply is refused. When a reply breaks several rules, the reason is the first one here.
   * The first eleven are the relay's clamp names; the last three are the kit's own.
   */
  public enum Reason {
    /**
     * A control, bidi override or bidi isolate character, or a {@code \n} where none is allowed.
     */
    CONTROL_CHARS("control_chars"),
    /** A string over its limit. */
    TEXT_LENGTH("text_length"),
    /** A heading level other than 1 or 2. */
    HEADING_LEVEL("heading_level"),
    /** A progress value outside 0–1. */
    PROGRESS_RANGE("progress_range"),
    /** A {@code lang} that is not a language tag. */
    LANG("lang"),
    /** A link URL the relay would drop. */
    LINK("link"),
    /** A button action outside {@code [A-Za-z0-9_.:-]{1,64}}. */
    BUTTON_ACTION("button_action"),
    /** More than four buttons. */
    BUTTONS("buttons"),
    /** An empty list, or one over twenty items. */
    LIST_ITEMS("list_items"),
    /** A required string that is empty after trimming. */
    EMPTY_ELEMENT("empty_element"),
    /** More than twenty-four elements. */
    ELEMENTS("elements"),
    /** No element at all. */
    EMPTY_CARD("empty_card"),
    /** A tutor note over 280 characters. */
    TUTOR_NOTE_LENGTH("tutor_note_length"),
    /** A reply over 32 768 bytes, encoded as sent. */
    REPLY_TOO_LARGE("reply_too_large");

    private final String wire;

    Reason(String wire) {
      this.wire = wire;
    }

    /**
     * The contract's spelling.
     *
     * @return the reason's name in the contract and the vectors
     */
    public String wire() {
      return wire;
    }
  }

  private Limits() {}

  /**
   * Checks a whole reply, {@code {card, tutor_note?}}, against every card rule, then encodes it
   * exactly as the kit sends it and checks its size.
   *
   * @param reply the reply as a JSON tree
   * @return the encoded reply: the bytes to send, never more than {@link #REPLY_MAX_BYTES}
   * @throws CardLimitException naming the first rule the reply breaks
   */
  public static byte[] validateReply(JsonNode reply) {
    EnumSet<Reason> found = ReplyCheck.reply(reply);
    byte[] encoded;
    try {
      encoded = Json.MAPPER.writeValueAsBytes(reply);
    } catch (JsonProcessingException e) {
      throw new IllegalStateException("a JSON tree always encodes", e);
    }
    if (encoded.length > REPLY_MAX_BYTES) {
      found.add(Reason.REPLY_TOO_LARGE);
    }
    first(found).ifPresent(Limits::refuse);
    return encoded;
  }

  /**
   * Cuts {@code text} the way the relay would: over {@code limit} characters, it becomes its first
   * {@code limit - 1} characters and {@code …}; otherwise it is unchanged. Never applied
   * implicitly.
   *
   * @param text the text
   * @param limit the most characters the result may have, at least 1
   * @return the text, cut to the limit
   */
  public static String truncate(String text, int limit) {
    if (limit < 1) {
      throw new IllegalArgumentException("a limit is at least 1");
    }
    if (Text.scalars(text) <= limit) {
      return text;
    }
    return text.substring(0, text.offsetByCodePoints(0, limit - 1)) + "…";
  }

  /** The first card rule (reasons 1–12) {@code card} breaks, if any. */
  static Optional<Reason> cardReason(JsonNode card) {
    return first(ReplyCheck.card(card));
  }

  static Optional<Reason> first(EnumSet<Reason> found) {
    return found.stream().findFirst();
  }

  static void refuse(Reason reason) {
    throw new CardLimitException(reason);
  }
}
