#!/usr/bin/env bash
# guard-code-patterns.sh — PreToolUse hook for Edit and Write tools
#
# Non-blocking warnings for code patterns that violate project conventions.
# These do NOT block the action — they inject reminders via additionalContext.

set -euo pipefail

INPUT=$(cat)
TOOL=$(echo "$INPUT" | jq -r '.tool_name // empty')
FILE_PATH=$(echo "$INPUT" | jq -r '.tool_input.file_path // empty')

if [[ -z "$FILE_PATH" ]]; then
  exit 0
fi

# Collect warnings — multiple may apply to the same edit
WARNINGS=""

# ─── Extract the content being written ────────────────────────────
# Edit tool: check old_string and new_string (we care about new_string)
# Write tool: check content
if [[ "$TOOL" == "Edit" ]]; then
  CONTENT=$(echo "$INPUT" | jq -r '.tool_input.new_string // empty')
elif [[ "$TOOL" == "Write" ]]; then
  CONTENT=$(echo "$INPUT" | jq -r '.tool_input.content // empty')
else
  exit 0
fi

if [[ -z "$CONTENT" ]]; then
  exit 0
fi

# ─── Check: query_as! macro (any Rust file) ──────────────────────
if echo "$FILE_PATH" | grep -qE '\.rs$'; then
  if echo "$CONTENT" | grep -qE 'query_as!\s*\('; then
    WARNINGS="${WARNINGS}SQLX RULE: Use sqlx::query_as::<_, Type>() function form, not the query_as!() macro. The macro requires compile-time DB access, fails on nullable fields, and slows builds.\n\n"
  fi
fi

# ─── Check: event bus in tests ────────────────────────────────────
if echo "$FILE_PATH" | grep -qE '(tests/|test_|_test\.rs)'; then

  if echo "$CONTENT" | grep -qE '\.bus\(\)\s*\.\s*send\(|\.settle\(\)'; then
    WARNINGS="${WARNINGS}TDD RULE: Never access the event bus directly in tests. Do not use bus().send() or settle(). Test through GraphQL mutations and queries using ctx.graphql() instead.\n\n"
  fi

  # ─── Check: raw SQL in tests ──────────────────────────────────
  if echo "$CONTENT" | grep -qE 'sqlx::query[^_]|sqlx::query!\s*\('; then
    WARNINGS="${WARNINGS}TDD RULE: Do not use raw SQL queries in tests. Use model methods instead (e.g., Domain::find_by_id(), Post::find_all()). Models are the data access layer.\n\n"
  fi
fi

# ─── Emit warnings if any ────────────────────────────────────────
if [[ -n "$WARNINGS" ]]; then
  # Escape for JSON
  ESCAPED=$(echo -e "$WARNINGS" | jq -Rs '.')
  jq -n --argjson ctx "$ESCAPED" '{
    "hookSpecificOutput": {
      "hookEventName": "PreToolUse",
      "additionalContext": $ctx
    }
  }'
  exit 0
fi

exit 0
