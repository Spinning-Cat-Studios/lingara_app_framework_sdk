// The `node:http` adapter (ADR 30.9.26am D2): a request listener over the
// core. It reads the raw body as bytes, and never more than the core's limit
// plus one, so an oversized body is refused without being buffered.

import type { IncomingMessage, ServerResponse } from "node:http";

import { handle, type LingaraApp } from "./core.js";

/** The first `limit` bytes of `chunks`, as one array. */
function concat(chunks: readonly Uint8Array[], limit: number): Uint8Array {
  const body = new Uint8Array(Math.min(chunks.reduce((n, c) => n + c.length, 0), limit));
  let offset = 0;
  for (const chunk of chunks) {
    const part = chunk.subarray(0, body.length - offset);
    body.set(part, offset);
    offset += part.length;
  }
  return body;
}

/**
 * Reads at most `limit` bytes of `req`, then stops reading. The stream is
 * paused, never destroyed, so the reply can still be written.
 */
function readBody(req: IncomingMessage, limit: number): Promise<Uint8Array> {
  const declared = Number(req.headers["content-length"]);
  // Lingara always sends Content-Length, so a declared oversize is refused unread.
  if (Number.isFinite(declared) && declared >= limit) return Promise.resolve(new Uint8Array(limit));
  return new Promise((resolve, reject) => {
    const chunks: Uint8Array[] = [];
    let length = 0;
    const done = (): void => {
      req.off("data", onData).off("end", done).off("error", reject);
      resolve(concat(chunks, limit));
    };
    const onData = (chunk: Uint8Array): void => {
      chunks.push(chunk);
      length += chunk.length;
      if (length >= limit) {
        req.pause();
        done();
      }
    };
    req.on("data", onData).on("end", done).on("error", reject);
  });
}

/**
 * A `node:http` request listener for `app`:
 * `createServer(nodeHandler(app)).listen(8787)`. Mount it on a route whose
 * body nothing has parsed yet: the signature covers the raw bytes.
 */
export function nodeHandler(app: LingaraApp): (req: IncomingMessage, res: ServerResponse) => void {
  return (req, res) => {
    const request = { method: req.method ?? "", headers: req.headers, body: (limit: number) => readBody(req, limit) };
    handle(app, request)
      .then((reply) => {
        res.writeHead(reply.status, { ...reply.headers, "content-length": String(reply.body.length) });
        res.end(reply.body);
      })
      .catch(() => {
        // The core answers every failure itself; this is a broken socket.
        if (!res.headersSent) res.writeHead(500);
        res.end();
      })
      // A body the core did not read (405, 413) is discarded, not buffered.
      .finally(() => req.resume());
  };
}
