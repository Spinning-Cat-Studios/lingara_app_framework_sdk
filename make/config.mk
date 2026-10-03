# Shared variables (ADR 30.9.26al D7).
#
# LANGS starts empty. Each make/<lang>.mk (ADR 30.9.26am) appends
# `LANGS += <lang>`, defines codegen-<lang>, check-codegen-<lang> and
# test-<lang>, and sets CONFORMANCE_APP_CMD_<lang>; the aggregates in
# make/quality.mk and make/conformance.mk run over whatever has joined.

LANGS :=
HELP_SECTIONS :=

CARGO ?= cargo

SPEC_DIR := spec
SPEC_VIEW_DIR := $(SPEC_DIR)/generator
