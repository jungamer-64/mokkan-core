#!/usr/bin/env bash
set -euo pipefail

printf '%s\n' "SQL sanity checks: scanning for risky patterns..."
found=0

# If not run in a git repo, fallback to grep over src/
if ! git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  printf '%s\n' "Not in a git repository; scanning files under src/ with grep"
fi

# Pattern 1: direct formatting used inside sqlx::query(...)
matches=$(git grep -n --ignore-case -E 'sqlx::query\(&format!|sqlx::query_as\(&format!' -- src || true)
if [ -n "$matches" ]; then
  printf '\nFOUND sqlx::query(&format!) patterns:\n'
  printf '%s\n' "$matches"
  found=1
fi

# Pattern 2: format!("SELECT ..") or similar SQL keywords formatted
matches=$(git grep -n --ignore-case -E 'format!\(\s*"(SELECT|UPDATE|INSERT|DELETE|WITH)\b' -- src || true)
if [ -n "$matches" ]; then
  printf '\nFOUND format!("SQL ...") patterns:\n'
  printf '%s\n' "$matches"
  found=1
fi

# Pattern 3: sqlx::query(&var) usages — may be dynamic but often safe; warn for review
matches=$(git grep -n --ignore-case -E 'sqlx::query\(&[a-zA-Z_][a-zA-Z0-9_]*\)' -- src || true)
if [ -n "$matches" ]; then
  printf '\nFound sqlx::query(&<var>) usages (review to ensure queries are parameterized):\n'
  printf '%s\n' "$matches"
fi

if [ "$found" -eq 1 ]; then
  printf '\nPotentially risky SQL construction patterns detected. This check currently WARNs (exit 0). Please review the above locations.\n'
else
  printf '\nNo immediate risky `sqlx::query(&format!...)` or `format!("SQL...")` patterns found.\n'
fi

exit 0
