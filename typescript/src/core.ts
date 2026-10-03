// The handler core (ADR 30.9.26am D4), framework-neutral: method, size,
// signature, lenient decode, dispatch, validate, encode. An adapter supplies
// the method, the headers and a body reader, and writes back what `handle`
// returns. The signature check is the library's (`verifySignature`), never
// the kit's own.

import { Webhook, type WebhookHeaders } from "@lingara/api";

import { Reply } from "./card.js";
import {
  APP_OPERATIONS,
  APP_SLOT_NAMES,
  CONTEXT_SLICE_REQUIRED,
  type AppActionRequest,
  type AppOperation,
  type AppRenderRequest,
  type Card,
  type ContextSlice,
} from "./generated/apps.js";
import { CardLimitError, REQUEST_MAX_BYTES, validateReply } from "./limits.js";

/** What a render or action function returns: a card, or a reply carrying a card and a tutor note. */
export type AppReply = Card | Reply;
export type RenderFn = (request: AppRenderRequest) => AppReply | Promise<AppReply>;
export type ActionFn = (request: AppActionRequest) => AppReply | Promise<AppReply>;

export interface LingaraAppOptions {
  /** The app's signing secret (`lgr_whsec_…`), or several during a rotation. */
  readonly secret: string | readonly string[];
  readonly render: RenderFn;
  /** One function per button `action`, keyed on the `action_id` it comes back as. */
  readonly actions?: Readonly<Record<string, ActionFn>>;
}

/** An app: its verifier, built once, and its functions. */
export interface LingaraApp {
  readonly webhook: Webhook;
  readonly render: RenderFn;
  readonly actions: ReadonlyMap<string, ActionFn>;
}

/**
 * Builds an app. The library's `Webhook` is constructed here, once, so a
 * malformed secret fails at startup rather than on the first request.
 */
export function lingaraApp(options: LingaraAppOptions): LingaraApp {
  return {
    webhook: new Webhook(options.secret),
    render: options.render,
    actions: new Map(Object.entries(options.actions ?? {})),
  };
}

/** What an adapter hands the core. `body` reads at most `limit` bytes and may stop there. */
export interface CoreRequest {
  readonly method: string;
  readonly headers: WebhookHeaders;
  readonly body: (limit: number) => Promise<Uint8Array>;
}

/** What the core answers; an adapter writes it as is. */
export interface CoreResponse {
  readonly status: number;
  readonly headers: Readonly<Record<string, string>>;
  readonly body: Uint8Array;
}

const JSON_TYPE = { "content-type": "application/json" } as const;
const encoder = new TextEncoder();
const empty = (status: number): CoreResponse => ({ status, headers: {}, body: new Uint8Array() });
const error = (status: number, code: string): CoreResponse => ({
  status,
  headers: JSON_TYPE,
  body: encoder.encode(JSON.stringify({ error: code })),
});
export const BAD_REQUEST = error(400, "bad_request");
export const HANDLER_FAILED = error(500, "handler_failed");

export type AppRequest = AppRenderRequest | AppActionRequest;

type Json = Record<string, unknown>;
const isObject = (v: unknown): v is Json => typeof v === "object" && v !== null && !Array.isArray(v);
const COMMON = ["id", "install_id", "subject", "locale"] as const;
const ACTION = ["action_id", "card_etag"] as const;

/**
 * One context slice: a known kind with its required members, `null` for an
 * unknown kind (skipped, AK2's forward compatibility), `undefined` when it
 * does not decode. Unknown members are kept and ignored.
 */
function decodeSlice(raw: unknown): ContextSlice | null | undefined {
  if (!isObject(raw) || typeof raw["kind"] !== "string") return undefined;
  if (!Object.hasOwn(CONTEXT_SLICE_REQUIRED, raw["kind"])) {
    console.warn(`lingara-apps: skipped a context slice of unknown kind ${JSON.stringify(raw["kind"].slice(0, 64))}`);
    return null;
  }
  const required = CONTEXT_SLICE_REQUIRED[raw["kind"] as ContextSlice["kind"]];
  return required.every((k) => raw[k] !== undefined && raw[k] !== null) ? (raw as ContextSlice) : undefined;
}

/** The context, element by element, before anything reads it as the union. */
function decodeContext(raw: unknown): ContextSlice[] | undefined {
  if (!Array.isArray(raw)) return undefined;
  const slices: ContextSlice[] = [];
  for (const element of raw) {
    const slice = decodeSlice(element);
    if (slice === undefined) return undefined;
    if (slice !== null) slices.push(slice);
  }
  return slices;
}

function parse(body: Uint8Array): Json | undefined {
  try {
    const value: unknown = JSON.parse(new TextDecoder("utf-8", { fatal: true }).decode(body));
    return isObject(value) ? value : undefined;
  } catch {
    return undefined;
  }
}

/** The request, decoded leniently into the generated type, or `undefined` (a `400`). */
export function decodeRequest(body: Uint8Array): AppRequest | undefined {
  const raw = parse(body);
  const type = raw?.["type"];
  if (raw === undefined || typeof type !== "string" || !Object.hasOwn(APP_OPERATIONS, type)) return undefined;
  const fields = type === "app.action" ? [...COMMON, ...ACTION] : COMMON;
  if (!fields.every((k) => typeof raw[k] === "string")) return undefined;
  if (!(APP_SLOT_NAMES as readonly unknown[]).includes(raw["slot"])) return undefined;
  const context = decodeContext(raw["context"]);
  return context === undefined ? undefined : ({ ...raw, context } as AppRequest);
}

function log(operation: AppOperation, installId: string, reason: string): void {
  // The operation, the install and the reason: never the body, a secret or a signature.
  console.error(`lingara-apps: ${operation} for install ${installId} failed: ${reason}`);
}

function reasonOf(e: unknown): string {
  if (e instanceof CardLimitError) return `CardLimitError ${e.reason}`;
  return e instanceof Error ? `raised ${e.name}: ${e.message}` : "raised a non-Error value";
}

/** The developer's function for a request, or `undefined` for an unregistered action. */
function route(app: LingaraApp, request: AppRequest): (() => AppReply | Promise<AppReply>) | undefined {
  const operation: AppOperation = request.type;
  switch (operation) {
    case "app.render":
      return () => app.render(request as AppRenderRequest);
    case "app.action": {
      const fn = app.actions.get((request as AppActionRequest).action_id);
      return fn && (() => fn(request as AppActionRequest));
    }
    default: {
      // A third operation in the view fails the build here until it is handled.
      const unhandled: never = operation;
      return unhandled;
    }
  }
}

async function answer(request: AppRequest, fn: () => AppReply | Promise<AppReply>): Promise<CoreResponse> {
  try {
    const result = await fn();
    const wire = result instanceof Reply ? result.toJSON() : { card: result };
    const validated = validateReply(wire);
    if (validated.ok) return { status: 200, headers: JSON_TYPE, body: validated.bytes };
    log(request.type, request.install_id, `reply refused: ${validated.reason}`);
  } catch (e) {
    log(request.type, request.install_id, reasonOf(e));
  }
  return HANDLER_FAILED;
}

/** One request, start to finish (AK1–AK4). */
export async function handle(app: LingaraApp, request: CoreRequest): Promise<CoreResponse> {
  if (request.method !== "POST") return empty(405);
  const body = await request.body(REQUEST_MAX_BYTES + 1);
  if (body.length > REQUEST_MAX_BYTES) return empty(413);
  try {
    await app.webhook.verifySignature(body, request.headers);
  } catch {
    return empty(401);
  }
  const decoded = decodeRequest(body);
  if (decoded === undefined) return BAD_REQUEST;
  const fn = route(app, decoded);
  return fn === undefined ? BAD_REQUEST : answer(decoded, fn);
}
