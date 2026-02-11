# Claude Code Safety Hooks

## Overview

This project uses Claude Code PreToolUse hooks to enforce development rules at
the tool level. Instead of relying on all-caps instructions in CLAUDE.md (which
degrade as context windows grow), the hooks intercept dangerous actions before
they execute and inject the relevant rule into Claude's immediate context.

The previous approach relied on prompt-level rules in `CLAUDE.md` and
`.clauderules` that used repeated emphasis and all-caps formatting to try to
prevent violations. This failed in practice — Claude would comply early in a
session but drift as the rules got compacted out of the context window.

## What changed

### New files

| File | Purpose |
|------|---------|
| `.claude/settings.json` | Hook configuration (project-level, committed) |
| `.claude/hooks/guard-db-commands.sh` | Blocks DB-modifying Bash commands |
| `.claude/hooks/guard-migrations.sh` | Blocks edits to existing migration files |
| `.claude/hooks/guard-code-patterns.sh` | Warns on bad code patterns (non-blocking) |

### Modified files

| File | What changed |
|------|-------------|
| `CLAUDE.md` | Rewrote DB safety and migration sections. Removed all-caps shouting, replaced with calm references to the hooks as the enforcement mechanism. All other rules (SQLx patterns, domain structure, Restate workflows) preserved and tightened. |
| `.clauderules` | Rewrote TDD rules. Removed repeated "TDD TDD TDD" headers and "FORBIDDEN" formatting. Same rules, stated once clearly. |

## How the hooks work

### Architecture

```
Claude tries a tool call (Bash, Edit, Write)
    ↓
Claude Code fires PreToolUse event
    ↓
Matching hook script receives JSON via stdin
    ↓
Script checks patterns in the command/file path
    ↓
If dangerous: outputs JSON with permissionDecision: "deny" + additionalContext
If warning:   outputs JSON with additionalContext only (non-blocking)
If safe:      exits 0 silently
```

The `additionalContext` field is the key innovation — it injects text into
Claude's immediate working memory, right at the point of the blocked action.
This keeps the rule in "near memory" rather than relying on CLAUDE.md being
loaded at conversation start.

### Hook 1: guard-db-commands.sh (hard block)

Matches: `Bash` tool calls

**Blocks:**
- `cargo sqlx migrate run` and variants
- SQL write keywords (INSERT, UPDATE, DELETE, ALTER, DROP, TRUNCATE, CREATE,
  GRANT, REVOKE) via psql or docker exec
- SQL file imports (`< dump.sql`, `cat dump.sql | psql`)
- DROP/CREATE DATABASE commands

**Allows:**
- SELECT queries
- psql meta-commands (`\dt`, `\d`, `\l`, `\dn`, etc.)
- `pg_dump`
- All non-database Bash commands (`cargo build`, `docker compose up`, etc.)

When blocked, injects: Rule Zero text explaining that DB modifications require
explicit user approval ("go", "do it", "yes", "proceed").

### Hook 2: guard-migrations.sh (hard block)

Matches: `Edit` and `Write` tool calls

**Blocks:**
- Any `Edit` to a file in `packages/server/migrations/`
- Any `Write` that would overwrite an existing file in `packages/server/migrations/`

**Allows:**
- `Write` of a new migration file (file does not yet exist on disk)
  - Also injects a non-blocking reminder to not run it without permission

When blocked, injects: Migration immutability rule explaining SQLx checksums and
why modifying applied migrations breaks production.

### Hook 3: guard-code-patterns.sh (soft warning)

Matches: `Edit` and `Write` tool calls

**Warns (non-blocking) on:**
- `query_as!()` macro in any `.rs` file — should use `query_as::<_, Type>()`
- `bus().send()` or `settle()` in test files — should test through GraphQL
- `sqlx::query!()` in test files — should use model methods

These warnings are injected as `additionalContext` but do not block the action.
Claude receives the reminder and can self-correct.

## Testing the hooks

Hooks load at session start. If you modify `.claude/settings.json` or the hook
scripts, start a new session (`/clear`) to pick up changes.

### Manual script tests (work anytime)

```bash
# Test: migrate run → should output deny JSON
echo '{"tool_input":{"command":"cargo sqlx migrate run"},"tool_name":"Bash"}' \
  | .claude/hooks/guard-db-commands.sh

# Test: SELECT → should exit silently
echo '{"tool_input":{"command":"docker compose exec postgres psql -U postgres -d mndigitalaid -c \"SELECT count(*) FROM posts\""},"tool_name":"Bash"}' \
  | .claude/hooks/guard-db-commands.sh

# Test: edit existing migration → should output deny JSON
cat <<'EOF' | .claude/hooks/guard-migrations.sh
{"tool_input":{"file_path":"packages/server/migrations/000001_create_extensions.sql","old_string":"CREATE","new_string":"DROP"},"tool_name":"Edit"}
EOF

# Test: query_as! in Rust file → should output warning JSON
cat <<'EOF' | .claude/hooks/guard-code-patterns.sh
{"tool_input":{"file_path":"src/models/post.rs","new_string":"sqlx::query_as!( Post, \"SELECT *\" )"},"tool_name":"Edit"}
EOF
```

### Live tests (require new session)

In a new Claude Code session, try asking Claude to:
1. Run `cargo sqlx migrate run` — should be blocked
2. Edit an existing migration file — should be blocked
3. Run `INSERT INTO posts ...` via psql — should be blocked
4. Run `SELECT * FROM posts` via psql — should pass
5. Write code with `query_as!()` — should get a warning

## Important notes

- Hooks are project-level (`.claude/settings.json`), so they apply to anyone
  who clones the repo and uses Claude Code.
- Hook scripts must be executable (`chmod +x`).
- The `$CLAUDE_PROJECT_DIR` variable is set by Claude Code at runtime.
- Hard blocks use `permissionDecision: "deny"` — the action is prevented.
- Soft warnings use only `additionalContext` — the action proceeds but Claude
  sees the reminder.
