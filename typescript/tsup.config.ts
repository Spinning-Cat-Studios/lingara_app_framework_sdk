import { defineConfig } from "tsup";

export default defineConfig([
  // The package: ESM and CJS, each with its own declaration file, as
  // @lingara/api ships. `@lingara/api` stays an import (a dependency).
  {
    entry: { index: "src/index.ts" },
    format: ["esm", "cjs"],
    dts: true,
    target: "es2022",
    platform: "neutral",
    clean: true,
    sourcemap: false,
    external: ["@lingara/api"],
    outExtension: ({ format }) => ({ js: format === "esm" ? ".mjs" : ".cjs" }),
  },
  // The conformance fixture app, built against the built package rather
  // than `src/`, so conformance tests the artefact that ships.
  {
    entry: { fixture: "conformance/fixture.ts" },
    outDir: "conformance/dist",
    format: ["esm"],
    target: "es2022",
    platform: "node",
    clean: true,
    outExtension: () => ({ js: ".mjs" }),
    esbuildPlugins: [
      {
        name: "lingara-apps-alias",
        setup(build) {
          build.onResolve({ filter: /^@lingara\/apps$/ }, () => ({ path: "../../dist/index.mjs", external: true }));
        },
      },
    ],
  },
]);
