package com.getlingara.apps.kotlin.conformance

import com.getlingara.apps.kotlin.Reply
import com.getlingara.apps.kotlin.card
import com.getlingara.apps.kotlin.ktor.lingaraApp
import com.getlingara.apps.kotlin.lingaraApp
import com.getlingara.apps.kotlin.model.AppRenderRequest
import com.getlingara.apps.kotlin.model.AppSlotName
import com.getlingara.apps.kotlin.model.ContextSliceKind
import com.getlingara.apps.kotlin.reply
import io.ktor.server.cio.CIO
import io.ktor.server.engine.embeddedServer
import io.ktor.server.routing.route
import io.ktor.server.routing.routing
import kotlinx.coroutines.runBlocking

/**
 * The contract's fixture app (§F) on the Kotlin kit: every reply is exact, because the host judges
 * them JSON-equal. It reads its secrets from `LINGARA_APPS_CONFORMANCE_SECRETS` and its port from
 * `LINGARA_APPS_CONFORMANCE_PORT`, listens on 127.0.0.1 and prints `listening <port>` first.
 */
fun main() {
    val secrets = System.getenv("LINGARA_APPS_CONFORMANCE_SECRETS").split(",")
    val port = System.getenv("LINGARA_APPS_CONFORMANCE_PORT")?.toInt() ?: 0
    val app =
        lingaraApp {
            secrets(*secrets.toTypedArray())
            render(::render)
            action("inc") { r ->
                card {
                    progress(0.5, "inc")
                    text(r.cardEtag)
                }
            }
            action("boom") { error("boom") }
            // The public builder refuses 21 items, so the action raises.
            action("overflow") { card { list((1..21).map { item.text("$it") }) } }
            // Every card rule passes; the encoded reply is over 32 KiB.
            action("huge") { card { repeat(24) { text("漢".repeat(600)) } } }
        }
    val server = embeddedServer(CIO, port = port, host = "127.0.0.1") { routing { route("{...}") { lingaraApp(app) } } }
    server.start(wait = false)
    val bound =
        runBlocking {
            server.engine
                .resolvedConnectors()
                .first()
                .port
        }
    println("listening $bound")
    System.out.flush()
    Thread.currentThread().join()
}

private fun render(request: AppRenderRequest): Reply {
    val c =
        card {
            heading(request.slot.value, 1)
            request.context.forEach { text(ContextSliceKind.of(it).value) }
        }
    return if (request.slot == AppSlotName.HOME_PERIOD_SIDE) reply(c).tutorNote("fixture note") else c
}
