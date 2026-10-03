package com.getlingara.apps.kotlin

import com.getlingara.apps.kotlin.model.ButtonStyle
import com.getlingara.apps.kotlin.model.CardElement
import com.getlingara.apps.kotlin.model.CardElementButton
import com.getlingara.apps.kotlin.model.CardElementDivider
import com.getlingara.apps.kotlin.model.CardElementHeading
import com.getlingara.apps.kotlin.model.CardElementLink
import com.getlingara.apps.kotlin.model.CardElementList
import com.getlingara.apps.kotlin.model.CardElementProgress
import com.getlingara.apps.kotlin.model.CardElementTerm
import com.getlingara.apps.kotlin.model.CardElementText
import com.getlingara.apps.kotlin.model.ListItem
import com.getlingara.apps.kotlin.model.ListItemTerm
import com.getlingara.apps.kotlin.model.ListItemText

/** List items, for [CardBuilder.list]: `item.text("one")`, `item.term("二", reading = "èr")`. */
public object Item {
    /** A text item: 1–600 characters, `\n` allowed. */
    public fun text(
        text: String,
        lang: String? = null,
    ): ListItem = ListItemText(text, lang)

    /** A term item: the word 1–60 characters, the reading at most 120, the gloss at most 160. */
    public fun term(
        word: String,
        reading: String? = null,
        gloss: String? = null,
        lang: String? = null,
    ): ListItem = ListItemTerm(word, reading, gloss, lang)
}

/** The scope of [card]: one method per element, each with its arm's fields. */
@CardDsl
public class CardBuilder internal constructor() {
    private val elements = mutableListOf<CardElement>()

    /** The list-item factory, so a list reads `list(item.text("one"), item.term("二"))`. */
    public val item: Item = Item

    internal fun elements(): List<CardElement> = elements.toList()

    /** A heading: 1–80 characters, [level] 1 or 2. */
    public fun heading(
        text: String,
        level: Int,
    ) {
        elements += CardElementHeading(text, level)
    }

    /** A text element: 1–600 characters, `\n` allowed. */
    public fun text(
        text: String,
        lang: String? = null,
    ) {
        elements += CardElementText(text, lang)
    }

    /** A term: the word 1–60 characters, the reading at most 120, the gloss at most 160. */
    public fun term(
        word: String,
        reading: String? = null,
        gloss: String? = null,
        lang: String? = null,
    ) {
        elements += CardElementTerm(word, reading, gloss, lang)
    }

    /** A list of 1–20 items. */
    public fun list(vararg items: ListItem) {
        elements += CardElementList(items.toList())
    }

    /** A list of 1–20 items. */
    public fun list(items: List<ListItem>) {
        elements += CardElementList(items.toList())
    }

    /** A progress bar: [value] 0 to 1 inclusive, [label] 1–60 characters. */
    public fun progress(
        value: Double,
        label: String,
    ) {
        elements += CardElementProgress(value, label)
    }

    /** A divider. */
    public fun divider() {
        elements += CardElementDivider
    }

    /** A button: [label] 1–32 characters; [action] comes back as the request's `action_id`. */
    public fun button(
        label: String,
        action: String,
        style: ButtonStyle? = null,
    ) {
        elements += CardElementButton(label, action, style)
    }

    /** A link: `https`, no user information, a host name (never an IP address), ≤ 2 048 bytes. */
    public fun link(
        label: String,
        url: String,
    ) {
        elements += CardElementLink(label, url)
    }
}

/** Keeps an outer builder's methods out of a nested builder's scope. */
@DslMarker
public annotation class CardDsl
