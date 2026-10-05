#!/usr/bin/env bash
# The snippet-scope check over a scratch tree (ADR 4.10.26e D4, AC1–AC3).
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
check="$here/check.sh"

# A clean tree under $1: a spec with two scopes, a vector declaring a real one,
# and a snippet whose context region reads only the slices.
write_tree() {
  local root="$1"
  mkdir -p "$root"/{spec,conformance/vectors,snippets/kit}
  cat > "$root/spec/asyncapi.json" <<'JSON'
{"components": {"securitySchemes": {"oauth": {"flows": {"clientCredentials": {
  "availableScopes": {"lesson_plans:read": "Read plans.", "vocab:generate": "Generate."}}}}}}}
JSON
  cat > "$root/conformance/vectors/manifest.json" <<'JSON'
{"description": "x", "vectors": [
  {"name": "ok", "manifest": {"scopes": ["lesson_plans:read"]}, "expect": {"ok": true}},
  {"name": "none", "manifest": {"scopes": []}, "expect": {"ok": true}}]}
JSON
  cat > "$root/snippets/kit/context.ts" <<'TS'
// lingara:begin context
const line = `${p.sets_completed} of ${p.set_count} sets done`;
// lingara:end
TS
}

# Runs the check on $1, which must fail and print every remaining argument.
expect_failure() {
  local root="$1"; shift
  local out; out="$(mktemp)"
  if bash "$check" "$root" > "$out"; then cat "$out"; echo "FAIL: $root passed"; return 1; fi
  for want in "$@"; do
    grep -qF -- "$want" "$out" || { cat "$out"; echo "FAIL: missing '$want'"; return 1; }
  done
}

expect_success() {
  local out; out="$(mktemp)"
  bash "$check" "$1" > "$out" || { cat "$out"; echo "FAIL: $1 was refused"; return 1; }
}

# 4.10.26e AC1: an unknown scope in any kit's call spelling fails, naming file
# and string; a real one passes, and D3's comment line is not a declaration.
test_a_scope_outside_the_spec_fails() {
  local tmp; tmp="$(mktemp -d)"
  trap 'rm -rf "$tmp"' RETURN
  write_tree "$tmp/bad"
  printf '  .scopes("plans:read")\n' > "$tmp/bad/snippets/kit/manifest.ts"
  printf '    scopes("plans:read")\n' > "$tmp/bad/snippets/kit/Manifest.kt"
  printf "        ->scopes('plans:read')\n" > "$tmp/bad/snippets/kit/manifest.php"
  printf '\t\tScopes("plans:read").\n' > "$tmp/bad/snippets/kit/manifest.go"
  expect_failure "$tmp/bad" \
    'snippets/kit/manifest.ts:1: scope "plans:read"' \
    'snippets/kit/Manifest.kt:1: scope "plans:read"' \
    'snippets/kit/manifest.php:1: scope "plans:read"' \
    'snippets/kit/manifest.go:1: scope "plans:read"'

  write_tree "$tmp/good"
  printf '  .scopes("lesson_plans:read")\n' > "$tmp/good/snippets/kit/manifest.ts"
  printf "        ->scopes('lesson_plans:read')\n" > "$tmp/good/snippets/kit/manifest.php"
  printf "  // \`.scopes(…)\` lists the API scopes your client uses, for the learner's consent page. This app uses none.\n" \
    > "$tmp/good/snippets/kit/comment.ts"
  printf "    # \`.scopes(…)\` lists the API scopes your client uses, for the learner's consent page. This app uses none.\n" \
    > "$tmp/good/snippets/kit/comment.rb"
  expect_success "$tmp/good"
}

# 4.10.26e AC2: an unknown scope in a manifest vector fails.
test_an_unknown_vector_scope_fails() {
  local tmp; tmp="$(mktemp -d)"
  trap 'rm -rf "$tmp"' RETURN
  write_tree "$tmp/t"
  cat > "$tmp/t/conformance/vectors/manifest.json" <<'JSON'
{"vectors": [{"name": "dup", "manifest": {"scopes": ["plans:read", "plans:read"]}, "expect": {"refused": "duplicate"}}]}
JSON
  expect_failure "$tmp/t" 'conformance/vectors/manifest.json: scope "plans:read"'
}

# 4.10.26e AC3: a lesson-plan read in a context region fails in all three
# spellings; the same call outside a context region does not.
test_a_lesson_plan_read_in_a_context_region_fails() {
  local tmp; tmp="$(mktemp -d)"
  trap 'rm -rf "$tmp"' RETURN
  local call
  for call in 'client.getLessonPlan(id)' 'client.get_lesson_plan(id)' 'client.GetLessonPlan(ctx, id)'; do
    write_tree "$tmp/in"
    printf '  # lingara:begin context\n  plan = %s\n  # lingara:end\n' "$call" > "$tmp/in/snippets/kit/app.rb"
    expect_failure "$tmp/in" "snippets/kit/app.rb:2: lesson-plan read in a context region; the app's client reads its owner's account"
    rm -rf "$tmp/in"
  done

  write_tree "$tmp/out"
  printf '// lingara:begin handler\nclient.getLessonPlan(id)\n// lingara:end\nclient.get_lesson_plan(id)\n' \
    > "$tmp/out/snippets/kit/handler.ts"
  expect_success "$tmp/out"
}

for t in test_a_scope_outside_the_spec_fails test_an_unknown_vector_scope_fails \
  test_a_lesson_plan_read_in_a_context_region_fails; do
  "$t"
  echo "✓ check_test.sh: $t"
done
