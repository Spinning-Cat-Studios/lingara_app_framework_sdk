// The fixture app (conformance/CONTRACT.md §F), on the kit's public API and
// its node:http adapter only. Built against the built package: tsup rewrites
// `@lingara/apps` to ../../dist/index.mjs.

import { createServer } from "node:http";
import type { AddressInfo } from "node:net";

import { card, item, lingaraApp, nodeHandler, reply } from "@lingara/apps";

const secrets = (process.env["LINGARA_APPS_CONFORMANCE_SECRETS"] ?? "").split(",").filter((s) => s !== "");
const port = Number(process.env["LINGARA_APPS_CONFORMANCE_PORT"] ?? "0");

const app = lingaraApp({
  secret: secrets,
  render(request) {
    const built = card().heading(request.slot, 1);
    for (const slice of request.context) built.text(slice.kind);
    const rendered = built.build();
    return request.slot === "home.side" ? reply(rendered).tutorNote("fixture note") : rendered;
  },
  actions: {
    inc: (request) => card().progress(0.5, "inc").text(request.card_etag).build(),
    boom: () => {
      throw new Error("boom");
    },
    // 21 items: build() refuses the list (list_items), so the core answers 500.
    overflow: () => card().list(Array.from({ length: 21 }, (_, i) => item.text(String(i + 1)))).build(),
    // 24 legal elements whose encoded reply is over 32 768 bytes.
    huge: () => {
      const built = card();
      for (let i = 0; i < 24; i++) built.text("漢".repeat(600));
      return built.build();
    },
  },
});

const server = createServer(nodeHandler(app));
server.listen(port, "127.0.0.1", () => {
  process.stdout.write(`listening ${(server.address() as AddressInfo).port}\n`);
});
