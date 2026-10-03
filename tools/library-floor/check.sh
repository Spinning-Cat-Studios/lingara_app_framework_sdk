#!/usr/bin/env bash
# One library floor in seven manifests (ADR 30.9.26am D1).
#
# LIBRARY_FLOOR at the root holds the first `lingara` library release every
# kit depends on. Each kit's manifest spells it in its ecosystem's idiom; this
# reads each spelling back, normalises it (RubyGems' `.pre.` form included)
# and fails when any manifest declares another floor, or none.
#
# Usage: tools/library-floor/check.sh [root]   (root defaults to the repo)
set -euo pipefail

root="${1:-$(cd "$(dirname "$0")/../.." && pwd)}"
floor_file="$root/LIBRARY_FLOOR"
[ -f "$floor_file" ] || { echo "✗ $floor_file is missing"; exit 1; }
floor="$(tr -d '[:space:]' < "$floor_file")"
semver='[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.]+)?'

# RubyGems writes 0.1.0-alpha.7 as 0.1.0.pre.alpha.7.
from_rubygems() { sed -E 's/^([0-9]+\.[0-9]+\.[0-9]+)\.pre\./\1-/'; }

# The first version string on the first line matching $2 in file $1.
first_version() {
  local file="$1" pattern="$2" re="${3:-$semver}"
  [ -f "$file" ] || { echo "MISSING"; return; }
  grep -E "$pattern" "$file" | head -n 1 | grep -oE "$re" | head -n 1 || true
}

failures=0
check() {
  local label="$1" got="$2"
  if [ "$got" = "MISSING" ]; then
    echo "✗ $label: manifest not found"; failures=$((failures + 1))
  elif [ -z "$got" ]; then
    echo "✗ $label: declares no lingara library floor"; failures=$((failures + 1))
  elif [ "$got" != "$floor" ]; then
    echo "✗ $label: floor $got, LIBRARY_FLOOR is $floor"; failures=$((failures + 1))
  else
    echo "✓ $label: $got"
  fi
}

check "typescript/package.json" \
  "$(first_version "$root/typescript/package.json" '"@lingara/api"[[:space:]]*:[[:space:]]*"\^')"
check "rust/Cargo.toml" \
  "$(first_version "$root/rust/Cargo.toml" '^lingara[[:space:]]*=')"
check "go/go.mod" \
  "$(first_version "$root/go/go.mod" 'github.com/Spinning-Cat-Studios/lingara_api_clients/go[[:space:]]+v')"
check "java/build.gradle.kts" \
  "$(first_version "$root/java/build.gradle.kts" '"com\.getlingara:lingara-java:')"
check "kotlin/build.gradle.kts" \
  "$(first_version "$root/kotlin/build.gradle.kts" '"com\.getlingara:lingara-kotlin:')"
ruby_raw="$(first_version "$root/ruby/lingara-apps.gemspec" 'add_dependency[[:space:]]*\(?"lingara"' '[0-9]+\.[0-9]+\.[0-9]+(\.pre\.[0-9A-Za-z.]+)?')"
check "ruby/lingara-apps.gemspec" "$( [ "$ruby_raw" = MISSING ] && echo MISSING || printf '%s' "$ruby_raw" | from_rubygems)"
check "php/composer.json" \
  "$(first_version "$root/php/composer.json" '"spinningcatstudios/lingara"[[:space:]]*:[[:space:]]*"\^')"

if [ "$failures" -ne 0 ]; then
  echo "✗ $failures manifest(s) disagree with LIBRARY_FLOOR ($floor)"
  exit 1
fi
echo "✓ seven manifests, one library floor: $floor"
