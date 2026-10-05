#!/usr/bin/env bash
# Every scope the kit spells is a real one, and no snippet reads a learner's
# plan through the app's own client (ADR 4.10.26e D4).
#
# 1. A quoted string on the same line as a `scopes(` / `Scopes(` call under
#    snippets/, and every string in a `scopes` array of the manifest
#    conformance vectors, must be a key of an `availableScopes` object in
#    spec/asyncapi.json. Names only: a client's allowed_scopes is the
#    upload's check (Backend A1 §2), as the vectors' description says.
# 2. No `lingara:begin context` region calls a lesson-plan read: the app's
#    client reads its owner's account, never the learner's.
#
# Usage: tools/snippet-scopes/check.sh [root]   (root defaults to the repo)
set -euo pipefail

root="${1:-$(cd "$(dirname "$0")/../.." && pwd)}"
spec="$root/spec/asyncapi.json"
vectors="$root/conformance/vectors/manifest.json"
snippets="$root/snippets"
for f in "$spec" "$vectors"; do [ -f "$f" ] || { echo "✗ $f is missing"; exit 1; }; done
[ -d "$snippets" ] || { echo "✗ $snippets is missing"; exit 1; }

known="$(jq -r '[.. | objects | select(has("availableScopes")) | .availableScopes | keys[]] | unique[]' "$spec")"
[ -n "$known" ] || { echo "✗ $spec lists no availableScopes"; exit 1; }
is_known() { printf '%s\n' "$known" | grep -qxF -- "$1"; }

excludes=(--exclude-dir=node_modules --exclude-dir=target --exclude-dir=build
  --exclude-dir=vendor --exclude-dir=.gradle)
rel() { printf '%s' "${1#"$root"/}"; }

failures=0
fail() { echo "✗ $1"; failures=$((failures + 1)); }

# 1a. Snippet calls: any prefix (`.`, `->`, or none as in Kotlin's DSL).
snippet_strings=0
while IFS= read -r hit; do
  file="${hit%%:*}"; rest="${hit#*:}"; line="${rest%%:*}"; text="${rest#*:}"
  while IFS= read -r quoted; do
    [ -n "$quoted" ] || continue
    snippet_strings=$((snippet_strings + 1))
    scope="${quoted:1:${#quoted}-2}"
    is_known "$scope" || fail "$(rel "$file"):$line: scope \"$scope\" is not in spec/asyncapi.json availableScopes"
  done < <(printf '%s\n' "$text" | grep -oE "\"[^\"]*\"|'[^']*'" || true)
done < <(grep -rnE "${excludes[@]}" '(^|[^A-Za-z0-9_])[sS]copes\(' "$snippets" || true)

# 1b. Vector scopes.
vector_strings=0
while IFS= read -r scope; do
  vector_strings=$((vector_strings + 1))
  is_known "$scope" || fail "$(rel "$vectors"): scope \"$scope\" is not in spec/asyncapi.json availableScopes"
done < <(jq -r '.. | objects | select(has("scopes")) | .scopes | arrays | .[] | strings' "$vectors")

# 2. Lesson-plan reads inside context regions.
plan_reads=0
while IFS= read -r hit; do
  [ -n "$hit" ] || continue
  plan_reads=$((plan_reads + 1))
  fail "$(rel "${hit%%:*}"):${hit#*:}: lesson-plan read in a context region; the app's client reads its owner's account, not the learner's"
done < <(grep -rlE "${excludes[@]}" 'lingara:begin context' "$snippets" | while IFS= read -r file; do
  awk -v f="$file" '
    /lingara:begin context([^A-Za-z0-9_-]|$)/ { inside = 1; next }
    /lingara:end/ { inside = 0 }
    inside && /getLessonPlan|get_lesson_plan|GetLessonPlan/ { print f ":" FNR }
  ' "$file"
done)

echo "  read: $snippet_strings snippet scope string(s), $vector_strings vector scope(s), $plan_reads context plan read(s)"
if [ "$failures" -ne 0 ]; then
  echo "✗ $failures snippet-scope failure(s)"
  exit 1
fi
echo "✓ every scope the kit spells is in the spec; no context region reads a lesson plan"
