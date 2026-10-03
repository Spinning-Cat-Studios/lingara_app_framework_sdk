#!/usr/bin/env bash
# The floor check over a scratch tree (ADR 30.9.26am D1, AC29).
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
check="$here/check.sh"

# Seven manifests spelling one floor in their idioms, written under $1.
write_tree() {
  local root="$1" floor="$2" ruby_floor="$3"
  mkdir -p "$root"/{typescript,rust,go,java,kotlin,ruby,php}
  printf '%s\n' "$floor" > "$root/LIBRARY_FLOOR"
  printf '{\n  "dependencies": {\n    "@lingara/api": "^%s"\n  }\n}\n' "$floor" > "$root/typescript/package.json"
  printf '[dependencies]\nlingara = { version = "%s", default-features = false }\nserde = "1"\n' "$floor" > "$root/rust/Cargo.toml"
  printf 'module x\n\ngo 1.23\n\nrequire github.com/Spinning-Cat-Studios/lingara_api_clients/go v%s\n' "$floor" > "$root/go/go.mod"
  printf 'dependencies {\n    api("com.getlingara:lingara-java:%s")\n}\n' "$floor" > "$root/java/build.gradle.kts"
  printf 'dependencies {\n    api("com.getlingara:lingara-kotlin:%s")\n}\n' "$floor" > "$root/kotlin/build.gradle.kts"
  printf 'Gem::Specification.new do |spec|\n  spec.add_dependency "lingara", ">= %s", "< 0.2"\nend\n' "$ruby_floor" > "$root/ruby/lingara-apps.gemspec"
  printf '{\n  "require": {\n    "spinningcatstudios/lingara": "^%s"\n  }\n}\n' "$floor" > "$root/php/composer.json"
}

# 30.9.26am AC29: one floor in seven idioms passes; one manifest on another
# floor fails, naming it.
test_one_floor_in_seven_manifests() {
  local tmp; tmp="$(mktemp -d)"
  trap 'rm -rf "$tmp"' RETURN
  write_tree "$tmp/same" 0.1.0-alpha.10 0.1.0.pre.alpha.10
  "$check" "$tmp/same" > "$tmp/same.out" || { cat "$tmp/same.out"; echo "FAIL: one floor was refused"; return 1; }

  write_tree "$tmp/ruby-drift" 0.1.0-alpha.10 0.1.0.pre.alpha.9
  if "$check" "$tmp/ruby-drift" > "$tmp/drift.out"; then echo "FAIL: a drifted gemspec passed"; return 1; fi
  grep -q 'ruby/lingara-apps.gemspec: floor 0.1.0-alpha.9' "$tmp/drift.out" || { cat "$tmp/drift.out"; return 1; }

  write_tree "$tmp/go-drift" 0.1.0-alpha.10 0.1.0.pre.alpha.10
  sed -i.bak 's/v0.1.0-alpha.10/v0.1.0-alpha.11/' "$tmp/go-drift/go/go.mod"
  if "$check" "$tmp/go-drift" > "$tmp/go.out"; then echo "FAIL: a drifted go.mod passed"; return 1; fi

  write_tree "$tmp/missing" 0.1.0-alpha.10 0.1.0.pre.alpha.10
  printf '{\n  "dependencies": {}\n}\n' > "$tmp/missing/php/composer.json"
  if "$check" "$tmp/missing" > "$tmp/missing.out"; then echo "FAIL: a manifest without the library passed"; return 1; fi
  grep -q 'php/composer.json: declares no lingara library floor' "$tmp/missing.out" || { cat "$tmp/missing.out"; return 1; }
}

test_one_floor_in_seven_manifests
echo "✓ check_test.sh: test_one_floor_in_seven_manifests"
