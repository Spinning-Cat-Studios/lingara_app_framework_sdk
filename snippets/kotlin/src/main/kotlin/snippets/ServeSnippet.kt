package snippets

// lingara:begin serve
import com.getlingara.apps.kotlin.LingaraApp
import com.getlingara.apps.kotlin.ktor.lingaraApp
import io.ktor.server.cio.CIO
import io.ktor.server.engine.embeddedServer
import io.ktor.server.routing.route
import io.ktor.server.routing.routing

// lingara:end

/** The documentation site's serve example. */
fun serveSnippet(app: LingaraApp) {
    // lingara:begin serve
    // Route.lingaraApp reads the raw body itself, so content negotiation never parses it.
    embeddedServer(CIO, port = 8080) {
        routing {
            route("/lingara/render") { lingaraApp(app) }
        }
    }.start(wait = true)
    // lingara:end
}
