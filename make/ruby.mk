# The Ruby app kit (ADR 30.9.26am; the library's make/ruby.mk, ADR 29.9.26t).
#
# ruby/ is the gem `lingara-apps`, whose one runtime dependency is the
# `lingara` library. Its models come from :codegen's generateRuby
# (openapi-generator, models only, synced into lib/lingara/apps/generated/)
# and its unions, arms, operations and version constants from
# ruby/codegen/generate.rb, a standard-library script that writes beside
# generated/ (the Sync deletes anything else inside it). Both halves are
# committed, so building, testing and releasing the gem need no JDK: only
# regenerating does.
#
# The fixture app runs on rack + webrick from ruby/conformance/Gemfile, a
# bundle of its own, so neither server ever reaches the gem's manifest.
# conformance-ruby is never .PHONY: GNU make skips the implicit-rule search
# for a phony target, which would silently drop the recipe of the
# conformance-% pattern rule in make/conformance.mk.

LANGS += ruby

RUBY ?= ruby
BUNDLE ?= bundle
GRADLE ?= ./gradlew
GRADLE_FLAGS ?= -q --console=plain
RUBY_DIR := ruby
RUBY_LIB := $(RUBY_DIR)/lib/lingara/apps
RUBY_SCRIPT_OUTPUTS := unions.rb operations.rb version.rb
RUBY_CONFORMANCE_GEMFILE := $(abspath $(RUBY_DIR)/conformance/Gemfile)
# Each Gemfile.lock records the gem's own version, and a frozen bundle (CI's)
# refuses a lockfile that disagrees with version.rb. So the lockfiles'
# version lines are generated with version.rb, and checked with it.
RUBY_LOCKS := $(RUBY_DIR)/Gemfile.lock $(RUBY_DIR)/conformance/Gemfile.lock
RUBY_GEM_VERSION = $(RUBY) -I$(RUBY_DIR)/lib -rlingara/apps/version -e 'print Lingara::Apps::GEM_VERSION'

CONFORMANCE_APP_CMD_ruby = env BUNDLE_GEMFILE=$(RUBY_CONFORMANCE_GEMFILE) $(BUNDLE) exec $(RUBY) $(abspath $(RUBY_DIR)/conformance/fixture.rb)

.PHONY: codegen-ruby check-codegen-ruby test-ruby help-ruby

## The generated models, then the unions, arms, operations and version, from
## the view; then the lockfiles' version lines.
codegen-ruby:
	$(GRADLE) $(GRADLE_FLAGS) :codegen:generateRuby
	$(RUBY) $(RUBY_DIR)/codegen/generate.rb
	@v=$$($(RUBY_GEM_VERSION)) && for f in $(RUBY_LOCKS); do \
	  $(RUBY) -e 'f = ARGV[0]; File.write(f, File.read(f).sub(/^    lingara-apps \(.+\)$$/, "    lingara-apps (#{ARGV[1]})"))' $$f "$$v"; \
	done

## Fails on any byte of difference from a fresh codegen. Both halves write
## into a temporary directory, never over the working tree, and only the
## generated files are compared: the hand-written core beside them is not.
check-codegen-ruby:
	@tmp=$$(mktemp -d); status=0; \
	$(GRADLE) $(GRADLE_FLAGS) :codegen:generateRuby -PcodegenOut=$$tmp/generated || status=1; \
	[ $$status -ne 0 ] || $(RUBY) $(RUBY_DIR)/codegen/generate.rb --out $$tmp || status=1; \
	[ $$status -ne 0 ] || diff -ru $(RUBY_LIB)/generated $$tmp/generated || status=1; \
	for f in $(RUBY_SCRIPT_OUTPUTS); do [ $$status -ne 0 ] || diff -u $(RUBY_LIB)/$$f $$tmp/$$f || status=1; done; \
	for f in $(RUBY_LOCKS); do [ $$status -ne 0 ] || grep -qxF "    lingara-apps ($$($(RUBY_GEM_VERSION)))" $$f || { echo "✗ $$f does not lock lingara-apps at version.rb's GEM_VERSION"; status=1; }; done; \
	rm -rf $$tmp; \
	if [ $$status -ne 0 ]; then echo "✗ ruby/lib/lingara/apps' generated files are stale: run make codegen-ruby"; exit 1; fi; \
	echo "✓ ruby/lib/lingara/apps' generated files are current"

## The unit, codegen, adapter and gemspec tests; RuboCop (standard's style
## and the budgets in ruby/.rubocop.yml); rbs validate; the snippets load
## under -w.
test-ruby:
	cd $(RUBY_DIR) && $(BUNDLE) exec $(RUBY) -Ilib -Itest -e 'Dir["test/*_test.rb"].sort.each { |f| require File.expand_path(f) }'
	cd $(RUBY_DIR) && $(BUNDLE) exec rubocop
	@cd $(RUBY_DIR) && $(RUBY) -e 'gen = %w[unions.rb operations.rb version.rb].map { |f| "lib/lingara/apps/#{f}" }; \
	  long = (Dir["{lib,codegen,conformance,test}/**/*.rb"] - gen).reject { |f| f.include?("/generated/") }.select { |f| File.foreach(f).count > 600 }; \
	  abort("✗ over the 600-line file budget: #{long.join(", ")}") unless long.empty?'
	cd $(RUBY_DIR) && $(BUNDLE) exec rbs -I sig validate
	@for f in snippets/ruby/*.rb; do $(RUBY) -wc $$f > /dev/null || exit 1; done
	cd $(RUBY_DIR) && $(BUNDLE) exec $(RUBY) -Ilib -e 'Dir["../snippets/ruby/*.rb"].sort.each { |f| require File.expand_path(f) }'
	@echo "✓ snippets/ruby parses under -w and loads against the kit"

# The pattern rule in make/conformance.mk runs the host against the fixture
# app; this installs the fixture's bundle first. No recipe of its own, and
# not .PHONY (see the header).
conformance-ruby: ruby-conformance-bundle

.PHONY: ruby-conformance-bundle
ruby-conformance-bundle:
	@BUNDLE_GEMFILE=$(RUBY_CONFORMANCE_GEMFILE) $(BUNDLE) check > /dev/null || BUNDLE_GEMFILE=$(RUBY_CONFORMANCE_GEMFILE) $(BUNDLE) install

help-ruby:
	@echo ""
	@echo "Ruby:"
	@echo "  make codegen-ruby        - Regenerate ruby/lib/lingara/apps' models, unions, operations and version, and the lockfiles' version lines"
	@echo "  make check-codegen-ruby  - Fail when the generated files are stale"
	@echo "  make test-ruby           - Unit tests, RuboCop (style and budgets), rbs validate, the snippets"
	@echo "  make conformance-ruby    - The host against the Ruby fixture app"

HELP_SECTIONS += ruby
