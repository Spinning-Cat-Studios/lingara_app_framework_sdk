package com.getlingara.apps;

/**
 * What a render or action function returns (ADR 30.9.26am D4): always a card, either alone (a
 * {@link Card}) or with a tutor note ({@link CardReply}). There is no {@code unchanged}, {@code
 * fallback} or {@code etag}: those are the relay's words to its clients, never an app's.
 */
public sealed interface Reply permits Card, CardReply {
  /**
   * A reply carrying {@code card}, to which a tutor note can be added.
   *
   * @param card the card
   * @return the reply
   */
  static CardReply reply(Card card) {
    return new CardReply(card, null);
  }
}
