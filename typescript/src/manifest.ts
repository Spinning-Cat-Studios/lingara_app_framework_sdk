// The manifest builder (ADR 30.9.26am D5), hand-written against the
// manifest's wire form, which is in no spec the view reads. Its closed
// values still come from the view: `slots` from the generated AppSlotName,
// `context` from the ContextSlice kinds the emitter writes.
//
// `validateManifest` refuses what the upload would refuse or normalise away
// where a kit can know it, naming the first broken rule in the order
// `conformance/vectors/manifest.json` describes.

import { APP_SLOT_NAMES, CONTEXT_SLICE_KINDS, type AppSlotName, type ContextSliceKind } from "./generated/apps.js";
import { hasControl, scalarCount, trimWhiteSpace } from "./limits.js";

export const MANIFEST_NAME_MAX_CHARS = 40;
export const MANIFEST_DESCRIPTION_MAX_CHARS = 280;
export const MANIFEST_STORED_MAX_BYTES = 65_536;

export const MANIFEST_RULES = [
  "manifest_version",
  "default_locale",
  "name",
  "description",
  "render_url",
  "slots",
  "context",
  "duplicate",
  "too_large",
] as const;
export type ManifestRule = (typeof MANIFEST_RULES)[number];

/** A manifest the upload would refuse, with the first rule it breaks. */
export class ManifestError extends Error {
  override readonly name = "ManifestError";
  readonly rule: ManifestRule;

  constructor(rule: ManifestRule) {
    super(`the manifest breaks the ${rule} rule`);
    this.rule = rule;
  }
}

/** The manifest's wire form. */
export interface AppManifest {
  readonly manifest_version: 1;
  readonly default_locale: string;
  readonly name: Readonly<Record<string, string>>;
  readonly description: Readonly<Record<string, string>>;
  readonly render_url: string;
  readonly slots: readonly AppSlotName[];
  readonly context: readonly ContextSliceKind[];
  readonly scopes: readonly string[];
  readonly tutor_note: boolean;
  readonly listed: boolean;
}

type Json = Record<string, unknown>;
const isObject = (v: unknown): v is Json => typeof v === "object" && v !== null && !Array.isArray(v);
// Zero-width characters, beyond the card rules' controls and bidi marks.
const ZERO_WIDTH = /[\u200B-\u200F\uFEFF]/;

function localized(value: unknown, max: number): boolean {
  if (!isObject(value) || Object.keys(value).length === 0) return false;
  return Object.values(value).every((v) => {
    if (typeof v !== "string") return false;
    const trimmed = trimWhiteSpace(v);
    const n = scalarCount(trimmed);
    return n >= 1 && n <= max && !hasControl(trimmed, false) && !ZERO_WIDTH.test(trimmed);
  });
}

function localeIn(locale: string, map: unknown): boolean {
  return !isObject(map) || Object.hasOwn(map, locale);
}

function defaultLocaleOk(m: Json): boolean {
  const locale = m["default_locale"];
  return typeof locale === "string" && locale !== "" && localeIn(locale, m["name"]) && localeIn(locale, m["description"]);
}

function within(value: unknown, allowed: readonly string[], optional: boolean): boolean {
  if (value === undefined) return optional;
  if (!Array.isArray(value) || (!optional && value.length === 0)) return false;
  return value.every((v) => typeof v === "string" && allowed.includes(v));
}

function hasDuplicate(value: unknown): boolean {
  if (!Array.isArray(value)) return false;
  const seen = value.map((v) => JSON.stringify(v));
  return new Set(seen).size !== seen.length;
}

function storedBytes(m: Json): number {
  const stored = {
    default_locale: m["default_locale"],
    name: m["name"],
    description: m["description"],
    slots: m["slots"],
    context: m["context"] ?? [],
    scopes: m["scopes"] ?? [],
    tutor_note: m["tutor_note"] ?? false,
  };
  return new TextEncoder().encode(JSON.stringify(stored)).length;
}

const RENDER_URL = /^https:\/\/[^/?#]/i;

/** Each rule, in order, as a predicate that holds when the rule is kept. */
const CHECKS: readonly (readonly [ManifestRule, (m: Json) => boolean])[] = [
  ["manifest_version", (m) => m["manifest_version"] === 1],
  ["default_locale", defaultLocaleOk],
  ["name", (m) => localized(m["name"], MANIFEST_NAME_MAX_CHARS)],
  ["description", (m) => localized(m["description"], MANIFEST_DESCRIPTION_MAX_CHARS)],
  ["render_url", (m) => typeof m["render_url"] === "string" && RENDER_URL.test(m["render_url"])],
  ["slots", (m) => within(m["slots"], APP_SLOT_NAMES, false)],
  ["context", (m) => within(m["context"], CONTEXT_SLICE_KINDS, true)],
  ["duplicate", (m) => !["slots", "context", "scopes"].some((k) => hasDuplicate(m[k]))],
  ["too_large", (m) => storedBytes(m) <= MANIFEST_STORED_MAX_BYTES],
];

/** The first rule `manifest` breaks, or `undefined`. Accepts any JSON value. */
export function manifestRule(manifest: unknown): ManifestRule | undefined {
  const m = isObject(manifest) ? manifest : {};
  return CHECKS.find(([, kept]) => !kept(m))?.[0];
}

/** Throws `ManifestError` naming the first rule `manifest` breaks. */
export function validateManifest(manifest: unknown): void {
  const rule = manifestRule(manifest);
  if (rule !== undefined) throw new ManifestError(rule);
}

/** A name or description: a locale map, or a bare string meaning the default locale's. */
export type Localized = string | Readonly<Record<string, string>>;

/** A fluent manifest. `build()` validates; `toJson()` writes the upload. */
export class ManifestBuilder {
  #defaultLocale = "";
  #name: Localized = "";
  #description: Localized = "";
  #renderUrl = "";
  #slots: AppSlotName[] = [];
  #context: ContextSliceKind[] = [];
  #scopes: string[] = [];
  #tutorNote = false;
  #listed = false;

  defaultLocale(locale: string): this {
    this.#defaultLocale = locale;
    return this;
  }

  name(name: Localized): this {
    this.#name = name;
    return this;
  }

  description(description: Localized): this {
    this.#description = description;
    return this;
  }

  renderUrl(url: string): this {
    this.#renderUrl = url;
    return this;
  }

  slots(...slots: AppSlotName[]): this {
    this.#slots = slots;
    return this;
  }

  context(...kinds: ContextSliceKind[]): this {
    this.#context = kinds;
    return this;
  }

  scopes(...scopes: string[]): this {
    this.#scopes = scopes;
    return this;
  }

  /** Whether the app's replies may carry a tutor note. */
  tutorNote(enabled = true): this {
    this.#tutorNote = enabled;
    return this;
  }

  /** Whether the app appears in the catalogue; `false` unless set. */
  listed(listed = true): this {
    this.#listed = listed;
    return this;
  }

  #localized(value: Localized): Readonly<Record<string, string>> {
    return typeof value === "string" ? { [this.#defaultLocale]: value } : value;
  }

  /** The manifest, or `ManifestError` naming the first rule it breaks. */
  build(): AppManifest {
    const manifest: AppManifest = {
      manifest_version: 1,
      default_locale: this.#defaultLocale,
      name: this.#localized(this.#name),
      description: this.#localized(this.#description),
      render_url: this.#renderUrl,
      slots: [...this.#slots],
      context: [...this.#context],
      scopes: [...this.#scopes],
      tutor_note: this.#tutorNote,
      listed: this.#listed,
    };
    validateManifest(manifest);
    return manifest;
  }

  /** The validated manifest as the JSON the console accepts as an upload. */
  toJson(): string {
    return `${JSON.stringify(this.build(), null, 2)}\n`;
  }
}

/** Starts a manifest. */
export function manifest(): ManifestBuilder {
  return new ManifestBuilder();
}
