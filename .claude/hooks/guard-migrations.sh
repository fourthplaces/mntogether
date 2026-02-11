#!/usr/bin/env bash
# guard-migrations.sh — PreToolUse hook for Edit and Write tools
#
# Blocks edits to EXISTING migration files. Allows creating new ones.
# Injects the migration immutability rule into Claude's near-memory context.

set -euo pipefail

INPUT=$(cat)
TOOL_NAME=$(echo "$INPUT" | jq -r '.hook_event_name // empty')

# Get the file path from tool input (both Edit and Write use file_path)
FILE_PATH=$(echo "$INPUT" | jq -r '.tool_input.file_path // empty')

# Nothing to check if no file path
if [[ -z "$FILE_PATH" ]]; then
  exit 0
fi

# Only care about migration files
if ! echo "$FILE_PATH" | grep -q 'packages/server/migrations/'; then
  exit 0
fi

# ─── Edit tool: ALWAYS block on existing migration files ──────────
# The Edit tool by definition modifies existing files.
TOOL=$(echo "$INPUT" | jq -r '.tool_name // empty')

if [[ "$TOOL" == "Edit" ]]; then
  jq -n '{
    "hookSpecificOutput": {
      "hookEventName": "PreToolUse",
      "permissionDecision": "deny",
      "permissionDecisionReason": "BLOCKED: Attempted edit of existing migration file.",
      "additionalContext": "MIGRATION FILES ARE SACRED — DO NOT TOUCH.\n\nNEVER edit an existing migration file. NEVER. SQLx checksums every migration file. If you modify a file that has been applied, it WILL break production deployments. There is no recovery without manual DB surgery.\n\nDO NOT edit files in packages/server/migrations/ that already exist.\nDO NOT \"fix\" typos in existing migrations.\nDO NOT add lines to existing migrations.\nDO NOT remove lines from existing migrations.\nDO NOT reformat existing migrations.\n\nIF YOU NEED TO FIX A MIGRATION: Create a NEW migration file with the next sequential number.\n\nThis rule exists because it was violated and caused production incidents."
    }
  }'
  exit 0
fi

# ─── Write tool: block if the file already exists on disk ─────────
# Writing a NEW migration is fine. Overwriting an existing one is not.
if [[ "$TOOL" == "Write" ]]; then
  if [[ -f "$FILE_PATH" ]]; then
    jq -n '{
      "hookSpecificOutput": {
        "hookEventName": "PreToolUse",
        "permissionDecision": "deny",
        "permissionDecisionReason": "BLOCKED: Attempted overwrite of existing migration file.",
        "additionalContext": "MIGRATION FILES ARE SACRED — DO NOT TOUCH.\n\nThis file already exists on disk. You cannot overwrite it.\n\nSQLx checksums every migration file. If you modify a file that has been applied, it WILL break production deployments.\n\nIF YOU NEED TO FIX A MIGRATION: Create a NEW migration file with the next sequential number.\n\nThis rule exists because it was violated and caused production incidents."
      }
    }'
    exit 0
  fi

  # New migration file — allow but remind about not running it
  jq -n '{
    "hookSpecificOutput": {
      "hookEventName": "PreToolUse",
      "additionalContext": "Migration file is being created. Remember: Do NOT run `cargo sqlx migrate run` or any migration command without explicit user permission. Tell the user the file was created and wait for them to say \"go\" or \"do it\" before executing."
    }
  }'
  exit 0
fi

# ─── Anything else passes through ────────────────────────────────
exit 0
