package com.getlingara.apps.kotlin

import kotlinx.serialization.json.Json

/**
 * The kit's one `Json` (ADR 30.9.26am D4, D6). Lenient on the way in: an unknown key is ignored
 * wherever it appears, whatever the view's `additionalProperties` says, so an older kit still
 * answers a newer relay. Exact on the way out: compact, raw UTF-8, `/` unescaped (the defaults),
 * and an absent optional member omitted rather than written as `null`.
 */
internal val AppJson: Json =
    Json {
        ignoreUnknownKeys = true
        explicitNulls = false
        encodeDefaults = false
    }
