# The PHP app kit (ADR 30.9.26am; the library's make/php.mk, ADR 29.9.26u).
#
# php/ is the Composer package spinningcatstudios/lingara-apps, whose
# `require` is php, the lingara library and two PSR interface packages. Its
# models come from :codegen's generatePhp (openapi-generator's php-nextgen,
# on the one pin the JVM and Ruby kits share) and its unions, arms and
# operations from php/codegen/generate.php, a standard-library script. Both
# halves are committed, so building, testing and releasing the package need
# no JDK: only regenerating does.
#
# The fixture app runs twice, once per PSR-7 implementation (D9), through
# make/conformance.mk's variant mechanism: CONFORMANCE_VARIANTS_php names
# them and the host runs as --lang php/<variant>. conformance-php has no
# recipe and is not .PHONY, so it takes the conformance-% pattern rule's
# recipe and adds only the Composer install as a prerequisite (a .PHONY
# target skips the implicit-rule search and would drop that recipe).

LANGS += php

PHP ?= php
COMPOSER ?= composer
GRADLE ?= ./gradlew
GRADLE_FLAGS ?= -q --console=plain
PHP_DIR := php
PHP_SRC := $(PHP_DIR)/src
PHP_VENDOR := $(PHP_DIR)/vendor/autoload.php
# Every generated path under php/src/, relative to it.
PHP_GENERATED := Generated ObjectSerializer.php

CONFORMANCE_VARIANTS_php := nyholm guzzle
CONFORMANCE_APP_CMD_php_nyholm = $(PHP) $(abspath $(PHP_DIR))/conformance/serve.php nyholm
CONFORMANCE_APP_CMD_php_guzzle = $(PHP) $(abspath $(PHP_DIR))/conformance/serve.php guzzle

.PHONY: codegen-php check-codegen-php test-php help-php

# The lockfile is a prerequisite only when it exists: the first install
# writes it.
$(PHP_VENDOR): $(PHP_DIR)/composer.json $(wildcard $(PHP_DIR)/composer.lock)
	$(COMPOSER) --working-dir=$(PHP_DIR) install --no-interaction --no-progress
	@touch $@

## The generated models, then the unions, arms and operations, from the view.
codegen-php:
	$(GRADLE) $(GRADLE_FLAGS) :codegen:generatePhp
	$(PHP) $(PHP_DIR)/codegen/generate.php

## Fails on any byte of difference from a fresh codegen. Both halves write
## into a temporary directory, never over the working tree, and only the
## generated files are compared: the hand-written core beside them is not.
check-codegen-php:
	@tmp=$$(mktemp -d); status=0; \
	$(GRADLE) $(GRADLE_FLAGS) :codegen:generatePhp -PcodegenOut=$$tmp > /dev/null || status=1; \
	[ $$status -ne 0 ] || $(PHP) $(PHP_DIR)/codegen/generate.php --out $$tmp || status=1; \
	for f in $(PHP_GENERATED); do [ $$status -ne 0 ] || diff -ru $(PHP_SRC)/$$f $$tmp/$$f || status=1; done; \
	rm -rf $$tmp; \
	if [ $$status -ne 0 ]; then echo "✗ php/src's generated files are stale: run make codegen-php"; exit 1; fi; \
	echo "✓ php/src's generated files are current"

## PHPUnit, PHPStan (which also type-checks snippets/php and the fixture app
## against the kit), php-cs-fixer, the PHPCS budgets, and the snippets' syntax.
test-php: $(PHP_VENDOR)
	cd $(PHP_DIR) && vendor/bin/phpunit
	cd $(PHP_DIR) && vendor/bin/phpstan analyse --no-progress --memory-limit=1G
	cd $(PHP_DIR) && vendor/bin/php-cs-fixer check --show-progress=none
	cd $(PHP_DIR) && vendor/bin/phpcs
	@for f in snippets/php/*.php; do $(PHP) -l $$f > /dev/null || exit 1; done; echo "✓ snippets/php parses"

# The pattern rule in make/conformance.mk runs the host once per variant.
# No recipe, and not .PHONY (see the header).
conformance-php: $(PHP_VENDOR)

help-php:
	@echo ""
	@echo "PHP:"
	@echo "  make codegen-php         - Regenerate php/src's models, unions, arms and operations"
	@echo "  make check-codegen-php   - Fail when the generated files are stale"
	@echo "  make test-php            - PHPUnit, PHPStan, php-cs-fixer, the budgets, snippets"
	@echo "  make conformance-php     - The host against the PHP fixture app, on nyholm/psr7 and on guzzlehttp/psr7"

HELP_SECTIONS += php
