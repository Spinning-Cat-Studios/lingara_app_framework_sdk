# The app view (ADR 30.9.26al D4).
#
# The kits generate their models with the OpenAPI generators their languages'
# libraries already use, so tools/app-codegen turns the AsyncAPI catalogue's
# app operations into an OpenAPI document with empty `paths`, in two dialects:
# spec/generator/apps.3.1.json and spec/generator/apps.3.0.json. The view is
# committed; `check-spec-view` fails on any byte of difference, so a
# hand-edited view is red.
#
# The view is built from the frozen snapshot of the registry's current
# version, never the live spec/asyncapi.json.

APP_CODEGEN := $(CARGO) run -q -p app-codegen --
APP_CODEGEN_ARGS := --registry $(SPEC_DIR)/versions.toml --source $(SPEC_DIR)/SOURCE --out-dir $(SPEC_VIEW_DIR)

.PHONY: spec-view check-spec-view help-spec

spec-view:
	$(APP_CODEGEN) $(APP_CODEGEN_ARGS)

check-spec-view:
	$(APP_CODEGEN) $(APP_CODEGEN_ARGS) --check

help-spec:
	@echo ""
	@echo "Spec:"
	@echo "  make spec-view         - Regenerate both dialects of the app view"
	@echo "  make check-spec-view   - Fail when the committed view differs from the spec"

HELP_SECTIONS += spec
