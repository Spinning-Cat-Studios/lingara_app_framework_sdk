# The Rust app kit, crate lingara-apps (ADR 30.9.26am; the clients'
# make/rust.mk shape).
#
# `rust/` is the `lingara-apps` crate, a member of the root workspace beside
# its codegen (`rust/xtask`), its fixture app (`rust/conformance`) and its
# snippets (`snippets/rust`). The generated types are committed under
# rust/src/generated/ and never hand-edited: check-codegen-rust is what makes
# a hand edit red.

LANGS += rust

RUST_XTASK := $(CARGO) run -q -p lingara-apps-xtask --
RUST_GENERATED := rust/src/generated
RUST_PACKAGES := -p lingara-apps -p lingara-apps-xtask -p lingara-apps-conformance-rust -p lingara-apps-snippets

# The fixture app is built first (conformance-rust's prerequisite), so the
# host's 30 s wait for `listening <port>` never pays for a compile.
CONFORMANCE_APP_CMD_rust = $(CARGO) run -q -p lingara-apps-conformance-rust

.PHONY: codegen-rust check-codegen-rust test-rust build-conformance-rust help-rust

## The models, unions and operations, from the app view.
codegen-rust:
	$(RUST_XTASK) codegen

## Fails on any byte of difference from a fresh codegen.
check-codegen-rust:
	@tmp=$$(mktemp -d); \
	$(RUST_XTASK) codegen --out-dir $$tmp >/dev/null && \
	diff -ru $(RUST_GENERATED) $$tmp; \
	status=$$?; rm -rf $$tmp; \
	if [ $$status -ne 0 ]; then echo "✗ $(RUST_GENERATED) is stale: run make codegen-rust"; exit 1; fi; \
	echo "✓ $(RUST_GENERATED) is current"

## Clippy over the four members (the generated module allows itself), the
## kit's unit tests with and without the axum adapter, the codegen's own
## tests (they read the view, which the packaged crate does not carry), and
## the snippets' build.
test-rust:
	$(CARGO) clippy $(RUST_PACKAGES) --all-targets --all-features -- -D warnings
	$(CARGO) clippy -p lingara-apps --all-targets -- -D warnings
	$(CARGO) test -p lingara-apps --all-features
	$(CARGO) test -p lingara-apps
	$(CARGO) test -p lingara-apps-xtask
	$(CARGO) build -p lingara-apps-snippets --examples

build-conformance-rust:
	$(CARGO) build -q -p lingara-apps-conformance-rust

# The pattern rule in make/conformance.mk runs the host. Naming the target
# here with a prerequisite and no recipe keeps that recipe while defining
# conformance-rust in a shipped make file, as CI calls it. Not .PHONY: a
# phony target skips the implicit-rule search.
conformance-rust: build-conformance-rust

help-rust:
	@echo ""
	@echo "Rust:"
	@echo "  make codegen-rust         - Regenerate rust/src/generated from the view"
	@echo "  make check-codegen-rust   - Fail when the generated files are stale"
	@echo "  make test-rust            - Clippy, unit tests with and without axum, codegen tests, snippets"
	@echo "  make conformance-rust     - The host against the fixture app"

HELP_SECTIONS += rust
