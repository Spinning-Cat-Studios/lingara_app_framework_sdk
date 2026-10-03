package com.getlingara.apps.kotlin

import com.getlingara.apps.kotlin.model.AppSlotName
import com.getlingara.apps.kotlin.model.ContextSliceKind
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.longOrNull

/**
 * [validateManifest]'s rules, in `conformance/vectors/manifest.json`'s order (ADR 30.9.26am D5):
 * the first broken rule names the refusal.
 */
internal object ManifestRules {
    private val STORED = listOf("default_locale", "name", "description", "slots", "context", "scopes", "tutor_note")

    fun firstBroken(manifest: JsonElement): String? {
        val m = manifest as? JsonObject ?: return "manifest_version"
        val rules: List<Pair<String, (JsonObject) -> Boolean>> =
            listOf(
                "manifest_version" to ::badVersion,
                "default_locale" to ::badDefaultLocale,
                "name" to { !localeMap(it["name"], Limits.MANIFEST_NAME_MAX_CHARS) },
                "description" to { !localeMap(it["description"], Limits.MANIFEST_DESCRIPTION_MAX_CHARS) },
                "render_url" to { !isHttpsUrl(it["render_url"]) },
                "slots" to ::badSlots,
                "context" to ::badContext,
                "duplicate" to ::hasDuplicate,
                "too_large" to ::tooLarge,
            )
        return rules.firstOrNull { (_, broken) -> broken(m) }?.first
    }

    private fun string(value: JsonElement?): String? = (value as? JsonPrimitive)?.takeIf { it.isString }?.content

    private fun badVersion(m: JsonObject): Boolean {
        val v = m["manifest_version"] as? JsonPrimitive
        return v == null || v.isString || v.longOrNull != 1L
    }

    private fun badDefaultLocale(m: JsonObject): Boolean {
        val locale = string(m["default_locale"])
        if (locale.isNullOrEmpty()) return true
        return listOf("name", "description").any { (m[it] as? JsonObject)?.containsKey(locale) == false }
    }

    /** A non-empty object whose every value is a clean string of 1–[max] characters, trimmed. */
    private fun localeMap(
        map: JsonElement?,
        max: Int,
    ): Boolean {
        if (map !is JsonObject || map.isEmpty()) return false
        return map.values.all { v -> string(v)?.let { cleanLabel(Text.trim(it), max) } ?: false }
    }

    private fun cleanLabel(
        trimmed: String,
        max: Int,
    ): Boolean = Text.scalars(trimmed) in 1..max && !Text.hasControl(trimmed, false) && !Text.hasZeroWidth(trimmed)

    /** `https://` (any case) and at least one character other than `/ ? #`. */
    private fun isHttpsUrl(url: JsonElement?): Boolean {
        val s = string(url) ?: return false
        return s.length > 8 && s.substring(0, 8).equals("https://", ignoreCase = true) && s[8] !in "/?#"
    }

    private fun badSlots(m: JsonObject): Boolean {
        val slots = m["slots"] as? JsonArray ?: return true
        return slots.isEmpty() || !slots.all { v -> AppSlotName.entries.any { it.value == string(v) } }
    }

    private fun badContext(m: JsonObject): Boolean {
        val context = m["context"] ?: return false
        if (context !is JsonArray) return true
        return !context.all { v -> string(v)?.let { ContextSliceKind.fromValue(it) } != null }
    }

    private fun hasDuplicate(m: JsonObject): Boolean =
        listOf("slots", "context", "scopes").any { key ->
            val values = m[key] as? JsonArray ?: return@any false
            values.toSet().size != values.size
        }

    /** The seven stored keys, `context` and `scopes` defaulting to [] and the note to false. */
    private fun tooLarge(m: JsonObject): Boolean {
        val stored =
            buildJsonObject {
                for (key in STORED) {
                    put(key, m[key] ?: if (key == "tutor_note") JsonPrimitive(false) else JsonArray(emptyList()))
                }
            }
        val bytes = AppJson.encodeToString(JsonObject.serializer(), stored).toByteArray(Charsets.UTF_8)
        return bytes.size > Limits.MANIFEST_MAX_BYTES
    }
}
