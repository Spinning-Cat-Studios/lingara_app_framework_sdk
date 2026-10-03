// The card rules (ADR 30.9.26am D5): the relay's clamp restated as
// refusals, held to `conformance/vectors/card-limits.json` and to the
// contract's reference validator. Every limit is a constant here and nowhere
// else, because the view carries none of them: they are post-parse clamps.
//
// A "character" is a Unicode scalar value (`for…of`), never a UTF-16 unit or
// a grapheme. "Empty" is empty after trimming Unicode White_Space, which is
// not what `String.prototype.trim` trims (it trims U+FEFF and keeps U+0085).

/** The largest reply body the relay reads, in bytes. */
export const REPLY_MAX_BYTES = 32_768;
/** The largest request body a kit reads, in bytes. */
export const REQUEST_MAX_BYTES = 65_536;
export const MAX_ELEMENTS = 24;
export const MAX_BUTTONS = 4;
export const MAX_LIST_ITEMS = 20;
export const TUTOR_NOTE_MAX_CHARS = 280;
export const MAX_URL_BYTES = 2048;

export const HEADING_MAX_CHARS = 80;
export const TEXT_MAX_CHARS = 600;
export const TERM_WORD_MAX_CHARS = 60;
export const TERM_READING_MAX_CHARS = 120;
export const TERM_GLOSS_MAX_CHARS = 160;
export const PROGRESS_LABEL_MAX_CHARS = 60;
export const BUTTON_LABEL_MAX_CHARS = 32;
export const LINK_LABEL_MAX_CHARS = 60;

export const LANG_PATTERN = /^[A-Za-z]{2,3}(-[A-Za-z0-9]{2,8}){0,3}$/;
export const ACTION_PATTERN = /^[A-Za-z0-9_.:-]{1,64}$/;

/** The refusal reasons, in the contract's order: a reply's reason is the first it breaks. */
export const REASONS = [
  "control_chars",
  "text_length",
  "heading_level",
  "progress_range",
  "lang",
  "link",
  "button_action",
  "buttons",
  "list_items",
  "empty_element",
  "elements",
  "empty_card",
  "tutor_note_length",
  "reply_too_large",
] as const;
export type CardLimitReason = (typeof REASONS)[number];

/** A card or reply the relay would clamp, refused with the first rule it breaks. */
export class CardLimitError extends Error {
  override readonly name = "CardLimitError";
  readonly reason: CardLimitReason;

  constructor(reason: CardLimitReason) {
    super(`the card breaks the ${reason} rule`);
    this.reason = reason;
  }
}

// Unicode White_Space, which Rust's `str::trim` trims.
const WHITE_SPACE = "[\\u0009-\\u000D\\u0020\\u0085\\u00A0\\u1680\\u2000-\\u200A\\u2028\\u2029\\u202F\\u205F\\u3000]";
const EDGES = new RegExp(`^${WHITE_SPACE}+|${WHITE_SPACE}+$`, "g");

/** Trims Unicode White_Space from both ends. */
export function trimWhiteSpace(text: string): string {
  return text.replace(EDGES, "");
}

/** The length in Unicode scalar values. */
export function scalarCount(text: string): number {
  return [...text].length;
}

/** C0 and C1 controls, and the bidi overrides and isolates. */
export function isStripped(c: string): boolean {
  const cp = c.codePointAt(0) ?? 0;
  return cp <= 0x1f || (cp >= 0x7f && cp <= 0x9f) || (cp >= 0x202a && cp <= 0x202e) || (cp >= 0x2066 && cp <= 0x2069);
}

/** Whether `text` holds a stripped character, `\n` aside when `newline` allows it. */
export function hasControl(text: string, newline: boolean): boolean {
  for (const c of text) if (isStripped(c) && !(newline && c === "\n")) return true;
  return false;
}

/**
 * The relay's cut, never applied implicitly: over `limit` scalar values, the
 * first `limit − 1` and `…`; otherwise unchanged.
 */
export function truncate(text: string, limit: number): string {
  const chars = [...text];
  if (chars.length <= limit) return text;
  return `${chars.slice(0, Math.max(limit - 1, 0)).join("")}…`;
}

/** `https`, no userinfo, a named host (never an IP literal), at most MAX_URL_BYTES. */
export function isSafeLink(raw: string): boolean {
  if (new TextEncoder().encode(raw).length > MAX_URL_BYTES) return false;
  let url: URL;
  try {
    url = new URL(raw);
  } catch {
    return false;
  }
  // WHATWG URL parsing (as the relay's) leaves an IPv4 host dotted-decimal
  // and an IPv6 host bracketed.
  const ip = url.hostname.startsWith("[") || /^\d+\.\d+\.\d+\.\d+$/.test(url.hostname);
  return url.protocol === "https:" && url.username === "" && url.password === "" && url.hostname !== "" && !ip;
}

/** One text field's rule: its limit, whether `\n` is legal, whether it must be non-empty. */
interface Field {
  readonly limit: number;
  readonly newline: boolean;
  readonly required: boolean;
}

const field = (limit: number, newline: boolean, required: boolean): Field => ({ limit, newline, required });
const HEADING = field(HEADING_MAX_CHARS, false, true);
const TEXT = field(TEXT_MAX_CHARS, true, true);
const WORD = field(TERM_WORD_MAX_CHARS, false, true);
const READING = field(TERM_READING_MAX_CHARS, false, false);
const GLOSS = field(TERM_GLOSS_MAX_CHARS, false, false);
const PROGRESS_LABEL = field(PROGRESS_LABEL_MAX_CHARS, false, true);
const BUTTON_LABEL = field(BUTTON_LABEL_MAX_CHARS, false, true);
const LINK_LABEL = field(LINK_LABEL_MAX_CHARS, false, true);
/** `lang`, `button.action` and `link.url`: no length limit of their own. */
const RAW = field(Number.POSITIVE_INFINITY, false, false);

type Json = Record<string, unknown>;
const asObject = (v: unknown): Json => (typeof v === "object" && v !== null && !Array.isArray(v) ? (v as Json) : {});
const present = (v: unknown): unknown => (v === null ? undefined : v);

/** The rules a reply breaks, collected; the reason is the first in REASONS order. */
class Found {
  readonly #broken = new Set<CardLimitReason>();

  when(reason: CardLimitReason, broken: boolean): void {
    if (broken) this.#broken.add(reason);
  }

  first(): CardLimitReason | undefined {
    return REASONS.find((r) => this.#broken.has(r));
  }

  text(value: unknown, rule: Field): string | undefined {
    if (typeof value !== "string") {
      this.when("empty_element", rule.required);
      return undefined;
    }
    this.when("control_chars", hasControl(value, rule.newline));
    this.when("text_length", scalarCount(value) > rule.limit);
    this.when("empty_element", rule.required && trimWhiteSpace(value) === "");
    return value;
  }

  lang(value: unknown): void {
    const tag = this.text(present(value), RAW);
    if (tag !== undefined) this.when("lang", !LANG_PATTERN.test(tag));
  }

  item(item: Json): void {
    if (item["type"] === "text") {
      this.text(item["text"], TEXT);
      this.lang(item["lang"]);
    } else if (item["type"] === "term") {
      this.text(item["word"], WORD);
      this.text(present(item["reading"]), READING);
      this.text(present(item["gloss"]), GLOSS);
      this.lang(item["lang"]);
    }
  }

  element(e: Json): void {
    switch (e["type"]) {
      case "heading":
        this.text(e["text"], HEADING);
        this.when("heading_level", e["level"] !== 1 && e["level"] !== 2);
        return;
      case "text":
      case "term":
        return this.item(e);
      case "list":
        return this.list(e);
      case "progress":
        this.when("progress_range", !inUnitRange(e["value"]));
        this.text(e["label"], PROGRESS_LABEL);
        return;
      case "button":
        return this.button(e);
      case "link":
        return this.link(e);
    }
  }

  button(e: Json): void {
    this.text(e["label"], BUTTON_LABEL);
    this.when("button_action", !ACTION_PATTERN.test(this.text(e["action"], RAW) ?? ""));
  }

  link(e: Json): void {
    this.text(e["label"], LINK_LABEL);
    this.when("link", !isSafeLink(this.text(e["url"], RAW) ?? ""));
  }

  list(e: Json): void {
    const items = Array.isArray(e["items"]) ? (e["items"] as unknown[]) : [];
    items.forEach((i) => this.item(asObject(i)));
    this.when("list_items", items.length === 0 || items.length > MAX_LIST_ITEMS);
  }

  card(card: unknown): void {
    const raw = asObject(card)["elements"];
    const elements = Array.isArray(raw) ? (raw as unknown[]).map(asObject) : [];
    elements.forEach((e) => this.element(e));
    this.when("buttons", elements.filter((e) => e["type"] === "button").length > MAX_BUTTONS);
    this.when("elements", elements.length > MAX_ELEMENTS);
    this.when("empty_card", elements.length === 0);
  }

  tutorNote(note: unknown): void {
    if (typeof note !== "string") return;
    this.when("control_chars", hasControl(note, true));
    // The relay turns `\n` into a space and trims before it counts.
    this.when("tutor_note_length", scalarCount(trimWhiteSpace(note.replaceAll("\n", " "))) > TUTOR_NOTE_MAX_CHARS);
  }
}

const inUnitRange = (v: unknown): boolean => typeof v === "number" && v >= 0 && v <= 1;

/** The result of validating a reply: the bytes to send, or the first reason it breaks. */
export type Validated = { readonly ok: true; readonly bytes: Uint8Array } | { readonly ok: false; readonly reason: CardLimitReason };

/**
 * Every card rule, the tutor note, and the size of the reply encoded exactly
 * as it will be sent: compact JSON, raw UTF-8, `/` unescaped. On success it
 * returns those bytes. Accepts any JSON value, so a vector feeds it as is.
 */
export function validateReply(reply: unknown): Validated {
  const found = new Found();
  found.card(asObject(reply)["card"]);
  found.tutorNote(asObject(reply)["tutor_note"]);
  const bytes = new TextEncoder().encode(JSON.stringify(reply));
  found.when("reply_too_large", bytes.length > REPLY_MAX_BYTES);
  const reason = found.first();
  return reason === undefined ? { ok: true, bytes } : { ok: false, reason };
}

/** The card rules alone (reasons 1–12): what `build()` runs. */
export function cardReason(card: unknown): CardLimitReason | undefined {
  const found = new Found();
  found.card(card);
  return found.first();
}

/** The tutor note's rules alone: what `reply(card).tutorNote(text)` runs. */
export function tutorNoteReason(note: string): CardLimitReason | undefined {
  const found = new Found();
  found.tutorNote(note);
  return found.first();
}
