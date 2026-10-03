// @lingara/apps: the Lingara app kit for Node (ADR 30.9.26am).

export { lingaraApp, handle, decodeRequest } from "./core.js";
export type {
  ActionFn,
  AppReply,
  AppRequest,
  CoreRequest,
  CoreResponse,
  LingaraApp,
  LingaraAppOptions,
  RenderFn,
} from "./core.js";
export { nodeHandler } from "./node.js";
export { card, item, reply, CardBuilder, Reply } from "./card.js";
export type { TermFields } from "./card.js";
export { manifest, validateManifest, manifestRule, ManifestBuilder, ManifestError, MANIFEST_RULES } from "./manifest.js";
export type { AppManifest, Localized, ManifestRule } from "./manifest.js";
export { validateReply, truncate, CardLimitError, REASONS } from "./limits.js";
export * as limits from "./limits.js";
export type { CardLimitReason, Validated } from "./limits.js";
export { APP_OPERATIONS, APP_REPLY_MESSAGE, APP_SLOT_NAMES, CONTEXT_SLICE_KINDS } from "./generated/apps.js";
export type * from "./generated/apps.js";
