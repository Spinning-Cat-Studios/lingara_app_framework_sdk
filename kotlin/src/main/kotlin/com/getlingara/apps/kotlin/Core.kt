package com.getlingara.apps.kotlin

import com.getlingara.apps.kotlin.model.AppActionRequest
import com.getlingara.apps.kotlin.model.AppOperation
import com.getlingara.apps.kotlin.model.AppRenderRequest
import com.getlingara.apps.kotlin.model.ContextSliceKind
import com.getlingara.kotlin.events.Webhook
import com.getlingara.kotlin.events.WebhookVerificationException
import kotlinx.coroutines.CancellationException
import kotlinx.serialization.DeserializationStrategy
import kotlinx.serialization.SerializationException
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import java.nio.ByteBuffer
import java.nio.charset.CharacterCodingException

/**
 * The handler core (ADR 30.9.26am D4): verify, decode, dispatch, validate, encode. It is
 * framework-neutral; `Route.lingaraApp` is a few lines over it.
 */
internal class Core(
    private val webhook: Webhook,
    private val render: RenderFunction,
    private val actions: Map<String, ActionFunction>,
) {
    suspend fun handle(
        method: String,
        headers: Map<String, List<String>>,
        readBody: suspend (Int) -> ByteArray,
    ): AppResponse {
        if (!method.equals("POST", ignoreCase = true)) return AppResponse.empty(405)
        val body = readBody(Limits.REQUEST_MAX_BYTES + 1)
        if (body.size > Limits.REQUEST_MAX_BYTES) return AppResponse.empty(413)
        try {
            webhook.verifySignature(body, headers)
        } catch (e: WebhookVerificationException) {
            return AppResponse.empty(401)
        }
        return decode(body)?.let { dispatch(it) } ?: AppResponse.error(400, "bad_request")
    }

    /** The answer to a decoded request, or null when it is a bad request. */
    private suspend fun dispatch(request: JsonObject): AppResponse? {
        val type = (request["type"] as? JsonPrimitive)?.takeIf { it.isString }?.content ?: return null
        val op = AppOperation.fromValue(type) ?: return null
        // Exhaustive over the generated operations: a third one fails compilation here.
        return when (op) {
            AppOperation.APP_RENDER -> {
                val r = read(AppRenderRequest.serializer(), request) ?: return null
                reply(op, r.installId) { render(r) }
            }
            AppOperation.APP_ACTION -> {
                val r = read(AppActionRequest.serializer(), request) ?: return null
                val action = actions[r.actionId] ?: return null
                reply(op, r.installId) { action(r) }
            }
        }
    }

    private suspend fun reply(
        op: AppOperation,
        installId: String,
        call: suspend () -> Reply,
    ): AppResponse =
        try {
            AppResponse(200, validateReply(call().toTree()))
        } catch (e: CancellationException) {
            throw e
        } catch (e: CardLimitException) {
            failed(op, installId, e.reason.wire)
        } catch (e: Exception) {
            failed(op, installId, e.javaClass.name)
        }

    private companion object {
        val LOG: System.Logger = System.getLogger("com.getlingara.apps.kotlin")

        /** The body as an object, every context slice of an unknown kind skipped; null if it is not one. */
        fun decode(body: ByteArray): JsonObject? {
            val tree =
                try {
                    val text =
                        Charsets.UTF_8
                            .newDecoder()
                            .decode(ByteBuffer.wrap(body))
                            .toString()
                    AppJson.parseToJsonElement(text)
                } catch (e: CharacterCodingException) {
                    return null
                } catch (e: SerializationException) {
                    return null
                }
            val request = tree as? JsonObject ?: return null
            val context = request["context"] as? JsonArray ?: return request
            return JsonObject(request + ("context" to JsonArray(context.filter(::isKnownSlice))))
        }

        fun isKnownSlice(slice: JsonElement): Boolean {
            val kind = ((slice as? JsonObject)?.get("kind") as? JsonPrimitive)?.content ?: ""
            val known = ContextSliceKind.fromValue(kind) != null
            if (!known) LOG.log(System.Logger.Level.INFO, "skipped a context slice of unknown kind {0}", kind)
            return known
        }

        fun <T> read(
            strategy: DeserializationStrategy<T>,
            request: JsonObject,
        ): T? =
            try {
                AppJson.decodeFromJsonElement(strategy, request)
            } catch (e: SerializationException) {
                null
            } catch (e: IllegalArgumentException) {
                null
            }

        /** Logs the operation, the install id and the reason; never the body, a secret or a signature. */
        fun failed(
            op: AppOperation,
            installId: String,
            reason: String,
        ): AppResponse {
            LOG.log(System.Logger.Level.WARNING, "{0} failed for install {1}: {2}", op.value, installId, reason)
            return AppResponse.error(500, "handler_failed")
        }
    }
}
