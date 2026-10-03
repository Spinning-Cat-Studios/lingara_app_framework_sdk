package com.getlingara.apps.kotlin.ktor

import com.getlingara.apps.kotlin.card
import com.getlingara.apps.kotlin.lingaraApp
import com.getlingara.apps.kotlin.model.AppActionRequest
import com.getlingara.apps.kotlin.model.AppRenderRequest
import com.getlingara.apps.kotlin.model.AppSlotName
import com.getlingara.apps.kotlin.model.ContextSliceLanguages
import com.getlingara.apps.kotlin.model.ContextSlicePlanSummary
import io.ktor.client.request.get
import io.ktor.client.request.header
import io.ktor.client.request.post
import io.ktor.client.request.setBody
import io.ktor.client.statement.HttpResponse
import io.ktor.client.statement.bodyAsText
import io.ktor.server.routing.route
import io.ktor.server.routing.routing
import io.ktor.server.testing.ApplicationTestBuilder
import io.ktor.server.testing.testApplication
import java.util.Base64
import javax.crypto.Mac
import javax.crypto.spec.SecretKeySpec
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertIs
import kotlin.test.assertNull

class KtorRouteTest {
    private val key = "0123456789abcdef0123456789abcdef".toByteArray()
    private val secret = "lgr_whsec_" + Base64.getEncoder().encodeToString(key)
    private val id = "lgr_msg_00000000000000000000000000000000"
    private val etag = "c1_8f14e45fceea167a5a36dedd4bea2543"
    private var rendered: AppRenderRequest? = null
    private var acted: AppActionRequest? = null

    private val app =
        lingaraApp {
            secrets(secret)
            render { r ->
                rendered = r
                card { heading(r.slot.value, 1) }
            }
            action("inc") { r ->
                acted = r
                card { text(r.cardEtag) }
            }
        }

    private val render =
        "{\"type\":\"app.render\",\"id\":\"$id\",\"install_id\":\"7c1d2e3f-4a5b-4c6d-8e7f-9a0b1c2d3e4f\"," +
            "\"subject\":\"lgr_sub_x\",\"slot\":\"home.side\",\"locale\":\"ja\",\"newer\":true," +
            "\"context\":[{\"kind\":\"languages\",\"source_lang\":\"en\",\"target_lang\":\"ja\"}," +
            "{\"kind\":\"weather\",\"sky\":\"clear\"}," +
            "{\"kind\":\"plan_summary\",\"plan_id\":\"p1\",\"target_lang\":\"ja\",\"set_count\":3,\"sets_completed\":1}]}"

    private fun signature(body: String): Pair<String, String> {
        val now = (System.currentTimeMillis() / 1000).toString()
        val mac = Mac.getInstance("HmacSHA256").apply { init(SecretKeySpec(key, "HmacSHA256")) }
        val signed = mac.doFinal("$id.$now.$body".toByteArray())
        return now to "v1," + Base64.getEncoder().encodeToString(signed)
    }

    /** Posts [sent], signed as though it were [signedBody]. */
    private suspend fun ApplicationTestBuilder.post(
        signedBody: String,
        sent: String,
    ): HttpResponse {
        val (timestamp, signature) = signature(signedBody)
        return client.post("/render") {
            header("webhook-id", id)
            header("webhook-timestamp", timestamp)
            header("webhook-signature", signature)
            header("content-type", "application/json")
            setBody(sent)
        }
    }

    private fun mounted(block: suspend ApplicationTestBuilder.() -> Unit) =
        testApplication {
            routing { route("/render") { lingaraApp(app) } }
            block()
        }

    /**
     * 30.9.26am AC12: through Route.lingaraApp on Ktor's test host, a tampered body is 401 and never
     * reaches the render function; a valid render reaches it with the decoded subject, slot and
     * slices, an unknown slice kind skipped; an action's card_etag reaches its function byte for byte.
     */
    @Test
    fun verifiesTheRawBodyBeforeDispatch() =
        mounted {
            val tampered = post(render, render.replace("home.side", "plans.empty_detail"))
            assertEquals(401, tampered.status.value)
            assertEquals("", tampered.bodyAsText())
            assertNull(rendered)

            val ok = post(render, render)
            assertEquals(200, ok.status.value)
            assertEquals("application/json", ok.headers["content-type"])
            assertEquals("{\"card\":{\"elements\":[{\"type\":\"heading\",\"text\":\"home.side\",\"level\":1}]}}", ok.bodyAsText())
            val r = rendered!!
            assertEquals("lgr_sub_x", r.subject)
            assertEquals(AppSlotName.HOME_PERIOD_SIDE, r.slot)
            assertEquals(2, r.context.size)
            assertIs<ContextSliceLanguages>(r.context[0])
            assertEquals(3L, (r.context[1] as ContextSlicePlanSummary).setCount)

            val action =
                render.replace("app.render", "app.action").replace("\"newer\":true", "\"action_id\":\"inc\",\"card_etag\":\"$etag\"")
            val pressed = post(action, action)
            assertEquals(200, pressed.status.value)
            assertEquals(etag, acted!!.cardEtag)
            assertEquals("{\"card\":{\"elements\":[{\"type\":\"text\",\"text\":\"$etag\"}]}}", pressed.bodyAsText())
        }

    @Test
    fun answersTheFixedErrors() =
        mounted {
            assertEquals(405, client.get("/render").status.value)
            val unknown = render.replace("app.render", "app.unknown")
            val bad = post(unknown, unknown)
            assertEquals(400, bad.status.value)
            assertEquals("{\"error\":\"bad_request\"}", bad.bodyAsText())
            val big = " ".repeat(65537)
            assertEquals(413, post(big, big).status.value)
        }
}
