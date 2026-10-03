package com.getlingara.apps;

import com.fasterxml.jackson.core.JsonProcessingException;
import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.node.ArrayNode;
import com.fasterxml.jackson.databind.node.ObjectNode;
import com.getlingara.apps.model.AppSlotName;
import com.getlingara.apps.model.ContextSliceKind;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.Objects;

/**
 * An app's manifest, the document {@code /admin/apps} accepts as an upload (ADR 30.9.26am D5):
 *
 * <pre>{@code
 * String json =
 *     Manifest.manifest()
 *         .defaultLocale("en")
 *         .name("Daily five")
 *         .description("Five words to review, picked from your plan.")
 *         .renderUrl("https://apps.example.com/lingara/render")
 *         .slots(AppSlotName.HOME_SIDE)
 *         .context(ContextSliceKind.LANGUAGES, ContextSliceKind.PLAN_SUMMARY)
 *         .build()
 *         .toJson();
 * }</pre>
 *
 * <p>The manifest is in no spec the kit generates from, so this builder is hand-written; its closed
 * values ({@link AppSlotName}, {@link ContextSliceKind}) are generated. {@link Builder#build()}
 * runs {@link #validateManifest}. The icon is uploaded in the console, never embedded.
 */
public final class Manifest {
  private final ObjectNode json;

  private Manifest(ObjectNode json) {
    this.json = json;
  }

  /**
   * A new manifest builder.
   *
   * @return the builder
   */
  public static Builder manifest() {
    return new Builder();
  }

  /**
   * Checks a manifest's wire form against every rule a kit can know, in the vectors' order.
   *
   * @param manifest the manifest as a JSON tree
   * @throws ManifestException naming the first rule it breaks
   */
  public static void validateManifest(JsonNode manifest) {
    ManifestRules.firstBroken(manifest)
        .ifPresent(
            rule -> {
              throw new ManifestException(rule);
            });
  }

  /**
   * The manifest as compact JSON, ready to upload.
   *
   * @return the JSON text
   */
  public String toJson() {
    try {
      return Json.MAPPER.writeValueAsString(json);
    } catch (JsonProcessingException e) {
      throw new IllegalStateException("a JSON tree always encodes", e);
    }
  }

  /**
   * The manifest as a JSON tree (a copy).
   *
   * @return the tree
   */
  public ObjectNode toTree() {
    return json.deepCopy();
  }

  /** The manifest builder. {@code listed} defaults to false, {@code tutor_note} to false. */
  public static final class Builder {
    private String defaultLocale;
    private Object name;
    private Object description;
    private String renderUrl;
    private final List<String> slots = new ArrayList<>();
    private final List<String> context = new ArrayList<>();
    private final List<String> scopes = new ArrayList<>();
    private boolean tutorNote;
    private boolean listed;

    private Builder() {}

    /**
     * The locale {@code name} and {@code description} always carry.
     *
     * @param locale a locale such as {@code en}
     * @return this builder
     */
    public Builder defaultLocale(String locale) {
      this.defaultLocale = locale;
      return this;
    }

    /**
     * The name in the default locale only.
     *
     * @param name 1–40 characters
     * @return this builder
     */
    public Builder name(String name) {
      this.name = Objects.requireNonNull(name, "name");
      return this;
    }

    /**
     * The name per locale; it must contain the default locale.
     *
     * @param names locale to name, each 1–40 characters
     * @return this builder
     */
    public Builder name(Map<String, String> names) {
      this.name = new LinkedHashMap<>(names);
      return this;
    }

    /**
     * The description in the default locale only.
     *
     * @param description 1–280 characters
     * @return this builder
     */
    public Builder description(String description) {
      this.description = Objects.requireNonNull(description, "description");
      return this;
    }

    /**
     * The description per locale; it must contain the default locale.
     *
     * @param descriptions locale to description, each 1–280 characters
     * @return this builder
     */
    public Builder description(Map<String, String> descriptions) {
      this.description = new LinkedHashMap<>(descriptions);
      return this;
    }

    /**
     * Where Lingara sends the app's signed requests.
     *
     * @param url an {@code https} URL
     * @return this builder
     */
    public Builder renderUrl(String url) {
      this.renderUrl = url;
      return this;
    }

    /**
     * The slots the app draws in; at least one.
     *
     * @param slots the slots
     * @return this builder
     */
    public Builder slots(AppSlotName... slots) {
      for (AppSlotName s : slots) {
        this.slots.add(s.getValue());
      }
      return this;
    }

    /**
     * The context slices the app asks the learner to share.
     *
     * @param kinds the slice kinds
     * @return this builder
     */
    public Builder context(ContextSliceKind... kinds) {
      for (ContextSliceKind k : kinds) {
        this.context.add(k.getValue());
      }
      return this;
    }

    /**
     * The API scopes the app's client asks for, such as {@code plans:read}.
     *
     * @param scopes the scopes
     * @return this builder
     */
    public Builder scopes(String... scopes) {
      this.scopes.addAll(List.of(scopes));
      return this;
    }

    /**
     * Whether the app may add a tutor note to its replies.
     *
     * @param tutorNote true to ask for it
     * @return this builder
     */
    public Builder tutorNote(boolean tutorNote) {
      this.tutorNote = tutorNote;
      return this;
    }

    /**
     * Whether the app is listed for every learner, rather than private to its owner.
     *
     * @param listed true to list it
     * @return this builder
     */
    public Builder listed(boolean listed) {
      this.listed = listed;
      return this;
    }

    /**
     * The manifest, once it keeps every rule {@link #validateManifest} checks.
     *
     * @return the manifest
     * @throws ManifestException naming the first rule it breaks
     */
    public Manifest build() {
      ObjectNode json = Json.MAPPER.createObjectNode();
      json.put("manifest_version", 1);
      json.put("default_locale", defaultLocale);
      json.set("name", localeMap(name));
      json.set("description", localeMap(description));
      json.put("render_url", renderUrl);
      json.set("slots", strings(slots));
      json.set("context", strings(context));
      json.set("scopes", strings(scopes));
      json.put("tutor_note", tutorNote);
      json.put("listed", listed);
      if (defaultLocale == null) {
        json.remove("default_locale");
      }
      validateManifest(json);
      return new Manifest(json);
    }

    /** A bare string is sugar for {@code {default_locale: s}}. */
    private JsonNode localeMap(Object value) {
      if (value instanceof String s) {
        return Json.MAPPER.valueToTree(Map.of(defaultLocale == null ? "" : defaultLocale, s));
      }
      return Json.MAPPER.valueToTree(value);
    }

    private static ArrayNode strings(List<String> values) {
      ArrayNode array = Json.MAPPER.createArrayNode();
      values.forEach(array::add);
      return array;
    }
  }
}
