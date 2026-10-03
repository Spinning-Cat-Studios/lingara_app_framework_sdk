# The Kotlin app kit (ADR 30.9.26am; the clients' make/<lang>.mk shape).
#
# kotlin/ is the Gradle project :kotlin in the repo-root build Java and Kotlin
# share. Its generated half comes from :codegen's generateKotlin, its fixture
# app is :kotlin:conformance (Ktor 3 on CIO) and its snippets
# :snippets:kotlin; every target below goes through the wrapper, so a JDK 17
# is the only thing a machine needs.
#
# KOTLIN_TEST_JDK is empty by default. Set, it runs the unit tests on that
# toolchain (-PtestJdk) while compilation stays on 17: a newest-LTS CI leg is
# `make test-kotlin KOTLIN_TEST_JDK=<n>`, never a bare ./gradlew.
#
# Two make rules the conformance lines depend on, as in make/java.mk:
#   - kotlin-fixture is .PHONY, so installDist runs every time and a local run
#     never tests a stale fixture; Gradle's up-to-date checks keep it cheap.
#   - conformance-kotlin is never .PHONY. GNU make skips the implicit-rule
#     search for a phony target, which would silently drop the recipe of the
#     conformance-% pattern rule in make/conformance.mk.

LANGS += kotlin

GRADLE ?= ./gradlew
GRADLE_FLAGS ?= -q --console=plain
KOTLIN_TEST_JDK ?=
KOTLIN_GENERATED := kotlin/src/generated/kotlin
KOTLIN_FIXTURE := kotlin/conformance/build/install/conformance/bin/conformance

CONFORMANCE_APP_CMD_kotlin = $(KOTLIN_FIXTURE)

.PHONY: codegen-kotlin check-codegen-kotlin test-kotlin kotlin-fixture help-kotlin

## The generated app types, unions, slice kinds and operations, from the view.
codegen-kotlin:
	$(GRADLE) $(GRADLE_FLAGS) :codegen:generateKotlin

## Fails on any byte of difference from a fresh codegen. It syncs into a
## temporary directory, never over the working tree, so an unrelated
## uncommitted edit cannot fail it.
check-codegen-kotlin:
	@tmp=$$(mktemp -d); status=0; \
	$(GRADLE) $(GRADLE_FLAGS) :codegen:generateKotlin -PcodegenOut=$$tmp || status=1; \
	[ $$status -ne 0 ] || diff -ru $(KOTLIN_GENERATED) $$tmp || status=1; \
	rm -rf $$tmp; \
	if [ $$status -ne 0 ]; then echo "✗ $(KOTLIN_GENERATED) is stale: run make codegen-kotlin"; exit 1; fi; \
	echo "✓ $(KOTLIN_GENERATED) is current"

## AppsCodegen's tests; Spotless (ktlint) and the JUnit suite (vectors, the
## Ktor route, the package shape); the fixture and the snippets compile.
test-kotlin:
	$(GRADLE) $(GRADLE_FLAGS) $(if $(KOTLIN_TEST_JDK),-PtestJdk=$(KOTLIN_TEST_JDK)) :codegen:check :kotlin:check :kotlin:conformance:check :snippets:kotlin:check

kotlin-fixture:
	$(GRADLE) $(GRADLE_FLAGS) :kotlin:conformance:installDist

# The pattern rule in make/conformance.mk runs the fixture. No recipe, and
# not .PHONY (see the header).
conformance-kotlin: kotlin-fixture

help-kotlin:
	@echo ""
	@echo "Kotlin:"
	@echo "  make codegen-kotlin        - Regenerate kotlin/src/generated/kotlin from the view"
	@echo "  make check-codegen-kotlin  - Fail when the generated sources are stale"
	@echo "  make test-kotlin           - Spotless, JUnit, AppsCodegen, snippets (KOTLIN_TEST_JDK=<n>)"
	@echo "  make conformance-kotlin    - The conformance host against the fixture app"

HELP_SECTIONS += kotlin
