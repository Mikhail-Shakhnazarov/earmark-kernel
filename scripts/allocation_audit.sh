#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

RUST_GLOB=(--glob '**/*.rs' --glob '!target/**' --glob '!**/tests/**')

count_matches() {
  local pattern="$1"
  { rg "$pattern" . "${RUST_GLOB[@]}" 2>/dev/null || true; } | wc -l | tr -d ' '
}

clone_count=$(count_matches "\\.clone\\(")
to_string_count=$(count_matches "\\.to_string\\(")
arc_str_count=$(count_matches "Arc<\\s*str\\s*>")
interner_count=$(count_matches "(?i)(string\\s*interner|interner)")

echo "allocation_audit"
echo "repo_root=$ROOT"
echo "clone_count=$clone_count"
echo "to_string_count=$to_string_count"
echo "arc_str_references=$arc_str_count"
echo "interner_references=$interner_count"

if [[ "$arc_str_count" != "0" ]]; then
  echo "error: Arc<str> usage detected; this roadmap keeps owned String in persisted models" >&2
  exit 1
fi
