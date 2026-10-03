package com.getlingara.apps.kotlin

import com.getlingara.apps.kotlin.model.AppActionRequest
import com.getlingara.apps.kotlin.model.AppRenderRequest
import com.getlingara.kotlin.events.Webhook
import java.time.Clock

/** A render function: a card for the slot the request names. */
public typealias RenderFunction = suspend (request: AppRenderRequest) -> Reply

/** An action function: the card after the learner pressed a button. */
public typealias ActionFunction = suspend (request: AppActionRequest) -> Reply

/**
 * A Lingara app (ADR 30.9.26am D4), built by [lingaraApp]: one render function, zero or more
 * action functions by id, and the app's signing secrets.
 *
 * [handle] is the framework-neutral core every adapter calls: `405` for anything but `POST` before
 * the body is read, `413` over 64 KiB, the Lingara library's `Webhook.verifySignature` (`401`,
 * empty), lenient decoding (`400`), dispatch, [validateReply], and `500 {"error":"handler_failed"}`
 * for a raised function or a refused reply. The kit never reads `user-agent` and enforces no time
 * limit.
 */
public class LingaraApp internal constructor(
    private val core: Core,
) {
    /**
     * Answers one request. [readBody] is called at most once, only for a `POST`, with the most bytes
     * the kit reads (64 KiB + 1); it returns at most that many.
     */
    public suspend fun handle(
        method: String,
        headers: Map<String, List<String>>,
        readBody: suspend (limit: Int) -> ByteArray,
    ): AppResponse = core.handle(method, headers, readBody)
}

/** What [LingaraApp.handle] answers. A non-empty body is always `application/json`. */
public class AppResponse internal constructor(
    public val status: Int,
    public val body: ByteArray,
) {
    /** The body's content type, or null when there is no body. */
    public val contentType: String? get() = if (body.isEmpty()) null else JSON

    public companion object {
        /** The content type every response with a body carries. */
        public const val JSON: String = "application/json"

        internal fun empty(status: Int): AppResponse = AppResponse(status, ByteArray(0))

        internal fun error(
            status: Int,
            error: String,
        ): AppResponse = AppResponse(status, "{\"error\":\"$error\"}".toByteArray(Charsets.UTF_8))
    }
}

/**
 * Builds an app. Its verifier is built here, so a malformed secret fails now, not on a request:
 *
 * ```
 * val app = lingaraApp {
 *     secrets(System.getenv("LINGARA_APP_SECRET"))
 *     render { request -> card { heading("Today", 1) } }
 *     action("next") { request -> card { text(request.cardEtag) } }
 * }
 * ```
 *
 * @throws IllegalArgumentException for no secret or a malformed one
 * @throws IllegalStateException for no render function
 */
public fun lingaraApp(build: LingaraAppBuilder.() -> Unit): LingaraApp = LingaraAppBuilder().apply(build).build()

/** The scope of [lingaraApp]. */
public class LingaraAppBuilder internal constructor() {
    private var secrets: List<String> = emptyList()
    private var render: RenderFunction? = null
    private val actions = linkedMapOf<String, ActionFunction>()

    /** The clock a request's timestamp is checked against; the system clock by default. */
    public var clock: Clock = Clock.systemUTC()

    /** The app's signing secrets (`lgr_whsec_…`): one, or two during a rotation. */
    public fun secrets(vararg secrets: String) {
        this.secrets = secrets.toList()
    }

    /** The render function. */
    public fun render(render: RenderFunction) {
        this.render = render
    }

    /** The function for a button's `action`, which comes back as the request's `action_id`. */
    public fun action(
        actionId: String,
        action: ActionFunction,
    ) {
        actions[actionId] = action
    }

    internal fun build(): LingaraApp {
        val render = checkNotNull(render) { "a LingaraApp needs a render function" }
        val webhook = Webhook(*secrets.toTypedArray(), clock = clock)
        return LingaraApp(Core(webhook, render, actions.toMap()))
    }
}
