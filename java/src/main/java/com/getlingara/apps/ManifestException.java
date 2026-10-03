package com.getlingara.apps;

/**
 * A manifest breaks one of {@link Manifest#validateManifest}'s rules (ADR 30.9.26am D5): the first
 * broken one, by its closed name ({@code manifest_version}, {@code default_locale}, {@code name},
 * {@code description}, {@code render_url}, {@code slots}, {@code context}, {@code duplicate},
 * {@code too_large}).
 */
public final class ManifestException extends RuntimeException {
  private static final long serialVersionUID = 1L;

  /** The rule broken. */
  private final String rule;

  ManifestException(String rule) {
    super("the manifest breaks the rule " + rule);
    this.rule = rule;
  }

  /**
   * The first rule the manifest breaks.
   *
   * @return the rule's name
   */
  public String rule() {
    return rule;
  }
}
