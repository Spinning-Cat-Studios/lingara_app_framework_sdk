package com.getlingara.apps.kotlin

import kotlinx.serialization.json.JsonElement

/**
 * The card rules, as constants in one place, and [validateReply] (ADR 30.9.26am D5): the relay's
 * clamp restated as refusals, held by `conformance/vectors/card-limits.json` to the same answer as
 * every other kit. The relay's bounds are post-parse clamps, not schema keywords.
 */
public object Limits {
    /** The largest reply body the relay reads, in UTF-8 bytes. */
    public const val REPLY_MAX_BYTES: Int = 32768

    /** The largest request body a kit reads, in bytes; one more is `413`. */
    public const val REQUEST_MAX_BYTES: Int = 65536
    public const val MAX_ELEMENTS: Int = 24
    public const val MAX_BUTTONS: Int = 4
    public const val MAX_LIST_ITEMS: Int = 20

    /** A tutor note's limit, in characters, once `\n` is a space and the ends are trimmed. */
    public const val TUTOR_NOTE_MAX_CHARS: Int = 280

    /** A link's URL limit, in UTF-8 bytes. */
    public const val MAX_URL_BYTES: Int = 2048
    public const val HEADING_MAX_CHARS: Int = 80
    public const val TEXT_MAX_CHARS: Int = 600
    public const val TERM_WORD_MAX_CHARS: Int = 60
    public const val TERM_READING_MAX_CHARS: Int = 120
    public const val TERM_GLOSS_MAX_CHARS: Int = 160
    public const val PROGRESS_LABEL_MAX_CHARS: Int = 60
    public const val BUTTON_LABEL_MAX_CHARS: Int = 32
    public const val LINK_LABEL_MAX_CHARS: Int = 60
    public const val MANIFEST_NAME_MAX_CHARS: Int = 40
    public const val MANIFEST_DESCRIPTION_MAX_CHARS: Int = 280

    /** The manifest's stored keys' limit, in bytes of compact UTF-8 JSON. */
    public const val MANIFEST_MAX_BYTES: Int = 65536
}

/**
 * Why a reply is refused. When a reply breaks several rules, the reason is the first one here. The
 * first eleven are the relay's clamp names; the last three are the kit's own.
 */
public enum class Reason(
    public val wire: String,
) {
    CONTROL_CHARS("control_chars"),
    TEXT_LENGTH("text_length"),
    HEADING_LEVEL("heading_level"),
    PROGRESS_RANGE("progress_range"),
    LANG("lang"),
    LINK("link"),
    BUTTON_ACTION("button_action"),
    BUTTONS("buttons"),
    LIST_ITEMS("list_items"),
    EMPTY_ELEMENT("empty_element"),
    ELEMENTS("elements"),
    EMPTY_CARD("empty_card"),
    TUTOR_NOTE_LENGTH("tutor_note_length"),
    REPLY_TOO_LARGE("reply_too_large"),
}

/** A card or a reply breaks a card rule: [reason] is the first one, in the contract's order. */
public class CardLimitException internal constructor(
    public val reason: Reason,
) : RuntimeException("the reply breaks the card rule ${reason.wire}")

/**
 * Checks a whole reply, `{card, tutor_note?}`, against every card rule, then encodes it exactly as
 * the kit sends it and checks its size. Returns those bytes.
 *
 * @throws CardLimitException naming the first rule the reply breaks
 */
public fun validateReply(reply: JsonElement): ByteArray {
    val found = ReplyCheck.reply(reply)
    val encoded = AppJson.encodeToString(JsonElement.serializer(), reply).toByteArray(Charsets.UTF_8)
    if (encoded.size > Limits.REPLY_MAX_BYTES) found.add(Reason.REPLY_TOO_LARGE)
    found.firstOrNull()?.let { throw CardLimitException(it) }
    return encoded
}

/**
 * Cuts [text] the way the relay would: over [limit] characters it becomes its first `limit - 1`
 * characters and `…`; otherwise it is unchanged. Never applied implicitly.
 */
public fun truncate(
    text: String,
    limit: Int,
): String {
    require(limit >= 1) { "a limit is at least 1" }
    if (Text.scalars(text) <= limit) return text
    return text.substring(0, text.offsetByCodePoints(0, limit - 1)) + "…"
}
