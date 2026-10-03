# The Go app kit (ADR 30.9.26am; the library's make/go.mk, ADR 29.9.26q).
#
# `go/` is the module github.com/Spinning-Cat-Studios/lingara_app_framework_sdk/go,
# whose one require is the lingara library. Its generator (`go/codegen`) and
# fixture app (`go/conformance`) are nested modules, so the module zip
# carries neither, and `snippets/go` is a module of its own. A nested module
# is outside its parent's `./...`, so the targets below name all four.
#
# GOTOOLCHAIN defaults to `local` for every entry point. Under Go's default,
# `auto`, a `go run <tool>@<pin>` may switch to whatever toolchain that tool
# asks for, and CI's Go 1.23 leg would silently run something newer. The one
# exception is lint-go: golangci-lint v2 needs Go 1.24 to build, so it runs on
# the stable leg only, as the library's linters do.
#
# Two make rules the conformance line depends on:
#   - $(GO_FIXTURE) is .PHONY, so it is rebuilt on every run. Otherwise an
#     existing binary would never be rebuilt after a kit edit, and a local
#     run would test stale code. Go's build cache keeps the rebuild cheap.
#   - conformance-go is never .PHONY. GNU make skips the implicit-rule search
#     for a phony target, which would silently drop the recipe of the
#     conformance-% pattern rule in make/conformance.mk. As an ordinary target
#     with no recipe of its own it takes that recipe.

LANGS += go

export GOTOOLCHAIN ?= local

GO ?= go
GOFMT ?= gofmt
GO_DIR := go
GO_VIEW := $(SPEC_VIEW_DIR)/apps.3.0.json
GO_MODULES := go go/codegen go/conformance snippets/go
GO_GENERATED := models_gen.go apps_gen.go
GO_FIXTURE := target/go-apps-fixture

# Pinned, and run with `go run`, so no go.mod ever names them. oapi-codegen's
# pin builds on the Go 1.23 floor, where check-codegen-go runs.
OAPI_CODEGEN := $(GO) run github.com/oapi-codegen/oapi-codegen/v2/cmd/oapi-codegen@v2.6.0
GOLANGCI_LINT ?= $(GO) run github.com/golangci/golangci-lint/v2/cmd/golangci-lint@v2.14.0

CONFORMANCE_APP_CMD_go = $(abspath $(GO_FIXTURE))

.PHONY: codegen-go check-codegen-go test-go lint-go help-go $(GO_FIXTURE)

# Writes the two *_gen.go files into $(1): oapi-codegen's models (it writes
# to stdout with no `output` key), then go/codegen's unions, slice kinds and
# operations, which reads the models it was just given.
define go_codegen
	$(OAPI_CODEGEN) -config $(GO_DIR)/oapi-codegen.yaml $(GO_VIEW) > $(1)/models_gen.go
	$(GO) -C $(GO_DIR)/codegen run . -view $(abspath $(GO_VIEW)) -models $(abspath $(1))/models_gen.go -out $(abspath $(1))
endef

## The generated models, unions, slice kinds and operations, from the view.
codegen-go:
	$(call go_codegen,$(GO_DIR))

## Fails on any byte of difference from a fresh codegen. It writes into a
## temporary directory, never over the working tree.
check-codegen-go:
	@tmp=$$(mktemp -d); status=0; \
	$(OAPI_CODEGEN) -config $(GO_DIR)/oapi-codegen.yaml $(GO_VIEW) > $$tmp/models_gen.go && \
	$(GO) -C $(GO_DIR)/codegen run . -view $(abspath $(GO_VIEW)) -models $$tmp/models_gen.go -out $$tmp || status=1; \
	for f in $(GO_GENERATED); do [ $$status -ne 0 ] || diff -u $(GO_DIR)/$$f $$tmp/$$f || status=1; done; \
	rm -rf $$tmp; \
	if [ $$status -ne 0 ]; then echo "✗ go/*_gen.go is stale: run make codegen-go"; exit 1; fi; \
	echo "✓ go/*_gen.go is current"

## gofmt and vet over the four modules (the snippets build with vet), the
## kit's and the generator's unit tests under the race detector.
test-go:
	@unformatted=$$($(GOFMT) -l $(GO_MODULES)); \
	if [ -n "$$unformatted" ]; then echo "✗ gofmt -l:"; echo "$$unformatted"; exit 1; fi
	@for m in $(GO_MODULES); do echo "go vet: $$m"; $(GO) -C $$m vet ./... || exit 1; done
	$(GO) -C $(GO_DIR) test -race ./...
	$(GO) -C $(GO_DIR)/codegen test -race ./...

## golangci-lint (the budgets in go/.golangci.yml) over the kit, its
## generator and its fixture app. Needs Go >= 1.24 to build the linter.
lint-go: export GOTOOLCHAIN = auto
lint-go:
	cd $(GO_DIR) && $(GOLANGCI_LINT) run ./...
	cd $(GO_DIR)/codegen && $(GOLANGCI_LINT) run --config ../.golangci.yml ./...
	cd $(GO_DIR)/conformance && $(GOLANGCI_LINT) run --config ../.golangci.yml ./...

$(GO_FIXTURE):
	$(GO) -C $(GO_DIR)/conformance build -o $(abspath $(GO_FIXTURE)) .

# The pattern rule in make/conformance.mk runs the host against the fixture.
# No recipe, and not .PHONY (see the header).
conformance-go: $(GO_FIXTURE)

help-go:
	@echo ""
	@echo "Go:"
	@echo "  make codegen-go          - Regenerate go/*_gen.go from the view"
	@echo "  make check-codegen-go    - Fail when the generated files are stale"
	@echo "  make test-go             - gofmt, vet over the four modules (snippets included), unit tests under -race"
	@echo "  make lint-go             - golangci-lint: the budgets in go/.golangci.yml"
	@echo "  make conformance-go      - The host against the Go fixture app"

HELP_SECTIONS += go
