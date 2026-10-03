# The conformance suite (ADR 30.9.26al D6).
#
# Read after every make/<lang>.mk (see the Makefile), so $(LANGS) is complete
# when `conformance`'s prerequisites expand. The direction is the reverse of
# the libraries': the host is the relay's stand-in and calls the kit's fixture
# app, so no harness can skip a case.
#
# A kit joins by setting CONFORMANCE_APP_CMD_<lang> to the command that starts
# its fixture app. A kit whose fixture runs on several stacks sets
# CONFORMANCE_VARIANTS_<lang> := a b instead, plus one
# CONFORMANCE_APP_CMD_<lang>_<variant> per variant; the host then runs once
# per variant, as --lang <lang>/<variant>.

CONFORMANCE_HOST := $(CARGO) run -q -p lingara-apps-conformance-host --
CONFORMANCE_CASES := conformance/cases

.PHONY: conformance check-conformance-coverage help-conformance

## Every landed kit's fixture app against every case.
conformance: $(addprefix conformance-,$(LANGS))
	@if [ -z "$(strip $(LANGS))" ]; then echo "no kit has landed yet: nothing to run"; fi

conformance-%:
	@if [ -n "$(strip $(CONFORMANCE_VARIANTS_$*))" ]; then \
	  for v in $(CONFORMANCE_VARIANTS_$*); do \
	    $(MAKE) -s conformance-variant LANG_ID=$*/$$v CMD_VAR=CONFORMANCE_APP_CMD_$*_$$v || exit 1; \
	  done; \
	else \
	  $(CONFORMANCE_HOST) run --lang $* --cases $(CONFORMANCE_CASES) -- $(CONFORMANCE_APP_CMD_$*); \
	fi

.PHONY: conformance-variant
conformance-variant:
	$(CONFORMANCE_HOST) run --lang $(LANG_ID) --cases $(CONFORMANCE_CASES) -- $($(CMD_VAR))

# The commands a landed kit is missing: CONFORMANCE_APP_CMD_<lang>, or one
# CONFORMANCE_APP_CMD_<lang>_<variant> per declared variant. `conformance-%`
# is a pattern rule, so every name "has" the target; only the variables say a
# fixture app exists.
conformance_missing_for = $(if $(strip $(CONFORMANCE_VARIANTS_$(1))),$(foreach v,$(CONFORMANCE_VARIANTS_$(1)),$(if $(strip $(CONFORMANCE_APP_CMD_$(1)_$(v))),,CONFORMANCE_APP_CMD_$(1)_$(v))),$(if $(strip $(CONFORMANCE_APP_CMD_$(1))),,CONFORMANCE_APP_CMD_$(1)))
CONFORMANCE_MISSING = $(strip $(foreach l,$(LANGS),$(call conformance_missing_for,$(l))))

## Every app operation and AK1–AK5 has a case, every case parses, and every
## landed kit has a fixture app command.
check-conformance-coverage:
	$(CONFORMANCE_HOST) check-coverage --view $(SPEC_VIEW_DIR)/apps.3.1.json --cases $(CONFORMANCE_CASES)
	@if [ -n "$(CONFORMANCE_MISSING)" ]; then \
	  echo "✗ not set: $(CONFORMANCE_MISSING)"; exit 1; \
	fi
	@echo "✓ every kit in LANGS has a fixture app: $(or $(LANGS),none yet)"

help-conformance:
	@echo ""
	@echo "Conformance:"
	@echo "  make conformance                 - Every kit's fixture app, every case: $(or $(LANGS),none yet)"
	@echo "  make conformance-<lang>          - One kit's fixture app"
	@echo "  make check-conformance-coverage  - Every app operation and AK1–AK5 has a case; every kit has a fixture app"

HELP_SECTIONS += conformance
