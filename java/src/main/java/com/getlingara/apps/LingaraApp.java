package com.getlingara.apps;

import com.getlingara.apps.model.AppActionRequest;
import com.getlingara.apps.model.AppRenderRequest;
import com.getlingara.client.events.Webhook;
import java.io.IOException;
import java.io.InputStream;
import java.time.Clock;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.Objects;

/**
 * A Lingara app: one render function, zero or more action functions by id, and the app's signing
 * secrets (ADR 30.9.26am D4).
 *
 * <pre>{@code
 * LingaraApp app =
 *     LingaraApp.builder()
 *         .secrets(System.getenv("LINGARA_APP_SECRET"))
 *         .render(request -> Card.card().heading("Today", 1).build())
 *         .action("next", request -> Card.card().text(request.getCardEtag()).build())
 *         .build();
 * HttpServer server = HttpServer.create(new InetSocketAddress(8080), 0);
 * server.createContext("/lingara/render", JdkHttpHandler.of(app));
 * }</pre>
 *
 * <p>{@link #handle} is the framework-neutral core every adapter calls: {@code 405} for anything
 * but {@code POST} before the body is read, {@code 413} over 64 KiB, the Lingara library's {@code
 * Webhook.verifySignature} ({@code 401}, empty), lenient decoding ({@code 400}), dispatch, {@link
 * Limits#validateReply} and {@code 500 {"error":"handler_failed"}} for a raised function or a
 * refused reply. The kit never reads {@code user-agent} and enforces no time limit.
 */
public final class LingaraApp {
  /** A render function: a card for the slot the request names. */
  @FunctionalInterface
  public interface Render {
    /**
     * Renders a card.
     *
     * @param request the decoded request
     * @return the reply: a {@link Card}, or {@code Reply.reply(card).tutorNote(…)}
     * @throws Exception any failure, answered as {@code 500}
     */
    Reply render(AppRenderRequest request) throws Exception;
  }

  /** An action function: the card after the learner pressed a button. */
  @FunctionalInterface
  public interface Action {
    /**
     * Answers an action.
     *
     * @param request the decoded request; its {@code card_etag} is exactly as received
     * @return the reply: a {@link Card}, or {@code Reply.reply(card).tutorNote(…)}
     * @throws Exception any failure, answered as {@code 500}
     */
    Reply act(AppActionRequest request) throws Exception;
  }

  private final Core core;

  private LingaraApp(Core core) {
    this.core = core;
  }

  /**
   * A new app builder.
   *
   * @return the builder
   */
  public static Builder builder() {
    return new Builder();
  }

  /**
   * Answers one request.
   *
   * @param method the HTTP method
   * @param headers the request's headers, any case
   * @param body the request body, read at most 64 KiB + 1 bytes and only for a {@code POST}
   * @return the response to send
   * @throws IOException when the body cannot be read
   */
  public AppResponse handle(String method, Map<String, List<String>> headers, InputStream body)
      throws IOException {
    return core.handle(method, headers, body);
  }

  /** The app builder. */
  public static final class Builder {
    private List<String> secrets = List.of();
    private Clock clock = Clock.systemUTC();
    private Render render;
    private final Map<String, Action> actions = new LinkedHashMap<>();

    private Builder() {}

    /**
     * The app's signing secrets: one, or two during a rotation.
     *
     * @param secrets each {@code lgr_whsec_…}
     * @return this builder
     */
    public Builder secrets(String... secrets) {
      this.secrets = List.of(secrets);
      return this;
    }

    /**
     * The clock a request's timestamp is checked against; the system clock by default.
     *
     * @param clock the clock
     * @return this builder
     */
    public Builder clock(Clock clock) {
      this.clock = Objects.requireNonNull(clock, "clock");
      return this;
    }

    /**
     * The render function.
     *
     * @param render the function
     * @return this builder
     */
    public Builder render(Render render) {
      this.render = Objects.requireNonNull(render, "render");
      return this;
    }

    /**
     * The function for a button's {@code action}, which comes back as the request's {@code
     * action_id}.
     *
     * @param actionId the id
     * @param action the function
     * @return this builder
     */
    public Builder action(String actionId, Action action) {
      actions.put(Objects.requireNonNull(actionId, "actionId"), Objects.requireNonNull(action));
      return this;
    }

    /**
     * The app. Its verifier is built here, so a malformed secret fails now, not on a request.
     *
     * @return the app
     * @throws IllegalStateException for no secret, a malformed one, or no render function
     */
    public LingaraApp build() {
      if (render == null) {
        throw new IllegalStateException("a LingaraApp needs a render function");
      }
      Webhook webhook = Webhook.of(secrets, clock);
      return new LingaraApp(new Core(webhook, render, Map.copyOf(actions)));
    }
  }
}
