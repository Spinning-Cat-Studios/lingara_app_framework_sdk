package snippets;

// lingara:begin serve
import com.getlingara.apps.JdkHttpHandler;
import com.getlingara.apps.LingaraApp;
import com.sun.net.httpserver.HttpServer;
import java.io.IOException;
import java.net.InetSocketAddress;

// lingara:end

/** The documentation site's serve example. */
public final class ServeSnippet {
  private ServeSnippet() {}

  static void serve(LingaraApp app) throws IOException {
    // lingara:begin serve
    // The JDK's own server: the handler reads the raw body, verifies it, then dispatches.
    HttpServer server = HttpServer.create(new InetSocketAddress(8080), 0);
    server.createContext("/lingara/render", JdkHttpHandler.of(app));
    server.start();
    // lingara:end
  }
}
