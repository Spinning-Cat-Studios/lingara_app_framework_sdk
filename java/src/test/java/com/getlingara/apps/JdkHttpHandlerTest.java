package com.getlingara.apps;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertInstanceOf;

import com.getlingara.apps.model.AppActionRequest;
import com.getlingara.apps.model.AppRenderRequest;
import com.getlingara.apps.model.AppSlotName;
import com.getlingara.apps.model.ContextSliceLanguages;
import com.getlingara.apps.model.ContextSlicePlanSummary;
import com.sun.net.httpserver.HttpServer;
import java.net.InetAddress;
import java.net.InetSocketAddress;
import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.nio.charset.StandardCharsets;
import java.util.Base64;
import java.util.concurrent.atomic.AtomicReference;
import javax.crypto.Mac;
import javax.crypto.spec.SecretKeySpec;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;

class JdkHttpHandlerTest {
  private static final byte[] KEY =
      "0123456789abcdef0123456789abcdef".getBytes(StandardCharsets.UTF_8);
  private static final String SECRET = "lgr_whsec_" + Base64.getEncoder().encodeToString(KEY);
  private static final String ID = "lgr_msg_00000000000000000000000000000000";
  private static final String ETAG = "c1_8f14e45fceea167a5a36dedd4bea2543";

  private final AtomicReference<AppRenderRequest> rendered = new AtomicReference<>();
  private final AtomicReference<AppActionRequest> acted = new AtomicReference<>();
  private final HttpClient client = HttpClient.newHttpClient();
  private HttpServer server;
  private URI uri;

  @BeforeEach
  void start() throws Exception {
    LingaraApp app =
        LingaraApp.builder()
            .secrets(SECRET)
            .render(
                r -> {
                  rendered.set(r);
                  return Card.card().heading(r.getSlot().getValue(), 1).build();
                })
            .action(
                "inc",
                r -> {
                  acted.set(r);
                  return Card.card().text(r.getCardEtag()).build();
                })
            .build();
    server = HttpServer.create(new InetSocketAddress(InetAddress.getLoopbackAddress(), 0), 0);
    server.createContext("/render", JdkHttpHandler.of(app));
    server.start();
    uri = URI.create("http://127.0.0.1:" + server.getAddress().getPort() + "/render");
  }

  @AfterEach
  void stop() {
    server.stop(0);
  }

  private static String signature(String body) throws Exception {
    long now = System.currentTimeMillis() / 1000;
    Mac mac = Mac.getInstance("HmacSHA256");
    mac.init(new SecretKeySpec(KEY, "HmacSHA256"));
    byte[] signed = mac.doFinal((ID + "." + now + "." + body).getBytes(StandardCharsets.UTF_8));
    return now + " v1," + Base64.getEncoder().encodeToString(signed);
  }

  /** Posts {@code sent}, signed as though it were {@code signedBody}. */
  private HttpResponse<String> post(String signedBody, String sent) throws Exception {
    String[] sig = signature(signedBody).split(" ", 2);
    HttpRequest request =
        HttpRequest.newBuilder(uri)
            .header("webhook-id", ID)
            .header("webhook-timestamp", sig[0])
            .header("webhook-signature", sig[1])
            .header("content-type", "application/json")
            .POST(HttpRequest.BodyPublishers.ofString(sent))
            .build();
    return client.send(request, HttpResponse.BodyHandlers.ofString());
  }

  private static String render() {
    return "{\"type\":\"app.render\",\"id\":\""
        + ID
        + "\",\"install_id\":\"7c1d2e3f-4a5b-4c6d-8e7f-9a0b1c2d3e4f\","
        + "\"subject\":\"lgr_sub_x\",\"slot\":\"home.side\",\"locale\":\"ja\",\"newer\":true,"
        + "\"context\":[{\"kind\":\"languages\",\"source_lang\":\"en\",\"target_lang\":\"ja\"},"
        + "{\"kind\":\"weather\",\"sky\":\"clear\"},"
        + "{\"kind\":\"plan_summary\",\"plan_id\":\"p1\",\"target_lang\":\"ja\","
        + "\"set_count\":3,\"sets_completed\":1}]}";
  }

  /**
   * 30.9.26am AC11: through JdkHttpHandler, a tampered body is 401 with an empty body and never
   * reaches the render function; a valid render reaches it with the decoded subject, slot and
   * slices, an unknown slice kind skipped; an action's card_etag reaches its function byte for
   * byte.
   */
  @Test
  void verifiesTheRawBodyBeforeDispatch() throws Exception {
    HttpResponse<String> tampered =
        post(render(), render().replace("home.side", "plans.empty_detail"));
    assertEquals(401, tampered.statusCode());
    assertEquals("", tampered.body());
    assertEquals(null, rendered.get());

    HttpResponse<String> ok = post(render(), render());
    assertEquals(200, ok.statusCode());
    assertEquals("application/json", ok.headers().firstValue("content-type").orElse(""));
    assertEquals(
        "{\"card\":{\"elements\":[{\"type\":\"heading\",\"text\":\"home.side\",\"level\":1}]}}",
        ok.body());
    AppRenderRequest r = rendered.get();
    assertEquals("lgr_sub_x", r.getSubject());
    assertEquals(AppSlotName.HOME_SIDE, r.getSlot());
    assertEquals(2, r.getContext().size());
    assertInstanceOf(ContextSliceLanguages.class, r.getContext().get(0));
    assertEquals(3L, ((ContextSlicePlanSummary) r.getContext().get(1)).setCount());

    String action =
        render()
            .replace("app.render", "app.action")
            .replace("\"newer\":true", "\"action_id\":\"inc\",\"card_etag\":\"" + ETAG + "\"");
    HttpResponse<String> pressed = post(action, action);
    assertEquals(200, pressed.statusCode());
    assertEquals(ETAG, acted.get().getCardEtag());
    assertEquals(
        "{\"card\":{\"elements\":[{\"type\":\"text\",\"text\":\"" + ETAG + "\"}]}}",
        pressed.body());
  }

  @Test
  void answersTheFixedErrors() throws Exception {
    HttpResponse<String> get =
        client.send(
            HttpRequest.newBuilder(uri).GET().build(), HttpResponse.BodyHandlers.ofString());
    assertEquals(405, get.statusCode());
    String unknown = render().replace("app.render", "app.unknown");
    HttpResponse<String> bad = post(unknown, unknown);
    assertEquals(400, bad.statusCode());
    assertEquals("{\"error\":\"bad_request\"}", bad.body());
    String unregistered =
        render()
            .replace("app.render", "app.action")
            .replace("\"newer\":true", "\"action_id\":\"nope\",\"card_etag\":\"" + ETAG + "\"");
    assertEquals(400, post(unregistered, unregistered).statusCode());
    String big = " ".repeat(Limits.REQUEST_MAX_BYTES + 1);
    assertEquals(413, post(big, big).statusCode());
  }
}
