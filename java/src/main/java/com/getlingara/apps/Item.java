package com.getlingara.apps;

import com.getlingara.apps.model.ListItem;
import com.getlingara.apps.model.ListItemTerm;
import com.getlingara.apps.model.ListItemText;

/**
 * A list item, for {@link Card.Builder#list(Item...)}: {@code Item.text("one")}, {@code
 * Item.term("二", "èr", "two", "zh")}. Each holds its generated arm's closed field set.
 */
public final class Item {
  private final ListItem model;

  private Item(ListItem model) {
    this.model = model;
  }

  /**
   * A text item: 1–600 characters, {@code \n} allowed.
   *
   * @param text the text
   * @return the item
   */
  public static Item text(String text) {
    return new Item(new ListItemText(text, null));
  }

  /**
   * A text item in a language.
   *
   * @param text the text
   * @param lang a language tag such as {@code zh-Hant}, or null
   * @return the item
   */
  public static Item text(String text, String lang) {
    return new Item(new ListItemText(text, lang));
  }

  /**
   * A term item.
   *
   * @param word the word, 1–60 characters
   * @return the item
   */
  public static Item term(String word) {
    return new Item(new ListItemTerm(word, null, null, null));
  }

  /**
   * A term item with its reading, gloss and language, each optional (null).
   *
   * @param word the word, 1–60 characters
   * @param reading at most 120 characters, or null
   * @param gloss at most 160 characters, or null
   * @param lang a language tag, or null
   * @return the item
   */
  public static Item term(String word, String reading, String gloss, String lang) {
    return new Item(new ListItemTerm(word, reading, gloss, lang));
  }

  /**
   * The generated arm this item is.
   *
   * @return the list item
   */
  public ListItem model() {
    return model;
  }
}
