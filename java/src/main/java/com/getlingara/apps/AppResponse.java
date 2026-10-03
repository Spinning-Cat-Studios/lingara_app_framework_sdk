package com.getlingara.apps;

import java.nio.charset.StandardCharsets;

/**
 * What {@link LingaraApp#handle} answers: a status and a body, which an adapter writes as they are.
 * A non-empty body is always {@code application/json}; a {@code 401}, {@code 405} or {@code 413}
 * has none.
 *
 * @param status the HTTP status
 * @param body the body, possibly empty
 */
public record AppResponse(int status, byte[] body) {
  /** The content type every response with a body carries. */
  public static final String JSON = "application/json";

  static AppResponse empty(int status) {
    return new AppResponse(status, new byte[0]);
  }

  static AppResponse error(int status, String error) {
    String json = "{\"error\":\"" + error + "\"}";
    return new AppResponse(status, json.getBytes(StandardCharsets.UTF_8));
  }

  /**
   * The body's content type: {@link #JSON}, or null when there is no body.
   *
   * @return the content type
   */
  public String contentType() {
    return body.length > 0 ? JSON : null;
  }

  /**
   * The body (a copy).
   *
   * @return the bytes
   */
  @Override
  public byte[] body() {
    return body.clone();
  }
}
