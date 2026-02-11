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

## Revision: prefix whitelist removed (v2)

### The bug

The initial version of `guard-db-commands.sh` included a safe-command prefix
whitelist — it extracted the first token of the Bash command (`git`, `echo`,
`cat`, etc.) and exited early if it matched, skipping all pattern checks. This
was added to prevent false positives when DB keywords appeared inside arguments
(e.g., `git commit -m "fix DROP DATABASE migration"` was being blocked because
the hook saw `DROP DATABASE` in the commit message).

The whitelist fixed the false positive but introduced a bypass: `cat` was in
the safe list, so `cat dump.sql | psql -U postgres -d db` would match `cat` as
the first token, exit early, and never reach the SQL file import check. The
entire pipe-to-psql detection was silently disabled for any command starting
with `cat`.

This went undetected during initial testing because the live hook (which scans
the raw Bash tool command) would catch test payloads containing
`cat dump.sql | psql` as a substring — making it *appear* blocked when it was
actually the test harness being blocked, not the payload.

### The fix

Removed the prefix whitelist entirely. Instead, each dangerous pattern check is
now self-sufficient:

1. **`cargo sqlx` commands** — matches a specific enough pattern
   (`cargo sqlx migrate`, `sqlx database reset|drop|create`) that it won't
   false-positive on commit messages or echoed strings.
2. **SQL write keywords and file imports** — gated behind a DB pathway check
   (`psql` or `docker exec/compose postgres` must be present in the command).
   A `git commit -m "INSERT INTO"` passes because there's no DB pathway.
3. **`DROP/CREATE DATABASE`** — moved inside the DB pathway gate (was
   previously a standalone check that matched bare keywords anywhere).

The key insight: you can't execute SQL without a client (`psql`, `docker exec`).
Checking for the client's presence is more robust than trying to enumerate every
safe command prefix.

### Testing challenges

**The hook-sees-itself problem.** Claude Code's PreToolUse hook receives the
entire raw Bash command string, including string literals, heredocs, and quoted
arguments. When Claude tries to run a test like:

```bash
echo '{"tool_input":{"command":"cargo sqlx migrate run"}}' | .claude/hooks/guard-db-commands.sh
```

The live hook sees `cargo sqlx migrate run` in the Bash command and blocks it
before the test ever reaches the script. This applies to any test approach where
dangerous strings appear in the command text — Python scripts with inline test
data, heredocs, etc.

**The workaround:** Write the test script to a file (using the Write tool, which
is not matched by the Bash hook), then execute the file. The Bash command
becomes `python3 .claude/hooks/test_db_guard.py` — no dangerous strings for the
live hook to intercept.

**Hook caching.** Hooks load at session start. Edits to hook scripts on disk
don't take effect until a new session (`/clear`). This means you can't test a
fix in the same session you write it — the old version keeps running.

### Test results (21/21 passing)

**Should DENY (9 tests):**

| Command | Result |
|---------|--------|
| `cargo sqlx migrate run` | denied |
| `cargo sqlx database reset` | denied |
| `psql -c "INSERT INTO posts ..."` | denied |
| `docker compose exec postgres psql -c "ALTER TABLE ..."` | denied |
| `psql -c "DROP DATABASE mydb"` | denied |
| `psql -d db < dump.sql` | denied |
| `psql -c "TRUNCATE TABLE posts"` | denied |
| `cat dump.sql \| psql -U postgres -d db` | denied (was bypassing) |
| `docker exec postgres psql -c "DELETE FROM posts"` | denied |

**Should PASS (12 tests):**

| Command | Result |
|---------|--------|
| `psql -c "SELECT count(*) FROM posts"` | passed |
| `pg_dump -U postgres -d db > dump.sql` | passed |
| `docker compose exec postgres psql -c "\dt"` | passed |
| `cargo build --release` | passed |
| `cargo test` | passed |
| `docker compose up -d` | passed |
| `git commit -m "fix DROP DATABASE migration"` | passed (was blocked) |
| `git commit -m "add INSERT for seed data"` | passed (was blocked) |
| `echo "INSERT INTO posts"` | passed |
| `cat src/main.rs` | passed |
| `ls packages/server/migrations/` | passed |
| `grep -r "INSERT" src/` | passed |

### Additional live test results

These were tested by having Claude attempt real tool calls in a session:

| Hook | Test | Result |
|------|------|--------|
| guard-db-commands | `cargo sqlx migrate run` (Bash) | denied |
| guard-db-commands | `SELECT count(*)` via psql (Bash) | passed |
| guard-db-commands | `INSERT INTO` via psql (Bash) | denied |
| guard-db-commands | `pg_dump` (Bash) | passed |
| guard-db-commands | `DROP DATABASE` (Bash) | denied |
| guard-db-commands | `psql -c "\dt"` (Bash) | passed |
| guard-migrations | Edit existing migration (Edit) | denied |
| guard-migrations | Write overwrite existing migration (Write) | denied |
| guard-migrations | Write new migration (Write) | allowed + reminder |
| guard-code-patterns | `query_as!()` in .rs file (Edit) | warning |

## Important notes

- Hooks are project-level (`.claude/settings.json`), so they apply to anyone
  who clones the repo and uses Claude Code.
- Hook scripts must be executable (`chmod +x`).
- The `$CLAUDE_PROJECT_DIR` variable is set by Claude Code at runtime.
- Hard blocks use `permissionDecision: "deny"` — the action is prevented.
- Soft warnings use only `additionalContext` — the action proceeds but Claude
  sees the reminder.
