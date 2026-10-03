# The TypeScript app kit, @lingara/apps (ADR 30.9.26am; the clients'
# make/typescript.mk shape).
#
# `typescript/` is an npm package whose one runtime dependency is
# @lingara/api. Every target that needs the pinned dev tools depends on
# $(TS_STAMP), a stamped `npm ci`, so a fresh checkout installs once and a
# changed lockfile reinstalls.
#
# The fixture app is built from the package's own `dist/` (tsup rewrites
# `@lingara/apps` to ../../dist/index.mjs), so conformance tests the artefact
# that ships, and it runs on node:http.

LANGS += typescript

TS_DIR := typescript
TS_NPM := npm --prefix $(TS_DIR)
TS_BIN := $(TS_DIR)/node_modules/.bin
TS_STAMP := $(TS_DIR)/node_modules/.lingara-ci-stamp
TS_VIEW := $(SPEC_VIEW_DIR)/apps.3.1.json

CONFORMANCE_APP_CMD_typescript = node $(TS_DIR)/conformance/dist/fixture.mjs

.PHONY: codegen-typescript check-codegen-typescript test-typescript build-typescript help-typescript

$(TS_STAMP): $(TS_DIR)/package.json $(TS_DIR)/package-lock.json
	$(TS_NPM) ci --no-audit --no-fund
	@touch $@

# Writes $(1)/schema.ts (openapi-typescript) and $(1)/apps.ts (the unions,
# the slice kinds and the operations, ADR 30.9.26am D6) from the 3.1 view.
define ts_codegen
	$(TS_BIN)/openapi-typescript $(TS_VIEW) -o $(1)/schema.ts --silent
	node $(TS_DIR)/scripts/codegenApps.mjs $(TS_VIEW) $(1)/schema.ts $(1)/apps.ts
endef

## The generated models, unions and operations, from the app view.
codegen-typescript: $(TS_STAMP)
	$(call ts_codegen,$(TS_DIR)/src/generated)

## Fails on any byte of difference from a fresh codegen.
check-codegen-typescript: $(TS_STAMP)
	@tmp=$$(mktemp -d); \
	{ $(TS_BIN)/openapi-typescript $(TS_VIEW) -o $$tmp/schema.ts --silent && \
	node $(TS_DIR)/scripts/codegenApps.mjs $(TS_VIEW) $$tmp/schema.ts $$tmp/apps.ts && \
	diff -ru $(TS_DIR)/src/generated $$tmp; }; \
	status=$$?; rm -rf $$tmp; \
	if [ $$status -ne 0 ]; then echo "✗ typescript/src/generated is stale: run make codegen-typescript"; exit 1; fi; \
	echo "✓ typescript/src/generated is current"

## The two bundles, their declarations and the fixture app.
build-typescript: $(TS_STAMP)
	$(TS_NPM) run build

## Lint budgets, strict types, unit tests, the build, the package checks and
## the snippets' type-check.
test-typescript: $(TS_STAMP)
	cd $(TS_DIR) && ./node_modules/.bin/eslint .
	cd $(TS_DIR) && ./node_modules/.bin/tsc --noEmit
	cd $(TS_DIR) && ./node_modules/.bin/vitest run
	$(TS_NPM) run build
	cd $(TS_DIR) && ./node_modules/.bin/publint --strict
	cd $(TS_DIR) && ./node_modules/.bin/attw --pack .
	$(TS_BIN)/tsc --noEmit -p snippets/typescript

# conformance-typescript needs the built fixture first; the pattern rule in
# make/conformance.mk does the rest.
conformance-typescript: build-typescript

help-typescript:
	@echo ""
	@echo "TypeScript:"
	@echo "  make codegen-typescript        - Regenerate typescript/src/generated from the app view"
	@echo "  make check-codegen-typescript  - Fail when the generated files are stale"
	@echo "  make test-typescript           - ESLint, tsc, vitest, build, publint, attw, snippets"
	@echo "  make conformance-typescript    - The fixture app (node:http) against every host case"

HELP_SECTIONS += typescript
