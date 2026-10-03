import { spawnSync } from "node:child_process";
import { copyFileSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

import { afterEach, describe, expect, it } from "vitest";

const here = (path: string): string => fileURLToPath(new URL(path, import.meta.url));
const SCRIPT = here("../../scripts/codegenApps.mjs");
const VIEW = here("../../../spec/generator/apps.3.1.json");
const SCHEMA = here("../generated/schema.ts");

let dir: string;
afterEach(() => rmSync(dir, { recursive: true, force: true }));

/** Runs the emitter over a copy of the generated schema.ts, plus any extra generated files. */
function emit(extra: Record<string, string> = {}): { status: number | null; stderr: string; out: string } {
  dir = mkdtempSync(join(tmpdir(), "codegen-apps-"));
  copyFileSync(SCHEMA, join(dir, "schema.ts"));
  for (const [name, content] of Object.entries(extra)) writeFileSync(join(dir, name), content);
  const run = spawnSync(process.execPath, [SCRIPT, VIEW, join(dir, "schema.ts"), join(dir, "apps.ts")], { encoding: "utf8" });
  let out = "";
  try {
    out = readFileSync(join(dir, "apps.ts"), "utf8");
  } catch {
    // Refused: nothing written.
  }
  return { status: run.status, stderr: run.stderr, out };
}

describe("codegenApps.mjs", () => {
  /** 30.9.26am AC15: the three unions with S1's arm names, the slice kinds, the operations, and the refusal. */
  it("the emitter writes the three unions and the operations", () => {
    const { status, out } = emit();
    expect(status).toBe(0);
    const arms = {
      CardElement: ["Heading", "Text", "Term", "List", "Progress", "Divider", "Button", "Link"],
      ListItem: ["Text", "Term"],
      ContextSlice: ["Languages", "PlanSummary", "ReviewDue", "TutorTopic"],
    };
    for (const [union, names] of Object.entries(arms)) {
      expect(out).toContain(`export type ${union} =\n${names.map((n) => `  | ${union}${n}`).join("\n")};`);
      for (const n of names) expect(out).toMatch(new RegExp(`^export type ${union}${n} = `, "m"));
    }
    expect(out).toContain('export const CONTEXT_SLICE_KINDS = ["languages", "plan_summary", "review_due", "tutor_topic"] as const;');
    expect(out).toContain('"app.render": { operation: "sendAppRender"');
    expect(out).toContain('"app.action": { operation: "sendAppAction"');
    expect(out).toContain('export const APP_REPLY_MESSAGE = "app.card";');
    expect(out).toContain('type: "app.render"; context: ContextSlice[]');
    expect(out).toContain("elements: CardElement[]");
    expect(out).toContain("items: ListItem[]");

    // The committed file is this output, byte for byte.
    expect(readFileSync(here("../generated/apps.ts"), "utf8")).toBe(out);

    // A generator-written file named after a union, or an export of one, is refused.
    const named = emit({ "CardElement.ts": "export {};\n" });
    expect(named.status).not.toBe(0);
    expect(named.stderr).toContain("CardElement.ts");
    const exported = emit({ "schema.ts": 'export type ContextSlice = { kind: "x" };\n' });
    expect(exported.status).not.toBe(0);
    expect(exported.stderr).toContain("ContextSlice");
  });
});
