# The Java app kit (ADR 30.9.26am; the clients' make/<lang>.mk shape).
#
# java/ is the Gradle project :java in the repo-root build Java and Kotlin
# share. Its generated half comes from :codegen's generateJava, its fixture
# app is :java:conformance and its snippets :snippets:java; every target
# below goes through the wrapper, so a JDK 17 is the only thing a machine
# needs.
#
# JAVA_TEST_JDK is empty by default. Set, it runs the unit tests on that
# toolchain (-PtestJdk) while compilation stays on 17: a newest-LTS CI leg is
# `make test-java JAVA_TEST_JDK=<n>`, never a bare ./gradlew.
#
# Two make rules the conformance lines depend on:
#   - java-fixture is .PHONY, so installDist runs every time and a local run
#     never tests a stale fixture; Gradle's up-to-date checks keep it cheap.
#   - conformance-java is never .PHONY. GNU make skips the implicit-rule
#     search for a phony target, which would silently drop the recipe of the
#     conformance-% pattern rule in make/conformance.mk. As an ordinary target
#     with no recipe of its own it takes that recipe, and the fixture runs
#     from the repository root with no Gradle output in its stream.

LANGS += java

GRADLE ?= ./gradlew
GRADLE_FLAGS ?= -q --console=plain
JAVA_TEST_JDK ?=
JAVA_GENERATED := java/src/generated/java
JAVA_FIXTURE := java/conformance/build/install/conformance/bin/conformance

CONFORMANCE_APP_CMD_java = $(JAVA_FIXTURE)

.PHONY: codegen-java check-codegen-java test-java java-fixture help-java

## The generated app types, unions, slice kinds and operations, from the view.
codegen-java:
	$(GRADLE) $(GRADLE_FLAGS) :codegen:generateJava

## Fails on any byte of difference from a fresh codegen. It syncs into a
## temporary directory, never over the working tree, so an unrelated
## uncommitted edit cannot fail it.
check-codegen-java:
	@tmp=$$(mktemp -d); status=0; \
	$(GRADLE) $(GRADLE_FLAGS) :codegen:generateJava -PcodegenOut=$$tmp || status=1; \
	[ $$status -ne 0 ] || diff -ru $(JAVA_GENERATED) $$tmp || status=1; \
	rm -rf $$tmp; \
	if [ $$status -ne 0 ]; then echo "✗ $(JAVA_GENERATED) is stale: run make codegen-java"; exit 1; fi; \
	echo "✓ $(JAVA_GENERATED) is current"

## AppsCodegen's tests; Spotless, Checkstyle and the JUnit suite (vectors,
## the adapter, the package shape); the fixture and the snippets compile.
test-java:
	$(GRADLE) $(GRADLE_FLAGS) $(if $(JAVA_TEST_JDK),-PtestJdk=$(JAVA_TEST_JDK)) :codegen:check :java:check :java:conformance:check :snippets:java:check

java-fixture:
	$(GRADLE) $(GRADLE_FLAGS) :java:conformance:installDist

# The pattern rule in make/conformance.mk runs the fixture. No recipe, and
# not .PHONY (see the header).
conformance-java: java-fixture

help-java:
	@echo ""
	@echo "Java:"
	@echo "  make codegen-java          - Regenerate java/src/generated from the view"
	@echo "  make check-codegen-java    - Fail when the generated sources are stale"
	@echo "  make test-java             - Spotless, Checkstyle, JUnit, AppsCodegen, snippets (JAVA_TEST_JDK=<n>)"
	@echo "  make conformance-java      - The conformance host against the fixture app"

HELP_SECTIONS += java
