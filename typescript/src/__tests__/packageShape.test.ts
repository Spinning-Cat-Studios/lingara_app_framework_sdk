import { readFileSync } from "node:fs";

import { describe, expect, it } from "vitest";

const read = (path: string): string => readFileSync(new URL(path, import.meta.url), "utf8");
const pkg = JSON.parse(read("../../package.json")) as Record<string, unknown>;
const floor = read("../../../LIBRARY_FLOOR").trim();

describe("package.json", () => {
  /** 30.9.26am AC22: @lingara/api at the library floor, and nothing else at run time. */
  it("the only runtime dependency is the library", () => {
    expect(pkg["dependencies"]).toEqual({ "@lingara/api": `^${floor}` });
    for (const key of ["peerDependencies", "optionalDependencies", "bundleDependencies", "bundledDependencies"]) {
      expect(pkg[key], key).toBeUndefined();
    }
  });

  it("is dual ESM/CJS, ships dist alone, and asks for the library's Node floor", () => {
    expect(pkg["name"]).toBe("@lingara/apps");
    expect(pkg["version"]).toBe(read("../../../VERSION").trim());
    expect(pkg["files"]).toEqual(["dist"]);
    expect(pkg["exports"]).toEqual({
      ".": {
        import: { types: "./dist/index.d.ts", default: "./dist/index.mjs" },
        require: { types: "./dist/index.d.cts", default: "./dist/index.cjs" },
      },
      "./package.json": "./package.json",
    });
    const library = JSON.parse(read("../../node_modules/@lingara/api/package.json")) as { engines: unknown };
    expect(pkg["engines"]).toEqual(library.engines);
  });

  // Importing the config loads ESLint and typescript-eslint, slow on a cold cache.
  it("the lint budgets are the portfolio's numbers", { timeout: 60_000 }, async () => {
    const config = new URL("../../eslint.config.js", import.meta.url).href;
    const { BUDGETS } = (await import(config)) as { BUDGETS: unknown };
    expect(BUDGETS).toEqual({ warnLines: 300, maxLines: 600, maxLinesPerFunction: 50, complexity: 10, maxParams: 5 });
  });
});
