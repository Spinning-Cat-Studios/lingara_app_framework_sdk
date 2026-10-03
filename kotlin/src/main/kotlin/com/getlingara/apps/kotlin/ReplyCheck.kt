package com.getlingara.apps.kotlin

import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonNull
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.doubleOrNull
import kotlinx.serialization.json.longOrNull
import java.util.EnumSet

/**
 * The rules a reply breaks, every one of them (ADR 30.9.26am D5): the contract's reference
 * validator, read rule for rule, over the generic JSON tree, so a vector, a builder-made card and a
 * developer's reply are checked by one walk. An [EnumSet] iterates in [Reason] order.
 */
internal class ReplyCheck private constructor() {
    /** One text field's rule: its limit, whether `\n` is legal in it, whether it is required. */
    private class Field(
        val limit: Int,
        val newline: Boolean,
        val required: Boolean,
    )

    private val found: EnumSet<Reason> = EnumSet.noneOf(Reason::class.java)

    private fun whenBroken(
        reason: Reason,
        broken: Boolean,
    ) {
        if (broken) found.add(reason)
    }

    /** Checks one text field; returns the string, or null when absent or not a string. */
    private fun text(
        value: JsonElement?,
        rule: Field,
    ): String? {
        val text = (value as? JsonPrimitive)?.takeIf { it.isString }?.content
        if (text == null) {
            whenBroken(Reason.EMPTY_ELEMENT, rule.required)
            return null
        }
        whenBroken(Reason.CONTROL_CHARS, Text.hasControl(text, rule.newline))
        whenBroken(Reason.TEXT_LENGTH, Text.scalars(text) > rule.limit)
        whenBroken(Reason.EMPTY_ELEMENT, rule.required && Text.trim(text).isEmpty())
        return text
    }

    private fun lang(value: JsonElement?) {
        text(value, RAW)?.let { whenBroken(Reason.LANG, !Text.isLangTag(it)) }
    }

    private fun item(item: JsonObject) {
        when (typeOf(item)) {
            "text" -> {
                text(item["text"], TEXT)
                lang(optional(item, "lang"))
            }
            "term" -> {
                text(item["word"], WORD)
                text(optional(item, "reading"), READING)
                text(optional(item, "gloss"), GLOSS)
                lang(optional(item, "lang"))
            }
        }
    }

    private fun element(e: JsonObject) {
        when (typeOf(e)) {
            "heading" -> {
                text(e["text"], HEADING)
                val level = (e["level"] as? JsonPrimitive)?.takeUnless { it.isString }?.longOrNull
                whenBroken(Reason.HEADING_LEVEL, level != 1L && level != 2L)
            }
            "text", "term" -> item(e)
            "list" -> list(e["items"] as? JsonArray ?: JsonArray(emptyList()))
            "progress" -> {
                val value = (e["value"] as? JsonPrimitive)?.takeUnless { it.isString }?.doubleOrNull
                whenBroken(Reason.PROGRESS_RANGE, value == null || value !in 0.0..1.0)
                text(e["label"], PROGRESS_LABEL)
            }
            "button" -> labelled(e, BUTTON_LABEL, Target("action", Reason.BUTTON_ACTION, Text::isActionId))
            "link" -> labelled(e, LINK_LABEL, Target("url", Reason.LINK, Text::isSafeLink))
        }
    }

    /** A button's `action` or a link's `url`: the member, its refusal and the rule it keeps. */
    private class Target(
        val key: String,
        val reason: Reason,
        val keeps: (String) -> Boolean,
    )

    private fun labelled(
        e: JsonObject,
        label: Field,
        target: Target,
    ) {
        text(e["label"], label)
        whenBroken(target.reason, !target.keeps(text(e[target.key], RAW) ?: ""))
    }

    private fun list(items: JsonArray) {
        items.forEach { (it as? JsonObject)?.let(::item) }
        whenBroken(Reason.LIST_ITEMS, items.isEmpty() || items.size > Limits.MAX_LIST_ITEMS)
    }

    private fun elements(elements: JsonArray) {
        elements.forEach { (it as? JsonObject)?.let(::element) }
        whenBroken(Reason.BUTTONS, elements.count { it is JsonObject && typeOf(it) == "button" } > Limits.MAX_BUTTONS)
        whenBroken(Reason.ELEMENTS, elements.size > Limits.MAX_ELEMENTS)
        whenBroken(Reason.EMPTY_CARD, elements.isEmpty())
    }

    private fun tutorNote(note: JsonElement?) {
        val text = (note as? JsonPrimitive)?.takeIf { it.isString }?.content ?: return
        whenBroken(Reason.CONTROL_CHARS, Text.hasControl(text, true))
        // The relay turns \n into a space and trims before it counts.
        val line = Text.trim(text.replace('\n', ' '))
        whenBroken(Reason.TUTOR_NOTE_LENGTH, Text.scalars(line) > Limits.TUTOR_NOTE_MAX_CHARS)
    }

    companion object {
        private val HEADING = Field(Limits.HEADING_MAX_CHARS, newline = false, required = true)
        private val TEXT = Field(Limits.TEXT_MAX_CHARS, newline = true, required = true)
        private val WORD = Field(Limits.TERM_WORD_MAX_CHARS, newline = false, required = true)
        private val READING = Field(Limits.TERM_READING_MAX_CHARS, newline = false, required = false)
        private val GLOSS = Field(Limits.TERM_GLOSS_MAX_CHARS, newline = false, required = false)
        private val PROGRESS_LABEL = Field(Limits.PROGRESS_LABEL_MAX_CHARS, newline = false, required = true)
        private val BUTTON_LABEL = Field(Limits.BUTTON_LABEL_MAX_CHARS, newline = false, required = true)
        private val LINK_LABEL = Field(Limits.LINK_LABEL_MAX_CHARS, newline = false, required = true)

        /** `lang`, `button.action` and `link.url`: no length limit of their own. */
        private val RAW = Field(Int.MAX_VALUE, newline = false, required = false)

        private fun typeOf(node: JsonObject): String = (node["type"] as? JsonPrimitive)?.takeIf { it.isString }?.content ?: ""

        /** A member, JSON `null` read as absent, as the reference reads optional fields. */
        private fun optional(
            parent: JsonObject,
            name: String,
        ): JsonElement? = parent[name]?.takeUnless { it is JsonNull }

        private fun elementsOf(card: JsonElement?): JsonArray =
            ((card as? JsonObject)?.get("elements") as? JsonArray) ?: JsonArray(emptyList())

        /** Every card rule and the tutor-note rule [reply] breaks; never the size rule. */
        fun reply(reply: JsonElement): EnumSet<Reason> {
            val check = ReplyCheck()
            val obj = reply as? JsonObject
            check.elements(elementsOf(obj?.get("card")))
            check.tutorNote(obj?.get("tutor_note"))
            return check.found
        }

        /** Every card rule [card], a `{elements}` object, breaks. */
        fun card(card: JsonElement): EnumSet<Reason> {
            val check = ReplyCheck()
            check.elements(elementsOf(card))
            return check.found
        }
    }
}
