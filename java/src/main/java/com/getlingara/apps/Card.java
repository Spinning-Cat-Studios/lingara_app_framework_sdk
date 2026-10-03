package com.getlingara.apps;

import com.fasterxml.jackson.databind.node.ObjectNode;
import com.getlingara.apps.model.ButtonStyle;
import com.getlingara.apps.model.CardElement;
import com.getlingara.apps.model.CardElementButton;
import com.getlingara.apps.model.CardElementDivider;
import com.getlingara.apps.model.CardElementHeading;
import com.getlingara.apps.model.CardElementLink;
import com.getlingara.apps.model.CardElementList;
import com.getlingara.apps.model.CardElementProgress;
import com.getlingara.apps.model.CardElementTerm;
import com.getlingara.apps.model.CardElementText;
import com.getlingara.apps.model.ListItem;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;

/**
 * A card that keeps every card rule (ADR 30.9.26am D5), built fluently:
 *
 * <pre>{@code
 * Card card =
 *     Card.card()
 *         .heading("Today", 1)
 *         .term("雨", "yǔ", "rain", "zh")
 *         .list(Item.text("one"), Item.term("二"))
 *         .button("Next", "next")
 *         .build();
 * }</pre>
 *
 * <p>Each element method takes its generated arm's closed field set. {@link Builder#build()}
 * refuses a card the relay would clamp, raising {@link CardLimitException} with the first broken
 * rule; it never cuts, strips or drops. A card is itself a {@link Reply}.
 */
public final class Card implements Reply {
  private final List<CardElement> elements;

  private Card(List<CardElement> elements) {
    this.elements = List.copyOf(elements);
  }

  /**
   * A new, empty card builder.
   *
   * @return the builder
   */
  public static Builder card() {
    return new Builder();
  }

  /**
   * The card's elements, in order.
   *
   * @return the generated elements
   */
  public List<CardElement> elements() {
    return elements;
  }

  /**
   * The generated card model.
   *
   * @return the model
   */
  public com.getlingara.apps.model.Card model() {
    return new com.getlingara.apps.model.Card().elements(new ArrayList<>(elements));
  }

  /** {@code {elements: [...]}} as the kit sends it. */
  ObjectNode toTree() {
    return Json.MAPPER.valueToTree(model());
  }

  /** The card builder; one method per element, each with its arm's fields. */
  public static final class Builder {
    private final List<CardElement> elements = new ArrayList<>();

    private Builder() {}

    private Builder add(CardElement element) {
      elements.add(element);
      return this;
    }

    /**
     * A heading.
     *
     * @param text 1–80 characters
     * @param level 1 or 2
     * @return this builder
     */
    public Builder heading(String text, int level) {
      return add(new CardElementHeading(text, level));
    }

    /**
     * A text element: 1–600 characters, {@code \n} allowed.
     *
     * @param text the text
     * @return this builder
     */
    public Builder text(String text) {
      return add(new CardElementText(text, null));
    }

    /**
     * A text element in a language.
     *
     * @param text the text
     * @param lang a language tag such as {@code ja}, or null
     * @return this builder
     */
    public Builder text(String text, String lang) {
      return add(new CardElementText(text, lang));
    }

    /**
     * A term.
     *
     * @param word 1–60 characters
     * @return this builder
     */
    public Builder term(String word) {
      return add(new CardElementTerm(word, null, null, null));
    }

    /**
     * A term with its reading, gloss and language, each optional (null).
     *
     * @param word 1–60 characters
     * @param reading at most 120 characters, or null
     * @param gloss at most 160 characters, or null
     * @param lang a language tag, or null
     * @return this builder
     */
    public Builder term(String word, String reading, String gloss, String lang) {
      return add(new CardElementTerm(word, reading, gloss, lang));
    }

    /**
     * A list of 1–20 items.
     *
     * @param items the items
     * @return this builder
     */
    public Builder list(Item... items) {
      return list(Arrays.asList(items));
    }

    /**
     * A list of 1–20 items.
     *
     * @param items the items
     * @return this builder
     */
    public Builder list(List<Item> items) {
      List<ListItem> models = items.stream().map(Item::model).toList();
      return add(new CardElementList(models));
    }

    /**
     * A progress bar.
     *
     * @param value 0 to 1 inclusive
     * @param label 1–60 characters
     * @return this builder
     */
    public Builder progress(double value, String label) {
      return add(new CardElementProgress(value, label));
    }

    /**
     * A divider.
     *
     * @return this builder
     */
    public Builder divider() {
      return add(new CardElementDivider());
    }

    /**
     * A button. Pressing it sends an action whose {@code action_id} is {@code action}.
     *
     * @param label 1–32 characters
     * @param action matching {@code [A-Za-z0-9_.:-]{1,64}}
     * @return this builder
     */
    public Builder button(String label, String action) {
      return add(new CardElementButton(label, action, null));
    }

    /**
     * A styled button.
     *
     * @param label 1–32 characters
     * @param action matching {@code [A-Za-z0-9_.:-]{1,64}}
     * @param style its style, or null
     * @return this builder
     */
    public Builder button(String label, String action, ButtonStyle style) {
      return add(new CardElementButton(label, action, style));
    }

    /**
     * A link: {@code https}, no user information, a host name (never an IP address), at most 2 048
     * bytes.
     *
     * @param label 1–60 characters
     * @param url the URL
     * @return this builder
     */
    public Builder link(String label, String url) {
      return add(new CardElementLink(label, url));
    }

    /**
     * The card, once it keeps every card rule.
     *
     * @return the card
     * @throws CardLimitException naming the first rule the card breaks
     */
    public Card build() {
      Card card = new Card(elements);
      Limits.cardReason(card.toTree()).ifPresent(Limits::refuse);
      return card;
    }
  }
}
