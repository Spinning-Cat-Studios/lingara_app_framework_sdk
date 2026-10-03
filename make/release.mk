# Release (ADR 30.9.26am D8, the clients' process ported). Ships with the
# repository.
#
# `languages.toml` is what a release publishes. tools/release-manifest is not
# copied here: it is installed from the public lingara_api_clients repository
# at a pinned tag, into .tools/ (gitignored), the first time a target needs it.

RELEASE_MANIFEST_TAG := v0.1.0-alpha.10
RELEASE_MANIFEST_REPO := https://github.com/Spinning-Cat-Studios/lingara_api_clients
RELEASE_MANIFEST_ROOT := .tools/release-manifest-$(RELEASE_MANIFEST_TAG)
RELEASE_MANIFEST := $(RELEASE_MANIFEST_ROOT)/bin/release-manifest

.PHONY: install-release-manifest bump-version release-matrix release-build release-conformance check-release-tree check-library-floor help-release

$(RELEASE_MANIFEST):
	$(CARGO) install --git $(RELEASE_MANIFEST_REPO) --tag $(RELEASE_MANIFEST_TAG) --locked --root $(RELEASE_MANIFEST_ROOT) release-manifest

## The pinned release-manifest; CI passes RELEASE_MANIFEST_ROOT to choose
## where, and puts its bin/ on PATH for tools/release/*.sh.
install-release-manifest: $(RELEASE_MANIFEST)

## Write V to VERSION, every version file and every `since = "next"`, then
## regenerate the version constants each kit's codegen writes, so
## check-codegen stays green.
bump-version: $(RELEASE_MANIFEST)
	@test -n "$(V)" || { echo "Usage: make bump-version V=X.Y.Z[-pre.N]" >&2; exit 2; }
	$(RELEASE_MANIFEST) bump $(V)
	$(MAKE) codegen

## The GitHub Actions matrix release.yml fans out over, for TAG.
release-matrix: $(RELEASE_MANIFEST)
	@test -n "$(TAG)" || { echo "Usage: make release-matrix TAG=vX.Y.Z[-pre.N]" >&2; exit 2; }
	@$(RELEASE_MANIFEST) matrix $(TAG)

## One kit's release build, ID being its languages.toml id: the kit's own
## targets, so the release build is the local build.
release-build:
	@test -n "$(ID)" || { echo "Usage: make release-build ID=<language id>" >&2; exit 2; }
	$(MAKE) check-codegen-$(ID) test-$(ID)

## One kit's fixture app against every conformance case.
release-conformance:
	@test -n "$(ID)" || { echo "Usage: make release-conformance ID=<language id>" >&2; exit 2; }
	$(MAKE) conformance-$(ID)

## languages.toml agrees with the tree.
check-release-tree: $(RELEASE_MANIFEST)
	$(RELEASE_MANIFEST) check

## One library floor in seven manifests (D1), and the check's own test.
check-library-floor:
	tools/library-floor/check_test.sh
	tools/library-floor/check.sh

help-release:
	@echo ""
	@echo "Release:"
	@echo "  make install-release-manifest    - The pinned release-manifest, into .tools/"
	@echo "  make bump-version V=<ver>        - Write the version everywhere, then make codegen"
	@echo "  make release-matrix TAG=<t>      - The release matrix for a tag"
	@echo "  make release-build ID=<id>       - One kit's check-codegen + tests"
	@echo "  make release-conformance ID=<id> - One kit's conformance suite"
	@echo "  make check-release-tree          - languages.toml agrees with the tree"
	@echo "  make check-library-floor         - One LIBRARY_FLOOR in seven manifests"

HELP_SECTIONS += release
