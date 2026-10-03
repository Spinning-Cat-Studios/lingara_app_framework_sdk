package com.getlingara.apps;

import com.fasterxml.jackson.core.JacksonException;
import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.node.ArrayNode;
import com.fasterxml.jackson.databind.node.ObjectNode;
import com.getlingara.apps.model.AppActionRequest;
import com.getlingara.apps.model.AppOperation;
import com.getlingara.apps.model.AppRenderRequest;
import com.getlingara.apps.model.ContextSliceKind;
import com.getlingara.client.events.Webhook;
import com.getlingara.client.events.WebhookVerificationException;
import java.io.IOException;
import java.io.InputStream;
import java.util.List;
import java.util.Map;

/**
 * The handler core (ADR 30.9.26am D4): verify, decode, dispatch, validate, encode. It is
 * framework-neutral; {@link JdkHttpHandler} is a few lines over it.
 */
final class Core {
  private static final System.Logger LOG = System.getLogger("com.getlingara.apps");
  private static final List<String> REQUIRED =
      List.of("id", "install_id", "subject", "slot", "locale", "context");
  private static final List<String> ACTION_REQUIRED = List.of("action_id", "card_etag");

  private final Webhook webhook;
  private final LingaraApp.Render render;
  private final Map<String, LingaraApp.Action> actions;

  Core(Webhook webhook, LingaraApp.Render render, Map<String, LingaraApp.Action> actions) {
    this.webhook = webhook;
    this.render = render;
    this.actions = actions;
  }

  /** A body that does not decode into the operation's request: answered {@code 400}. */
  private static final class BadRequest extends Exception {
    private static final long serialVersionUID = 1L;
  }

  AppResponse handle(String method, Map<String, List<String>> headers, InputStream in)
      throws IOException {
    if (!"POST".equalsIgnoreCase(method)) {
      return AppResponse.empty(405);
    }
    byte[] body = in.readNBytes(Limits.REQUEST_MAX_BYTES + 1);
    if (body.length > Limits.REQUEST_MAX_BYTES) {
      return AppResponse.empty(413);
    }
    try {
      webhook.verifySignature(body, headers);
    } catch (WebhookVerificationException e) {
      return AppResponse.empty(401);
    }
    try {
      return dispatch(decode(body));
    } catch (BadRequest e) {
      return AppResponse.error(400, "bad_request");
    }
  }

  /** The body as an object, with every context slice of an unknown kind skipped. */
  private static ObjectNode decode(byte[] body) throws BadRequest {
    JsonNode tree;
    try {
      tree = Json.MAPPER.readTree(body);
    } catch (IOException e) {
      throw new BadRequest();
    }
    if (tree == null || !tree.isObject()) {
      throw new BadRequest();
    }
    ObjectNode request = (ObjectNode) tree;
    if (request.get("context") instanceof ArrayNode context) {
      request.set("context", knownSlices(context));
    }
    return request;
  }

  private static ArrayNode knownSlices(ArrayNode context) {
    ArrayNode known = Json.MAPPER.createArrayNode();
    for (JsonNode slice : context) {
      String kind = slice.path("kind").asText("");
      if (ContextSliceKind.fromValue(kind).isPresent()) {
        known.add(slice);
      } else {
        LOG.log(System.Logger.Level.INFO, "skipped a context slice of unknown kind {0}", kind);
      }
    }
    return known;
  }

  private AppResponse dispatch(ObjectNode request) throws BadRequest {
    AppOperation op =
        AppOperation.fromValue(request.path("type").asText("")).orElseThrow(BadRequest::new);
    requireFields(request, REQUIRED);
    String installId = request.path("install_id").asText();
    // Exhaustive over the generated operations: a third one fails compilation here.
    return switch (op) {
      case APP_RENDER -> {
        AppRenderRequest r = read(request, AppRenderRequest.class);
        yield reply(op, installId, () -> render.render(r));
      }
      case APP_ACTION -> {
        requireFields(request, ACTION_REQUIRED);
        AppActionRequest r = read(request, AppActionRequest.class);
        LingaraApp.Action action = actions.get(r.getActionId());
        if (action == null) {
          throw new BadRequest();
        }
        yield reply(op, installId, () -> action.act(r));
      }
    };
  }

  private static void requireFields(ObjectNode request, List<String> names) throws BadRequest {
    for (String name : names) {
      if (!request.hasNonNull(name)) {
        throw new BadRequest();
      }
    }
  }

  private static <T> T read(ObjectNode request, Class<T> type) throws BadRequest {
    try {
      return Json.MAPPER.treeToValue(request, type);
    } catch (JacksonException | IllegalArgumentException e) {
      throw new BadRequest();
    }
  }

  /** A developer function's call, which may throw anything. */
  @FunctionalInterface
  private interface Call {
    Reply call() throws Exception;
  }

  private static AppResponse reply(AppOperation op, String installId, Call call) {
    try {
      Reply reply = call.call();
      if (reply == null) {
        throw new IllegalStateException("the function returned no reply");
      }
      return new AppResponse(200, Limits.validateReply(toTree(reply)));
    } catch (CardLimitException e) {
      return failed(op, installId, e.reason().wire());
    } catch (Exception e) {
      return failed(op, installId, e.getClass().getName());
    }
  }

  private static ObjectNode toTree(Reply reply) {
    if (reply instanceof Card card) {
      ObjectNode tree = Json.MAPPER.createObjectNode();
      tree.set("card", card.toTree());
      return tree;
    }
    return ((CardReply) reply).toTree();
  }

  /** Logs the operation, the install id and the reason; never the body, a secret or a signature. */
  private static AppResponse failed(AppOperation op, String installId, String reason) {
    LOG.log(
        System.Logger.Level.WARNING,
        "{0} failed for install {1}: {2}",
        op.getValue(),
        installId,
        reason);
    return AppResponse.error(500, "handler_failed");
  }
}
