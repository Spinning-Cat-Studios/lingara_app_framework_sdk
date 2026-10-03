import { readFileSync } from "node:fs";

import { describe, expect, it } from "vitest";

import { CardLimitError, ManifestError, card, item, manifest, manifestRule, reply, truncate, validateReply } from "../index.js";

interface Vector {
  readonly name: string;
  readonly expect: { readonly ok: true } | { readonly refused: string };
}
type CardVector = Vector & { readonly reply: unknown };
type ManifestVector = Vector & { readonly manifest: unknown };

const load = <T>(file: string): T[] =>
  (JSON.parse(readFileSync(new URL(`../../../conformance/vectors/${file}`, import.meta.url), "utf8")) as { vectors: T[] })
    .vectors;

const cardAnswer = (reply: unknown): object => {
  const validated = validateReply(reply);
  return validated.ok ? { ok: true } : { refused: validated.reason };
};

const manifestAnswer = (manifest: unknown): object => {
  const rule = manifestRule(manifest);
  return rule === undefined ? { ok: true } : { refused: rule };
};

describe("vectors", () => {
  /** 30.9.26am AC1: every card-limits.json and manifest.json vector, and truncate's astral boundary. */
  it("every card and manifest vector gives its expected answer", () => {
    const cards = load<CardVector>("card-limits.json");
    const manifests = load<ManifestVector>("manifest.json");
    expect(cards.length).toBeGreaterThan(0);
    expect(manifests.length).toBeGreaterThan(0);
    for (const v of cards) expect(cardAnswer(v.reply), v.name).toEqual(v.expect);
    for (const v of manifests) expect(manifestAnswer(v.manifest), v.name).toEqual(v.expect);

    const astral = "𝄞".repeat(81);
    const cut = truncate(astral, 80);
    expect([...cut].length).toBe(80);
    expect(cut).toBe(`${"𝄞".repeat(79)}…`);
    expect(truncate("𝄞".repeat(80), 80)).toBe("𝄞".repeat(80));
  });

  it("an ok reply is encoded exactly as sent: compact, raw UTF-8, slashes unescaped, no null note", () => {
    const validated = validateReply({ card: { elements: [{ type: "link", label: "漢", url: "https://a.example/b" }] } });
    expect(validated.ok).toBe(true);
    if (validated.ok) {
      expect(new TextDecoder().decode(validated.bytes)).toBe(
        '{"card":{"elements":[{"type":"link","label":"漢","url":"https://a.example/b"}]}}',
      );
    }
  });
});

describe("builders", () => {
  it("build() refuses what the relay would clamp, naming the reason", () => {
    const overflow = card().list(Array.from({ length: 21 }, (_, i) => item.text(String(i + 1))));
    expect(() => overflow.build()).toThrow(CardLimitError);
    expect(() => card().button("Next", "next kanji").build()).toThrow(expect.objectContaining({ reason: "button_action" }));
    expect(() => card().build()).toThrow(expect.objectContaining({ reason: "empty_card" }));
    expect(() => reply(card().heading("Hi").build()).tutorNote("a".repeat(281))).toThrow(
      expect.objectContaining({ reason: "tutor_note_length" }),
    );
  });

  it("builders omit unset optional members and write the eight element types", () => {
    const built = card()
      .heading("Today", 1)
      .text("Practise.")
      .term({ word: "雨", reading: "yǔ", gloss: "rain", lang: "zh" })
      .list([item.text("one"), item.term({ word: "二" })])
      .progress(0.4, "2 of 5")
      .divider()
      .button("Next", "next", "primary")
      .link("Why?", "https://apps.example.com/why")
      .build();
    expect(built.elements.map((e) => e.type)).toEqual(["heading", "text", "term", "list", "progress", "divider", "button", "link"]);
    expect(built.elements[1]).toEqual({ type: "text", text: "Practise." });
    expect(JSON.stringify(reply(built))).not.toContain("tutor_note");
    expect(JSON.stringify(reply(built).tutorNote("hi"))).toContain('"tutor_note":"hi"');
  });

  it("the manifest builder writes the wire form, a bare string meaning the default locale", () => {
    const built = manifest()
      .defaultLocale("en")
      .name("Daily five")
      .description("Five words to review.")
      .renderUrl("https://apps.example.com/lingara/render")
      .slots("home.side")
      .context("languages", "plan_summary")
      .build();
    expect(built).toEqual({
      manifest_version: 1,
      default_locale: "en",
      name: { en: "Daily five" },
      description: { en: "Five words to review." },
      render_url: "https://apps.example.com/lingara/render",
      slots: ["home.side"],
      context: ["languages", "plan_summary"],
      scopes: [],
      tutor_note: false,
      listed: false,
    });
    expect(() => manifest().defaultLocale("en").name("x").description("y").renderUrl("http://a").slots("home.side").build()).toThrow(
      ManifestError,
    );
  });
});
