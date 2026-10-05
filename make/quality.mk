# The aggregates over $(LANGS) (ADR 30.9.26al D7).
#
# Read after every make/<lang>.mk (see the Makefile): these prerequisite
# lists are expanded when this file is read. A kit never edits this file;
# appending itself to LANGS is the whole of joining.

.PHONY: codegen check-codegen test test-tools check-publishable help-quality

## Regenerate every kit's generated code.
codegen: $(addprefix codegen-,$(LANGS))

## The view is current, and so is every kit's generated code.
check-codegen: check-spec-view $(addprefix check-codegen-,$(LANGS))

test: test-tools $(addprefix test-,$(LANGS))

## The view builder's and the mock host's own tests.
test-tools:
	$(CARGO) test --workspace

## What a published snapshot must pass: run before every publish. Also one
## library floor in every kit's manifest, and languages.toml against the tree
## (make/release.mk, ADR 30.9.26am D1, D8), and every scope the kit spells
## against the spec (ADR 4.10.26e D4).
check-publishable: check-spec-view check-codegen check-library-floor check-snippet-scopes check-release-tree

help-quality:
	@echo ""
	@echo "Quality:"
	@echo "  make codegen             - Regenerate every kit: $(or $(LANGS),none yet)"
	@echo "  make check-codegen       - The view and every kit's generated code are current"
	@echo "  make test                - The tools' tests and every kit's"
	@echo "  make check-publishable   - What a publish must pass"

HELP_SECTIONS += quality
