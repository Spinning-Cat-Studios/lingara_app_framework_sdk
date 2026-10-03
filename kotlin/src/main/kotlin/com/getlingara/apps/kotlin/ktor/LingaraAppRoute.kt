package com.getlingara.apps.kotlin.ktor

import com.getlingara.apps.kotlin.AppResponse
import com.getlingara.apps.kotlin.LingaraApp
import io.ktor.http.ContentType
import io.ktor.http.HttpStatusCode
import io.ktor.server.request.httpMethod
import io.ktor.server.request.receiveChannel
import io.ktor.server.response.respondBytes
import io.ktor.server.routing.Route
import io.ktor.utils.io.ByteReadChannel
import io.ktor.utils.io.readAvailable
import java.io.ByteArrayOutputStream

/**
 * Mounts [app] on this route for every method (ADR 30.9.26am D2): `routing { route("/lingara/render")
 * { lingaraApp(app) } }`. It reads the raw body as bytes, at most 64 KiB + 1 of them, so Ktor's
 * content negotiation never parses it. Ktor 3 is the caller's: the kit declares it `compileOnly`.
 */
public fun Route.lingaraApp(app: LingaraApp) {
    handle {
        val headers =
            call.request.headers
                .entries()
                .associate { (name, values) -> name to values }
        val response =
            app.handle(call.request.httpMethod.value, headers) { limit -> call.receiveChannel().readAtMost(limit) }
        val type = if (response.body.isEmpty()) null else ContentType.parse(AppResponse.JSON)
        call.respondBytes(response.body, type, HttpStatusCode.fromValue(response.status))
    }
}

/** At most [limit] bytes, so an oversized body is refused without being buffered whole. */
private suspend fun ByteReadChannel.readAtMost(limit: Int): ByteArray {
    val out = ByteArrayOutputStream()
    val chunk = ByteArray(CHUNK)
    while (out.size() < limit) {
        val n = readAvailable(chunk, 0, minOf(CHUNK, limit - out.size()))
        if (n < 0) break
        out.write(chunk, 0, n)
    }
    return out.toByteArray()
}

private const val CHUNK = 8192
