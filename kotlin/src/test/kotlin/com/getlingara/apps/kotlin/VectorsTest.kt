package com.getlingara.apps.kotlin

import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import java.io.File
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith

class VectorsTest {
    private fun vectors(property: String): List<JsonObject> =
        Json
            .parseToJsonElement(File(System.getProperty(property)).readText())
            .jsonObject["vectors"]!!
            .jsonArray
            .map { it.jsonObject }

    private fun expected(expect: JsonElement): String {
        val e = expect.jsonObject
        return if (e["ok"]?.jsonPrimitive?.content == "true") "ok" else e["refused"]!!.jsonPrimitive.content
    }

    private fun cardAnswer(reply: JsonElement): String =
        try {
            validateReply(reply)
            "ok"
        } catch (e: CardLimitException) {
            e.reason.wire
        }

    private fun manifestAnswer(manifest: JsonElement): String =
        try {
            validateManifest(manifest)
            "ok"
        } catch (e: ManifestException) {
            e.rule
        }

    /**
     * 30.9.26am AC5: every card-limits.json vector's expect equals validateReply's answer, every
     * manifest.json vector's equals validateManifest's, and truncate of an 81-scalar astral heading
     * at 80 is 79 scalars plus an ellipsis.
     */
    @Test
    fun everyCardAndManifestVectorGivesItsExpectedAnswer() {
        for (v in vectors("lingara.cardVectors")) {
            assertEquals(expected(v["expect"]!!), cardAnswer(v["reply"]!!), v["name"].toString())
        }
        for (v in vectors("lingara.manifestVectors")) {
            assertEquals(expected(v["expect"]!!), manifestAnswer(v["manifest"]!!), v["name"].toString())
        }
        val cut = truncate("𝄞".repeat(81), 80)
        assertEquals(80, cut.codePointCount(0, cut.length))
        assertEquals("𝄞".repeat(79) + "…", cut)
        assertEquals("𝄞".repeat(80), truncate("𝄞".repeat(80), 80))
    }

    @Test
    fun theBuildersRefuseWhatTheRelayWouldClamp() {
        val overflow = assertFailsWith<CardLimitException> { card { list((1..21).map { item.text("$it") }) } }
        assertEquals(Reason.LIST_ITEMS, overflow.reason)
        val huge = card { repeat(24) { text("漢".repeat(600)) } }
        val tooLarge = assertFailsWith<CardLimitException> { validateReply(huge.toTree().let { JsonObject(mapOf("card" to it)) }) }
        assertEquals(Reason.REPLY_TOO_LARGE, tooLarge.reason)
        val note = assertFailsWith<CardLimitException> { reply(card { divider() }).tutorNote("a".repeat(281)) }
        assertEquals(Reason.TUTOR_NOTE_LENGTH, note.reason)
        assertEquals(Reason.LINK, assertFailsWith<CardLimitException> { card { link("Why", "https://127.0.0.1/") } }.reason)
    }

    @Test
    fun aBuilderMadeCardOmitsItsAbsentMembers() {
        val c =
            card {
                heading("Today", 1)
                term("雨", reading = "yǔ", gloss = "rain", lang = "zh")
                list(item.text("one"), item.term("二"))
                button("Next", "next")
                divider()
            }
        assertEquals(
            "{\"elements\":[{\"type\":\"heading\",\"text\":\"Today\",\"level\":1}," +
                "{\"type\":\"term\",\"word\":\"雨\",\"reading\":\"yǔ\",\"gloss\":\"rain\",\"lang\":\"zh\"}," +
                "{\"type\":\"list\",\"items\":[{\"type\":\"text\",\"text\":\"one\"},{\"type\":\"term\",\"word\":\"二\"}]}," +
                "{\"type\":\"button\",\"label\":\"Next\",\"action\":\"next\"},{\"type\":\"divider\"}]}",
            c.toTree().toString(),
        )
    }

    @Test
    fun theManifestBuilderWritesTheWireForm() {
        val json =
            manifest {
                defaultLocale = "en"
                name("Daily five")
                description("Five words to review.")
                renderUrl = "https://apps.example.com/lingara/render"
                slots(com.getlingara.apps.kotlin.model.AppSlotName.HOME_PERIOD_SIDE)
                context(com.getlingara.apps.kotlin.model.ContextSliceKind.LANGUAGES)
            }.toJson()
        assertEquals(
            "{\"manifest_version\":1,\"default_locale\":\"en\",\"name\":{\"en\":\"Daily five\"}," +
                "\"description\":{\"en\":\"Five words to review.\"}," +
                "\"render_url\":\"https://apps.example.com/lingara/render\"," +
                "\"slots\":[\"home.side\"],\"context\":[\"languages\"],\"scopes\":[]," +
                "\"tutor_note\":false,\"listed\":false}",
            json,
        )
        val e = assertFailsWith<ManifestException> { manifest { defaultLocale = "en" } }
        assertEquals("name", e.rule)
    }
}
