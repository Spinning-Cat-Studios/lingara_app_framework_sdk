package com.getlingara.apps;

/**
 * A card or a reply breaks a card rule (ADR 30.9.26am D5): {@link Card.Builder#build()}, {@link
 * CardReply#tutorNote(String)} and {@link Limits#validateReply} raise it, naming the first broken
 * rule in the contract's order. A kit refuses; it never cuts, strips or drops.
 */
public final class CardLimitException extends RuntimeException {
  private static final long serialVersionUID = 1L;

  /** The rule broken. */
  private final Limits.Reason reason;

  CardLimitException(Limits.Reason reason) {
    super("the reply breaks the card rule " + reason.wire());
    this.reason = reason;
  }

  /**
   * The first rule the card or reply breaks.
   *
   * @return the reason
   */
  public Limits.Reason reason() {
    return reason;
  }
}
