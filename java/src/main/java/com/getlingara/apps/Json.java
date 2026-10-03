package com.getlingara.apps;

import com.fasterxml.jackson.annotation.JsonInclude;
import com.fasterxml.jackson.databind.DeserializationFeature;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.SerializationFeature;

/**
 * The kit's one Jackson mapper (ADR 30.9.26am D4, D5, D6).
 *
 * <ul>
 *   <li>Lenient on the way in: an unknown field is ignored wherever it appears, whatever the view's
 *       {@code additionalProperties} says, so an older kit still answers a newer relay. Trailing
 *       tokens after the body's object are refused, since that body does not decode.
 *   <li>Exact on the way out: compact JSON, raw UTF-8, {@code /} unescaped, no HTML escaping (the
 *       defaults), and a null member omitted rather than written as {@code null}.
 * </ul>
 */
final class Json {
  static final ObjectMapper MAPPER =
      new ObjectMapper()
          .configure(DeserializationFeature.FAIL_ON_UNKNOWN_PROPERTIES, false)
          .configure(DeserializationFeature.FAIL_ON_TRAILING_TOKENS, true)
          .configure(SerializationFeature.FAIL_ON_EMPTY_BEANS, false)
          .setDefaultPropertyInclusion(JsonInclude.Include.NON_NULL);

  private Json() {}
}
