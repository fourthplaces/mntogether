#!/usr/bin/env bash
# guard-db-commands.sh — PreToolUse hook for Bash tool
#
# Blocks database-modifying commands and injects CLAUDE.md Rule Zero
# into Claude's near-memory context. Read-only commands pass through.

set -euo pipefail

INPUT=$(cat)
COMMAND=$(echo "$INPUT" | jq -r '.tool_input.command // empty')

# Nothing to check if there's no command
if [[ -z "$COMMAND" ]]; then
  exit 0
fi

# Normalize: lowercase for pattern matching, collapse whitespace
LOWER_CMD=$(echo "$COMMAND" | tr '[:upper:]' '[:lower:]' | tr -s '[:space:]' ' ')

# ─── Read-only whitelist ───────────────────────────────────────────
# These patterns are safe even when they appear inside psql/docker exec.
# Check these FIRST so they don't get caught by the broader SQL block.
is_readonly() {
  local cmd="$1"

  # pg_dump is always read-only
  if echo "$cmd" | grep -qE 'pg_dump'; then
    return 0
  fi

  # psql meta-commands: \dt, \d, \l, \dn, \di, \df, \du, \dx, \dv, \ds
  if echo "$cmd" | grep -qE '\\\\d[tldnifuxvs]'; then
    return 0
  fi

  # Pure SELECT (no INSERT/UPDATE/DELETE/ALTER/DROP/TRUNCATE/CREATE)
  # Must contain SELECT and not contain any write keywords
  if echo "$cmd" | grep -qiE '\bselect\b'; then
    if ! echo "$cmd" | grep -qiE '\b(insert|update|delete|alter|drop|truncate|create|grant|revoke)\b'; then
      return 0
    fi
  fi

  return 1
}

# ─── Dangerous pattern detection ──────────────────────────────────

# 1. Migration execution and sqlx database commands
if echo "$LOWER_CMD" | grep -qE 'cargo\s+sqlx\s+migrate|sqlx\s+migrate\s+run|sqlx\s+database\s+(reset|drop|create)'; then
  jq -n '{
    "hookSpecificOutput": {
      "hookEventName": "PreToolUse",
      "permissionDecision": "deny",
      "permissionDecisionReason": "BLOCKED: Database migration command detected.",
      "additionalContext": "RULE ZERO: NEVER MODIFY THE DATABASE WITHOUT EXPLICIT PERMISSION.\n\nThis command was blocked because it runs database migrations.\n\nYou MUST:\n1. Tell the user what migration you want to run and why\n2. STOP and WAIT for explicit approval (\"go\", \"do it\", \"yes\", \"proceed\")\n3. Only then execute the command\n\nWriting migration FILES is OK. RUNNING them is NEVER OK without explicit permission.\n\nThis rule exists because it was violated and caused a production incident."
    }
  }'
  exit 0
fi

# 2. SQL commands via psql, docker exec, or direct invocation
# First check if this even involves a database pathway
if echo "$LOWER_CMD" | grep -qE 'psql|docker.*(exec|compose).*postgres'; then

  # Allow read-only commands through
  if is_readonly "$LOWER_CMD"; then
    exit 0
  fi

  # Check for write SQL keywords
  if echo "$LOWER_CMD" | grep -qiE '\b(insert|update|delete|alter|drop|truncate|create|grant|revoke)\b'; then
    jq -n '{
      "hookSpecificOutput": {
        "hookEventName": "PreToolUse",
        "permissionDecision": "deny",
        "permissionDecisionReason": "BLOCKED: Database-modifying SQL detected.",
        "additionalContext": "RULE ZERO: NEVER MODIFY THE DATABASE WITHOUT EXPLICIT PERMISSION.\n\nThis command was blocked because it contains SQL that modifies database state (INSERT, UPDATE, DELETE, ALTER, DROP, TRUNCATE, CREATE, GRANT, or REVOKE).\n\nYou MUST:\n1. Tell the user exactly what SQL you want to run\n2. STOP and WAIT for explicit approval (\"go\", \"do it\", \"yes\", \"proceed\")\n3. Only then execute the command\n\nRead-only commands (SELECT, \\dt, pg_dump) are allowed without permission.\n\nThis rule exists because it was violated and caused a production incident."
      }
    }'
    exit 0
  fi

  # Piping a file into psql (e.g., `< dump.sql`) — could contain anything
  if echo "$LOWER_CMD" | grep -qE '<\s*\S+\.sql|cat.*\.sql.*\|\s*.*psql'; then
    jq -n '{
      "hookSpecificOutput": {
        "hookEventName": "PreToolUse",
        "permissionDecision": "deny",
        "permissionDecisionReason": "BLOCKED: SQL file import detected.",
        "additionalContext": "RULE ZERO: NEVER MODIFY THE DATABASE WITHOUT EXPLICIT PERMISSION.\n\nThis command was blocked because it pipes a SQL file into the database. The file may contain INSERT, ALTER, DROP, or other state-modifying statements.\n\nYou MUST:\n1. Tell the user what SQL file you want to import and why\n2. STOP and WAIT for explicit approval (\"go\", \"do it\", \"yes\", \"proceed\")\n3. Only then execute the command\n\nThis rule exists because it was violated and caused a production incident."
      }
    }'
    exit 0
  fi

  # Drop/create database commands
  if echo "$LOWER_CMD" | grep -qiE '(drop|create)\s+database'; then
    jq -n '{
      "hookSpecificOutput": {
        "hookEventName": "PreToolUse",
        "permissionDecision": "deny",
        "permissionDecisionReason": "BLOCKED: Database drop/create detected.",
        "additionalContext": "RULE ZERO: NEVER MODIFY THE DATABASE WITHOUT EXPLICIT PERMISSION.\n\nThis command was blocked because it drops or creates a database — a destructive, irreversible operation.\n\nYou MUST:\n1. Tell the user exactly what you want to do and why\n2. STOP and WAIT for explicit approval (\"go\", \"do it\", \"yes\", \"proceed\")\n3. Only then execute the command\n\nThis rule exists because it was violated and caused a production incident."
      }
    }'
    exit 0
  fi
fi

# ─── Everything else passes through ───────────────────────────────
exit 0
