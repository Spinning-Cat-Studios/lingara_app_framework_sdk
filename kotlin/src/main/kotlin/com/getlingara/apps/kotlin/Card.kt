package com.getlingara.apps.kotlin

import com.getlingara.apps.kotlin.model.CardElement
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.put
import com.getlingara.apps.kotlin.model.Card as CardModel

/**
 * What a render or action function returns (ADR 30.9.26am D4): always a card, alone ([Card]) or
 * with a tutor note ([CardReply]). There is no `unchanged`, `fallback` or `etag`: those are the
 * relay's words to its clients, never an app's.
 */
public sealed interface Reply

/**
 * A card that keeps every card rule (ADR 30.9.26am D5), built by [card]:
 *
 * ```
 * val c = card {
 *     heading("Today", 1)
 *     term("雨", reading = "yǔ", gloss = "rain", lang = "zh")
 *     list(item.text("one"), item.term("二"))
 *     button("Next", "next")
 * }
 * ```
 */
public class Card internal constructor(
    /** The card's elements, in order. */
    public val elements: List<CardElement>,
) : Reply {
    /** The generated card model. */
    public val model: CardModel get() = CardModel(elements)

    /** `{elements: [...]}` as the kit sends it. */
    internal fun toTree(): JsonObject = AppJson.encodeToJsonElement(CardModel.serializer(), model).jsonObject
}

/** A card and, optionally, a tutor note: `reply(card).tutorNote("…")`. The note is plain text. */
public class CardReply internal constructor(
    public val card: Card,
    public val tutorNote: String?,
) : Reply {
    /**
     * This reply with a tutor note, at most 280 characters once `\n` is a space and the ends are
     * trimmed. A `\n` is allowed, since the relay makes it a space.
     *
     * @throws CardLimitException `control_chars` or `tutor_note_length`
     */
    public fun tutorNote(text: String): CardReply {
        val withNote = CardReply(card, text)
        ReplyCheck.reply(withNote.toTree()).firstOrNull()?.let { throw CardLimitException(it) }
        return withNote
    }

    /** `{card, tutor_note?}`, the note omitted when absent. */
    internal fun toTree(): JsonObject =
        buildJsonObject {
            put("card", card.toTree())
            tutorNote?.let { put("tutor_note", it) }
        }
}

/** A reply carrying [card], to which a tutor note can be added. */
public fun reply(card: Card): CardReply = CardReply(card, null)

/**
 * Builds a card. Each element method takes its generated arm's closed field set; the card is
 * refused when it would be clamped, never cut, stripped or dropped.
 *
 * @throws CardLimitException naming the first rule the card breaks
 */
public fun card(build: CardBuilder.() -> Unit): Card {
    val card = Card(CardBuilder().apply(build).elements())
    ReplyCheck.card(card.toTree()).firstOrNull()?.let { throw CardLimitException(it) }
    return card
}

/** `{card, tutor_note?}` for any reply. */
internal fun Reply.toTree(): JsonObject =
    when (this) {
        is Card -> reply(this).toTree()
        is CardReply -> toTree()
    }
