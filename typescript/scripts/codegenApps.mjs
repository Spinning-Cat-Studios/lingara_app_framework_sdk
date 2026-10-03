// Writes src/generated/apps.ts from the app view's `x-lingara-unions` and
// `x-lingara-app-operations` (ADR 30.9.26am D6).
//
// openapi-typescript writes every arm (`CardElementHeading`, …) into
// schema.ts, but a union component is a bare `{type: object}` there, so this
// script writes each union as a type alias over the generated arms, re-points
// every model that holds a union (`Card.elements`, `CardElementList.items`,
// a request's `context`) at the alias, and writes the operations, the
// ContextSlice kinds and the AppSlotName values as constants the core and
// the manifest builder read.
//
// It refuses a generator-written file named after a union or an arm it
// writes, and a top-level export of schema.ts carrying such a name: either
// would ship two types for one name.
//
// Usage: node scripts/codegenApps.mjs <view.json> <schema.ts> <out.ts>
// Node only, no dependencies.

import { readFileSync, readdirSync, writeFileSync } from "node:fs";
import { basename, dirname } from "node:path";

const [viewPath, schemaPath, outPath] = process.argv.slice(2);
if (!viewPath || !schemaPath || !outPath) {
  console.error("usage: codegenApps.mjs <view.json> <schema.ts> <out.ts>");
  process.exit(2);
}

function fail(message) {
  console.error(`codegenApps: ${message}`);
  process.exit(1);
}

const view = JSON.parse(readFileSync(viewPath, "utf8"));
const schemas = view.components?.schemas ?? {};
const unions = view["x-lingara-unions"];
const operations = view["x-lingara-app-operations"];
if (!Array.isArray(unions) || unions.length === 0) fail(`${viewPath}: no x-lingara-unions`);
if (!Array.isArray(operations) || operations.length === 0) fail(`${viewPath}: no x-lingara-app-operations`);

const SCHEMA_PREFIX = "#/components/schemas/";

function component(ref) {
  if (typeof ref !== "string" || !ref.startsWith(SCHEMA_PREFIX)) fail(`unexpected ref: ${JSON.stringify(ref)}`);
  const name = ref.slice(SCHEMA_PREFIX.length);
  if (!Object.hasOwn(schemas, name)) fail(`ref names no component: ${ref}`);
  return name;
}

const unionNames = unions.map((u) => u.name);
const armNames = unions.flatMap((u) => u.arms.map((a) => component(a.schema)));

// D6: the refusal. A file the generator wrote beside schema.ts, or a
// top-level export of schema.ts, named after a union or an arm.
function refuseCollisions() {
  const written = new Set([...unionNames, ...armNames]);
  for (const file of readdirSync(dirname(schemaPath))) {
    const stem = file.replace(/\.(d\.)?[cm]?ts$/, "");
    if (written.has(stem)) fail(`the generated file ${file} is named after a union or arm this script writes`);
  }
  const source = readFileSync(schemaPath, "utf8");
  for (const match of source.matchAll(/^export\s+(?:type|interface|const|class|enum)\s+([A-Za-z_$][\w$]*)/gm)) {
    if (written.has(match[1])) fail(`${basename(schemaPath)} exports ${match[1]}, a union or arm this script writes`);
  }
}

// The union an arm-or-model property points at, directly or as array items.
function unionOf(property) {
  const ref = property.$ref ?? property.items?.$ref;
  if (ref === undefined) return undefined;
  const name = component(ref);
  return unionNames.includes(name) || lifted.has(name) ? name : undefined;
}

// Every non-union component that holds a union, transitively: these are
// re-pointed at the aliases. A fixed point over the view.
const lifted = new Set();
for (let grew = true; grew; ) {
  grew = false;
  for (const [name, schema] of Object.entries(schemas)) {
    if (unionNames.includes(name) || lifted.has(name)) continue;
    if (Object.values(schema.properties ?? {}).some((p) => unionOf(p) !== undefined)) {
      lifted.add(name);
      grew = true;
    }
  }
}

const requestTypes = new Map(operations.map((o) => [component(o.request), o.message]));

function overrides(name) {
  const schema = schemas[name];
  const required = new Set(schema.required ?? []);
  const entries = [];
  for (const [key, property] of Object.entries(schema.properties ?? {})) {
    const target = unionOf(property);
    if (target === undefined) continue;
    const type = property.$ref ? target : `${target}[]`;
    entries.push(`${key}${required.has(key) ? "" : "?"}: ${type}`);
  }
  const message = requestTypes.get(name);
  if (message !== undefined) entries.unshift(`type: ${JSON.stringify(message)}`);
  return entries;
}

function alias(name) {
  const entries = overrides(name);
  if (entries.length === 0) return `export type ${name} = Schemas[${JSON.stringify(name)}];`;
  const keys = entries.map((e) => JSON.stringify(e.split(/\??:/)[0])).join(" | ");
  return `export type ${name} = Omit<Schemas[${JSON.stringify(name)}], ${keys}> & { ${entries.join("; ")} };`;
}

function unionBlock(u) {
  const arms = u.arms.map((a) => component(a.schema));
  return [
    `/** The \`${u.name}\` union, tagged on \`${u.tag}\`. */`,
    `export type ${u.name} =\n${arms.map((a) => `  | ${a}`).join("\n")};`,
    ...arms.map((a) => alias(a)),
  ].join("\n");
}

function constList(name, values) {
  return `export const ${name} = [${values.map((v) => JSON.stringify(v)).join(", ")}] as const;`;
}

function sliceUnion() {
  const slice = unions.find((u) => u.name === "ContextSlice");
  if (slice === undefined) fail("x-lingara-unions has no ContextSlice");
  return slice;
}

function sliceRequired(slice) {
  const lines = slice.arms.map((a) => {
    const required = schemas[component(a.schema)].required ?? [];
    return `  ${JSON.stringify(a.value)}: [${required.map((r) => JSON.stringify(r)).join(", ")}],`;
  });
  return `export const CONTEXT_SLICE_REQUIRED: { readonly [K in ContextSliceKind]: readonly string[] } = {\n${lines.join("\n")}\n};`;
}

function operationBlock() {
  const lines = operations.map(
    (o) => `  ${JSON.stringify(o.message)}: { operation: ${JSON.stringify(o.operation)}, request: ${JSON.stringify(component(o.request))}, reply: ${JSON.stringify(o.reply_message)} },`,
  );
  const replies = [...new Set(operations.map((o) => o.reply_message))];
  if (replies.length !== 1) fail(`expected one reply message, found ${replies.join(", ")}`);
  return [
    `/** The request discriminators, from x-lingara-app-operations: a closed set. */`,
    `export const APP_OPERATIONS = {\n${lines.join("\n")}\n} as const;`,
    `export type AppOperation = keyof typeof APP_OPERATIONS;`,
    `/** The reply every operation expects. */`,
    `export const APP_REPLY_MESSAGE = ${JSON.stringify(replies[0])};`,
  ].join("\n");
}

function slotValues() {
  const slot = schemas.AppSlotName;
  if (!Array.isArray(slot?.enum)) fail("AppSlotName is not an enum");
  return slot.enum;
}

function emit() {
  const slice = sliceUnion();
  const models = Object.keys(schemas).filter((n) => !unionNames.includes(n) && !armNames.includes(n));
  return `// Generated by scripts/codegenApps.mjs from the app view's x-lingara-unions
// and x-lingara-app-operations (ADR 30.9.26am D6). Do not edit: run
// \`make codegen-typescript\`.

import type { components } from "./schema.js";

type Schemas = components["schemas"];

${models.map(alias).join("\n")}

${unions.map(unionBlock).join("\n\n")}

/** The ContextSlice kinds, in the view's order. */
${constList("CONTEXT_SLICE_KINDS", slice.arms.map((a) => a.value))}
export type ContextSliceKind = (typeof CONTEXT_SLICE_KINDS)[number];

/** Each known slice's required members, for the core's lenient decoder. */
${sliceRequired(slice)}

/** The AppSlotName values. */
${constList("APP_SLOT_NAMES", slotValues())}

${operationBlock()}
`;
}

refuseCollisions();
writeFileSync(outPath, emit());
