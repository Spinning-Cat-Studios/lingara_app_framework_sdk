// The card and reply builders (ADR 30.9.26am D5). A kit refuses what the
// relay would clamp: `build()` runs the card rules and throws
// `CardLimitError` with the first reason, so the clamp never fires on a
// kit-built card. Optional members left unset are omitted, never `null`.

import type {
  ButtonStyle,
  Card,
  CardElement,
  CardElementTerm,
  ListItem,
  ListItemTerm,
  ListItemText,
} from "./generated/apps.js";
import { CardLimitError, cardReason, tutorNoteReason } from "./limits.js";

/** A term's members: `word` required, the rest optional. */
export interface TermFields {
  readonly word: string;
  readonly reading?: string;
  readonly gloss?: string;
  readonly lang?: string;
}

/** Copies only the members that are set, so none is encoded as `null`. */
function defined<T extends object>(fields: T): T {
  return Object.fromEntries(Object.entries(fields).filter(([, v]) => v !== undefined)) as T;
}

function term(fields: TermFields): Omit<CardElementTerm, "type"> {
  return defined({ word: fields.word, reading: fields.reading, gloss: fields.gloss, lang: fields.lang }) as Omit<
    CardElementTerm,
    "type"
  >;
}

/** List items for `CardBuilder.list`. */
export const item = {
  text(text: string, lang?: string): ListItemText {
    return defined({ type: "text", text, lang }) as ListItemText;
  },
  term(fields: TermFields): ListItemTerm {
    return { type: "term", ...term(fields) };
  },
} as const;

/** A fluent card: one method per element, `build()` checks the card rules. */
export class CardBuilder {
  readonly #elements: CardElement[] = [];

  #push(element: CardElement): this {
    this.#elements.push(element);
    return this;
  }

  heading(text: string, level: 1 | 2 = 1): this {
    return this.#push({ type: "heading", text, level });
  }

  text(text: string, lang?: string): this {
    return this.#push(defined({ type: "text", text, lang }) as CardElement);
  }

  term(fields: TermFields): this {
    return this.#push({ type: "term", ...term(fields) });
  }

  list(items: readonly ListItem[]): this {
    return this.#push({ type: "list", items: [...items] });
  }

  progress(value: number, label: string): this {
    return this.#push({ type: "progress", value, label });
  }

  divider(): this {
    return this.#push({ type: "divider" });
  }

  /** `action` comes back as the request's `action_id`. */
  button(label: string, action: string, style?: ButtonStyle): this {
    return this.#push(defined({ type: "button", label, action, style }) as CardElement);
  }

  link(label: string, url: string): this {
    return this.#push({ type: "link", label, url });
  }

  /** The card, or `CardLimitError` naming the first card rule it breaks. */
  build(): Card {
    const card: Card = { elements: [...this.#elements] };
    const reason = cardReason(card);
    if (reason !== undefined) throw new CardLimitError(reason);
    return card;
  }
}

/** Starts a card. */
export function card(): CardBuilder {
  return new CardBuilder();
}

/** A card and, optionally, a tutor note: what a render or action function may return. */
export class Reply {
  readonly card: Card;
  #tutorNote: string | undefined;

  constructor(card: Card) {
    this.card = card;
  }

  /** Plain text, at most 280 characters once `\n` becomes a space and the ends are trimmed. */
  tutorNote(text: string): this {
    const reason = tutorNoteReason(text);
    if (reason !== undefined) throw new CardLimitError(reason);
    this.#tutorNote = text;
    return this;
  }

  /** The wire form, `{card, tutor_note?}`: an absent note is omitted. */
  toJSON(): { card: Card; tutor_note?: string } {
    return this.#tutorNote === undefined ? { card: this.card } : { card: this.card, tutor_note: this.#tutorNote };
  }
}

/** Wraps a built card so a tutor note can ride with it. */
export function reply(card: Card): Reply {
  return new Reply(card);
}
