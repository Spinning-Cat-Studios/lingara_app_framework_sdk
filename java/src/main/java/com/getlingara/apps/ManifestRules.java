package com.getlingara.apps;

import com.fasterxml.jackson.core.JsonProcessingException;
import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.node.ObjectNode;
import com.getlingara.apps.model.AppSlotName;
import com.getlingara.apps.model.ContextSliceKind;
import java.util.HashSet;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import java.util.Set;
import java.util.function.Predicate;

/**
 * {@code validateManifest}'s rules, in {@code conformance/vectors/manifest.json}'s order (ADR
 * 30.9.26am D5): the first broken rule names the refusal. What only the upload can know (the
 * supported locales, the rest of the URL policy, DNS, the client's allowed scopes) is not here.
 */
final class ManifestRules {
  private static final List<String> STORED =
      List.of("default_locale", "name", "description", "slots", "context", "scopes", "tutor_note");

  private ManifestRules() {}

  /** The first rule {@code m} breaks, if any. */
  static Optional<String> firstBroken(JsonNode m) {
    List<Map.Entry<String, Predicate<JsonNode>>> rules =
        List.of(
            Map.entry("manifest_version", ManifestRules::badVersion),
            Map.entry("default_locale", ManifestRules::badDefaultLocale),
            Map.entry("name", x -> !localeMap(x.get("name"), Limits.MANIFEST_NAME_MAX_CHARS)),
            Map.entry(
                "description",
                x -> !localeMap(x.get("description"), Limits.MANIFEST_DESCRIPTION_MAX_CHARS)),
            Map.entry("render_url", x -> !isHttpsUrl(x.get("render_url"))),
            Map.entry("slots", ManifestRules::badSlots),
            Map.entry("context", ManifestRules::badContext),
            Map.entry("duplicate", ManifestRules::hasDuplicate),
            Map.entry("too_large", ManifestRules::tooLarge));
    return rules.stream().filter(r -> r.getValue().test(m)).map(Map.Entry::getKey).findFirst();
  }

  private static boolean badVersion(JsonNode m) {
    JsonNode v = m.path("manifest_version");
    return !(v.isIntegralNumber() && v.canConvertToLong() && v.asLong() == 1);
  }

  private static boolean badDefaultLocale(JsonNode m) {
    JsonNode locale = m.path("default_locale");
    if (!locale.isTextual() || locale.asText().isEmpty()) {
      return true;
    }
    return missingKey(m.path("name"), locale.asText())
        || missingKey(m.path("description"), locale.asText());
  }

  /** An object that lacks {@code key}; anything else is the name or description rule's. */
  private static boolean missingKey(JsonNode map, String key) {
    return map.isObject() && !map.has(key);
  }

  /** A non-empty object whose every value is a clean string of 1–{@code max} characters. */
  private static boolean localeMap(JsonNode map, int max) {
    if (map == null || !map.isObject() || map.isEmpty()) {
      return false;
    }
    for (JsonNode value : map) {
      if (!value.isTextual() || !cleanLabel(Text.trim(value.asText()), max)) {
        return false;
      }
    }
    return true;
  }

  private static boolean cleanLabel(String trimmed, int max) {
    int n = Text.scalars(trimmed);
    return n >= 1 && n <= max && !Text.hasControl(trimmed, false) && !Text.hasZeroWidth(trimmed);
  }

  /** {@code https://} (any case) and at least one character other than {@code / ? #}. */
  private static boolean isHttpsUrl(JsonNode url) {
    if (url == null || !url.isTextual()) {
      return false;
    }
    String s = url.asText();
    return s.length() > 8
        && s.substring(0, 8).equalsIgnoreCase("https://")
        && "/?#".indexOf(s.charAt(8)) < 0;
  }

  private static boolean badSlots(JsonNode m) {
    JsonNode slots = m.get("slots");
    if (slots == null || !slots.isArray() || slots.isEmpty()) {
      return true;
    }
    return !allOf(slots, v -> AppSlotName.fromValue(v) != null);
  }

  private static boolean badContext(JsonNode m) {
    JsonNode context = m.get("context");
    if (context == null) {
      return false;
    }
    return !context.isArray() || !allOf(context, v -> ContextSliceKind.fromValue(v).isPresent());
  }

  /** Every element is a string {@code known} accepts. */
  private static boolean allOf(JsonNode list, Predicate<String> known) {
    for (JsonNode v : list) {
      if (!v.isTextual() || !isKnown(known, v.asText())) {
        return false;
      }
    }
    return true;
  }

  private static boolean isKnown(Predicate<String> known, String value) {
    try {
      return known.test(value);
    } catch (IllegalArgumentException e) {
      // The generated AppSlotName.fromValue throws for a value it does not know.
      return false;
    }
  }

  private static boolean hasDuplicate(JsonNode m) {
    for (String list : List.of("slots", "context", "scopes")) {
      JsonNode values = m.path(list);
      Set<JsonNode> seen = new HashSet<>();
      for (JsonNode v : values.isArray() ? values : List.<JsonNode>of()) {
        if (!seen.add(v)) {
          return true;
        }
      }
    }
    return false;
  }

  /**
   * The seven stored keys, {@code context} and {@code scopes} defaulting to [] and the note to
   * false.
   */
  private static boolean tooLarge(JsonNode m) {
    ObjectNode stored = Json.MAPPER.createObjectNode();
    for (String key : STORED) {
      JsonNode value = m.get(key);
      if (value != null) {
        stored.set(key, value);
      } else if (key.equals("tutor_note")) {
        stored.put(key, false);
      } else {
        stored.putArray(key);
      }
    }
    try {
      return Json.MAPPER.writeValueAsBytes(stored).length > Limits.MANIFEST_MAX_BYTES;
    } catch (JsonProcessingException e) {
      throw new IllegalStateException("a JSON tree always encodes", e);
    }
  }
}
