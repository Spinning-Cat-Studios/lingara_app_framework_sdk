package com.getlingara.apps.kotlin

import com.getlingara.apps.kotlin.model.AppSlotName
import com.getlingara.apps.kotlin.model.ContextSliceKind
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonNull
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.put
import kotlinx.serialization.json.putJsonArray

/**
 * An app's manifest, the document `/admin/apps` accepts as an upload (ADR 30.9.26am D5), built by
 * [manifest]. The manifest is in no spec the kit generates from, so the builder is hand-written;
 * its closed values ([AppSlotName], [ContextSliceKind]) are generated.
 */
public class Manifest internal constructor(
    /** The manifest's wire form. */
    public val json: JsonObject,
) {
    /** The manifest as compact JSON, ready to upload. */
    public fun toJson(): String = AppJson.encodeToString(JsonObject.serializer(), json)
}

/** A manifest breaks one of [validateManifest]'s rules: [rule] is the first, by its closed name. */
public class ManifestException internal constructor(
    public val rule: String,
) : RuntimeException("the manifest breaks the rule $rule")

/**
 * Checks a manifest's wire form against every rule a kit can know, in the vectors' order: what only
 * the upload can know (supported locales, the rest of the URL policy, DNS, allowed scopes) is not
 * here.
 *
 * @throws ManifestException naming the first rule it breaks
 */
public fun validateManifest(manifest: JsonElement) {
    ManifestRules.firstBroken(manifest)?.let { throw ManifestException(it) }
}

/**
 * Builds a manifest and runs [validateManifest] on it:
 *
 * ```
 * val json = manifest {
 *     defaultLocale = "en"
 *     name("Daily five")
 *     description("Five words to review, picked from your plan.")
 *     renderUrl = "https://apps.example.com/lingara/render"
 *     slots(AppSlotName.HOME_PERIOD_SIDE)
 *     context(ContextSliceKind.LANGUAGES)
 * }.toJson()
 * ```
 *
 * @throws ManifestException naming the first rule it breaks
 */
public fun manifest(build: ManifestBuilder.() -> Unit): Manifest {
    val json = ManifestBuilder().apply(build).toJson()
    validateManifest(json)
    return Manifest(json)
}

/** The scope of [manifest]. `listed` and `tutorNote` default to false. */
public class ManifestBuilder internal constructor() {
    /** The locale `name` and `description` always carry. */
    public var defaultLocale: String? = null

    /** Where Lingara sends the app's signed requests: an `https` URL. */
    public var renderUrl: String? = null

    /** Whether the app may add a tutor note to its replies. */
    public var tutorNote: Boolean = false

    /** Whether the app is listed for every learner, rather than private to its owner. */
    public var listed: Boolean = false

    private var name: Any? = null
    private var description: Any? = null
    private val slots = mutableListOf<String>()
    private val context = mutableListOf<String>()
    private val scopes = mutableListOf<String>()

    /** The name in the default locale only: 1–40 characters. */
    public fun name(name: String) {
        this.name = name
    }

    /** The name per locale, containing the default locale. */
    public fun name(names: Map<String, String>) {
        this.name = names.toMap()
    }

    /** The description in the default locale only: 1–280 characters. */
    public fun description(description: String) {
        this.description = description
    }

    /** The description per locale, containing the default locale. */
    public fun description(descriptions: Map<String, String>) {
        this.description = descriptions.toMap()
    }

    /** The slots the app draws in; at least one. */
    public fun slots(vararg slots: AppSlotName) {
        slots.mapTo(this.slots) { it.value }
    }

    /** The context slices the app asks the learner to share. */
    public fun context(vararg kinds: ContextSliceKind) {
        kinds.mapTo(context) { it.value }
    }

    /**
     * The API scopes the app's client uses, listed on the learner's consent page; each must be
     * one of the client's allowed scopes, such as `lesson_plans:read`.
     */
    public fun scopes(vararg scopes: String) {
        this.scopes += scopes
    }

    internal fun toJson(): JsonObject =
        buildJsonObject {
            put("manifest_version", 1)
            defaultLocale?.let { put("default_locale", it) }
            put("name", localeMap(name))
            put("description", localeMap(description))
            put("render_url", renderUrl)
            putJsonArray("slots") { slots.forEach { add(JsonPrimitive(it)) } }
            putJsonArray("context") { context.forEach { add(JsonPrimitive(it)) } }
            putJsonArray("scopes") { scopes.forEach { add(JsonPrimitive(it)) } }
            put("tutor_note", tutorNote)
            put("listed", listed)
        }

    /** A bare string is sugar for `{default_locale: s}`. */
    private fun localeMap(value: Any?): JsonElement =
        when (value) {
            is String -> buildJsonObject { put(defaultLocale ?: "", value) }
            is Map<*, *> -> buildJsonObject { value.forEach { (k, v) -> put(k.toString(), v.toString()) } }
            else -> JsonNull
        }
}
