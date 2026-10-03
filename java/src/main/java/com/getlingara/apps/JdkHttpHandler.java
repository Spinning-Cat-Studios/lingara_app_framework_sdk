package com.getlingara.apps;

import com.sun.net.httpserver.HttpExchange;
import com.sun.net.httpserver.HttpHandler;
import java.io.IOException;
import java.io.OutputStream;
import java.util.Objects;

/**
 * The JDK's own HTTP server as an adapter (ADR 30.9.26am D2): {@code
 * server.createContext("/lingara/render", JdkHttpHandler.of(app))}. It hands the method, the
 * headers and the raw body stream to {@link LingaraApp#handle} and writes back what it answers. The
 * module requires {@code jdk.httpserver} {@code static}, so an app that never names this class runs
 * on an image without it.
 */
// jdk.httpserver is required static on purpose (D1), so javac's note that its types are not
// re-exported is the design, not an oversight.
@SuppressWarnings("exports")
public final class JdkHttpHandler implements HttpHandler {
  private final LingaraApp app;

  private JdkHttpHandler(LingaraApp app) {
    this.app = app;
  }

  /**
   * An {@code HttpHandler} for {@code app}.
   *
   * @param app the app
   * @return the handler
   */
  public static JdkHttpHandler of(LingaraApp app) {
    return new JdkHttpHandler(Objects.requireNonNull(app, "app"));
  }

  @Override
  public void handle(HttpExchange exchange) throws IOException {
    try {
      AppResponse response =
          app.handle(
              exchange.getRequestMethod(), exchange.getRequestHeaders(), exchange.getRequestBody());
      byte[] body = response.body();
      if (body.length > 0) {
        exchange.getResponseHeaders().set("content-type", AppResponse.JSON);
      }
      exchange.sendResponseHeaders(response.status(), body.length > 0 ? body.length : -1);
      if (body.length > 0) {
        try (OutputStream out = exchange.getResponseBody()) {
          out.write(body);
        }
      }
    } finally {
      exchange.close();
    }
  }
}
