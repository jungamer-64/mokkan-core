#!/usr/bin/env bash
set -euo pipefail

echo "SQL sanity checks: scanning for risky patterns..."
found=0

# If not run in a git repo, fallback to grep over src/
if ! git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  echo "Not in a git repository; scanning files under src/ with grep"
fi

# Pattern 1: direct formatting used inside sqlx::query(...)
matches=$(git grep -n --ignore-case -E 'sqlx::query\(&format!|sqlx::query_as\(&format!' -- src || true)
if [ -n "$matches" ]; then
  echo "\nFOUND sqlx::query(&format!) patterns:"
  echo "$matches"
  found=1
fi

# Pattern 2: format!("SELECT ..") or similar SQL keywords formatted
matches=$(git grep -n --ignore-case -E 'format!\(\s*"(SELECT|UPDATE|INSERT|DELETE|WITH)\b' -- src || true)
if [ -n "$matches" ]; then
  echo "\nFOUND format!(\"SQL ...\") patterns:"
  echo "$matches"
  found=1
fi

# Pattern 3: sqlx::query(&var) usages — may be dynamic but often safe; warn for review
matches=$(git grep -n --ignore-case -E 'sqlx::query\(&[a-zA-Z_][a-zA-Z0-9_]*\)' -- src || true)
if [ -n "$matches" ]; then
  echo "\nFound sqlx::query(&<var>) usages (review to ensure queries are parameterized):"
  echo "$matches"
fi

if [ "$found" -eq 1 ]; then
  echo "\nPotentially risky SQL construction patterns detected. This check currently WARNs (exit 0). Please review the above locations."
else
  echo "\nNo immediate risky \`sqlx::query(&format!...)\` or \`format!(\"SQL...\")\` patterns found."
fi

exit 0
