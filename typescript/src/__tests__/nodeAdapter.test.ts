import { createHmac } from "node:crypto";
import { createServer, type Server } from "node:http";
import type { AddressInfo } from "node:net";

import { afterAll, beforeAll, describe, expect, it, vi } from "vitest";

import { card, lingaraApp, nodeHandler, type AppActionRequest, type AppRenderRequest } from "../index.js";

// A visibly fake secret, built at run time: lgr_whsec_ + base64 of 32 ASCII bytes.
const SECRET = `lgr_whsec_${Buffer.from("node-adapter-test-secret-0001!!!").toString("base64")}`;

/** Standard Webhooks: v1,<base64 HMAC-SHA256 over id.timestamp.body>. */
function sign(body: string, id = "lgr_msg_0123456789abcdef0123456789abcdef"): Record<string, string> {
  const timestamp = String(Math.floor(Date.now() / 1000));
  const key = Buffer.from(SECRET.slice("lgr_whsec_".length), "base64");
  const signature = createHmac("sha256", key).update(`${id}.${timestamp}.${body}`).digest("base64");
  return {
    "content-type": "application/json",
    "webhook-id": id,
    "webhook-timestamp": timestamp,
    "webhook-signature": `v1,${signature}`,
  };
}

const COMMON = {
  id: "lgr_msg_0123456789abcdef0123456789abcdef",
  install_id: "7c1d2e3f-4a5b-4c6d-8e7f-9a0b1c2d3e4f",
  subject: "lgr_sub_adapter",
  slot: "home.side",
  locale: "ja",
};

const render = vi.fn((request: AppRenderRequest) => card().heading(request.subject).build());
const inc = vi.fn((request: AppActionRequest) => card().text(request.card_etag).build());
let server: Server;
let url: string;

beforeAll(async () => {
  server = createServer(nodeHandler(lingaraApp({ secret: SECRET, render, actions: { inc } })));
  await new Promise<void>((resolve) => server.listen(0, "127.0.0.1", resolve));
  url = `http://127.0.0.1:${(server.address() as AddressInfo).port}/`;
});

afterAll(() => new Promise<void>((resolve) => server.close(() => resolve())));

const post = (body: string, headers: Record<string, string>): Promise<Response> =>
  fetch(url, { method: "POST", body, headers });

describe("nodeHandler", () => {
  /** 30.9.26am AC8: verify before dispatch; the decoded request reaches the function; card_etag byte for byte. */
  it("the node adapter verifies the raw body before dispatch", async () => {
    vi.spyOn(console, "warn").mockImplementation(() => undefined);
    const context = [
      { kind: "languages", source_lang: "en", target_lang: "ja", level: 3 },
      { kind: "weather", sky: "clear" },
      { kind: "review_due", due: 12, learned: 340 },
    ];
    const body = JSON.stringify({ type: "app.render", ...COMMON, context });

    const tampered = await post(body.replace("lgr_sub_adapter", "lgr_sub_adaptes"), sign(body));
    expect(tampered.status).toBe(401);
    expect(await tampered.text()).toBe("");
    expect(render).not.toHaveBeenCalled();

    const ok = await post(body, sign(body));
    expect(ok.status).toBe(200);
    expect(ok.headers.get("content-type")).toBe("application/json");
    expect(await ok.json()).toEqual({ card: { elements: [{ type: "heading", text: "lgr_sub_adapter", level: 1 }] } });
    expect(render).toHaveBeenCalledTimes(1);
    const received = render.mock.calls[0]![0];
    expect(received.subject).toBe("lgr_sub_adapter");
    expect(received.slot).toBe("home.side");
    expect(received.context).toEqual([context[0], context[2]]);

    const etag = "c1_opaque/+=. \\u00e9 étag";
    const action = JSON.stringify({ type: "app.action", ...COMMON, context: [], action_id: "inc", card_etag: etag });
    const acted = await post(action, sign(action));
    expect(acted.status).toBe(200);
    expect(inc.mock.calls[0]![0].card_etag).toBe(etag);
  });

  it("refuses a non-POST, an oversized body, an unknown action and a raising function", async () => {
    expect((await fetch(url)).status).toBe(405);
    const big = " ".repeat(65_537);
    expect((await post(big, sign(big))).status).toBe(413);

    const unknown = JSON.stringify({ type: "app.action", ...COMMON, context: [], action_id: "nope", card_etag: "c1_x" });
    const refused = await post(unknown, sign(unknown));
    expect(refused.status).toBe(400);
    expect(await refused.text()).toBe('{"error":"bad_request"}');

    const notJson = "{";
    expect((await post(notJson, sign(notJson))).status).toBe(400);

    vi.spyOn(console, "error").mockImplementation(() => undefined);
    inc.mockImplementationOnce(() => {
      throw new Error("boom");
    });
    const action = JSON.stringify({ type: "app.action", ...COMMON, context: [], action_id: "inc", card_etag: "c1_x" });
    const failed = await post(action, sign(action));
    expect(failed.status).toBe(500);
    expect(failed.headers.get("content-type")).toBe("application/json");
    expect(await failed.text()).toBe('{"error":"handler_failed"}');
  });

  it("a malformed secret fails when the app is built, not on the first request", () => {
    expect(() => lingaraApp({ secret: "whsec_nope", render })).toThrow();
  });
});
