# Root Editorial — Claude Code Rules

## Reading This Codebase

This repo has two eras. It was once a monolith that contained both Root Signal (scouting + extraction) and Root Editorial (CMS + publication). In early 2026 the two split into separate systems; this repo kept the Editorial half. The split produced a clean design on the runtime side but left archaeological layers in migrations, seed data, GraphQL types, and docs.

**When reading the repo, treat each surface differently:**

| Surface | Status | How to read it |
|---|---|---|
| Runtime code (`packages/server/src/`, resolvers, admin + web apps) | **Truth.** | This is the current design. When anything else disagrees with runtime, runtime wins. |
| [`docs/architecture/DATA_MODEL.md`](docs/architecture/DATA_MODEL.md) | **Canonical design.** | The living narrative description of the current data model. If it contradicts runtime, fix the doc; if it contradicts migrations, ignore the migrations. Update this doc as the design evolves. |
| Migrations (`packages/server/migrations/`) | **History, not design.** | Append-only log of changes someone made on a Tuesday. Many predate the pivot; many document attempted reworks that never fully shipped (see below). Never infer current design from migrations alone. |
| [`docs/architecture/DATABASE_SCHEMA.md`](docs/architecture/DATABASE_SCHEMA.md) | Stale. | Self-describes as covering through migration 171; schema is past 236. Regenerate or re-cross-reference against the actual schema before trusting. |
| Seed data (`data/*.json`) | Often lags. | Fields may exist in seed JSONs that the schema no longer stores — the seed loader silently drops them. Not authoritative. |
| GraphQL schema (`packages/shared/graphql/schema.ts`) | Mostly truth, some legacy. | Some fields (e.g. `Post.sourceUrl`, `Post.category`, `Post.urgency`) still exist but resolve against dropped columns or return null. Verify against runtime resolvers before treating as current surface area. |
| Older docs (pre-pivot, dated before 2026-03) | Legacy. | E.g. `ROOT_EDITORIAL_PIVOT.md` captures the pivot rationale and is a useful history reference, but its enumerations of "what Root Editorial does" reflect the intent of Feb 2026, not the shipped system. |

**The pivot boundary is approximately migration `000193`.** Migrations in the 189–193 range (`drop_agents_domain`, `drop_capacity_status`, `drop_memo_cache`, `drop_heat_map_points`, `drop_ai_columns`) are the pivot scrub. Anything before that range is pre-pivot; anything after is the current design layer.

**When a migration shows an "attempted rework" that the code doesn't reflect, the code's choice is the current design.** Example: migration `000197` reworked the tag taxonomy (split `service_area` into `county` + `city`, added `language` + `platform` + `verification` kinds) but the runtime never adopted it — layout engine and seed still use `kind = 'service_area'`. That means: current design = `service_area`. The migration is noise.

**If you find pre-pivot residue during a task** (dead seed fields, dead GraphQL types, dead tag kinds, dead columns no one reads), do not preserve it out of caution. Pre-pivot cruft is to be removed, not migrated around. Surface it in the relevant addendum / TODO entry and either scrub it inline (if trivial) or add it to the scrub backlog.

---

## Database Safety

Enforced by hooks in `.claude/hooks/` (registered in `.claude/settings.json`):

- **DB commands** → `guard-db-commands.sh` blocks migrations, write SQL, and file imports. Read-only queries pass through.
- **Migration files** → `guard-migrations.sh` blocks edits/overwrites of existing files. New migrations are allowed.
- **Code patterns** → `guard-code-patterns.sh` warns on `query_as!` macro and raw SQL in tests.

---

## Rust Conventions

### SQLx: Use `query_as` function, never the macro

```rust
// GOOD:
sqlx::query_as::<_, Self>("SELECT * FROM posts WHERE id = $1")
    .bind(id)
    .fetch_one(pool)
    .await
    .map_err(Into::into)

// BAD:
sqlx::query_as!(Self, "SELECT * FROM posts WHERE id = $1", id)
```

Derive `FromRow` on structs. Use `.bind()` for params. `Option<T>` for nullable fields.

### SQL queries live in models

All database queries go in `domains/*/models/`. Never in HTTP handlers or activities.

### Avoid JSONB

Normalize into relational tables. JSONB only for truly unstructured data (external API responses, arbitrary metadata).

### Activities are pure functions

Take `&ServerDeps` explicitly, return simple data types. No held state.

```rust
pub async fn approve_post(post_id: PostId, deps: &ServerDeps) -> Result<PostId> { ... }
```

---

## Architecture

`Next.js → GraphQL → Axum HTTP Server (9080) → Activities → PostgreSQL`

### Ports

| Service | Port | Notes |
|---|---|---|
| Admin app (Next.js) | 3000 | `packages/admin-app` — default `next dev` port |
| Web app (Next.js) | 3001 | `packages/web-app` — runs with `--port 3001` |
| Rust Server (Axum) | 9080 | HTTP/JSON API + SSE streams |
| PostgreSQL | 5432 | Docker container |

### HTTP Handler Pattern

```rust
// 1. Request/response types with standard serde
#[derive(Debug, Deserialize)]
pub struct MyRequest { pub id: Uuid }

#[derive(Debug, Serialize)]
pub struct MyResponse { pub name: String }

// 2. Axum handler — thin wrapper calling activities
async fn my_handler(
    State(state): State<AppState>,
    user: AdminUser,                    // Auth extractor (or AuthenticatedUser, OptionalUser)
    Json(req): Json<MyRequest>,
) -> ApiResult<Json<MyResponse>> {
    let result = activities::do_thing(req.id, &state.deps).await?;
    Ok(Json(result))
}

// 3. Register in api/routes/{domain}.rs
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/MyService/my_handler", post(my_handler))
}
```

Key rules:
- Handlers are thin — delegate business logic to `domains/*/activities/`
- Three auth extractors: `AdminUser`, `AuthenticatedUser`, `OptionalUser`
- `ApiError` returns `{"message": "..."}` JSON (Unauthorized/Forbidden/NotFound/BadRequest/Internal)
- URL paths follow `/{Service}/{handler}` or `/{Object}/{id}/{handler}` convention

### Dev Setup

```bash
docker compose up -d                          # All infrastructure + Rust server

# Frontend (run from repo root):
cd packages/admin-app && yarn dev             # Admin on :3000
cd packages/web-app && yarn dev --port 3001   # Public site on :3001
```

After Rust code changes: `docker compose up -d --build server`
After new migrations: `make migrate` then rebuild server

---

## Test-Driven Development

**RED → GREEN → REFACTOR** is mandatory for every feature, bug fix, or change to API edges.

1. **RED** — Write a failing test first. Run it. Confirm it fails for the right reason.
2. **GREEN** — Write the simplest code that makes it pass.
3. **REFACTOR** — Clean up while keeping tests green.

### Hard Rule: API Edge Testing Only

Test through Axum HTTP handlers and GraphQL endpoints. Verify via API response AND model queries. Mock external services via `TestDependencies`.

**Never** bypass the API with direct database manipulation, test internal implementation details, or call private functions.

```rust
// FORBIDDEN — bypasses the API layer
sqlx::query!("INSERT INTO posts ...", ...).execute(&pool).await;
```

### Use Models, Not Raw SQL

```rust
// GOOD:
let post = Post::find_by_id(id, pool).await?;
Post::approve(post_id, admin_id, pool).await?;

// BAD:
sqlx::query!("SELECT * FROM posts WHERE id = $1", id)
```

### Test Coverage

Every API edge must test: happy path, error cases, edge cases, state verification (via models), and API response shape.

### Test Structure

- Location: `packages/server/tests/`
- Naming: `{feature}_tests.rs`
- Harness: `#[test_context(TestHarness)]`
- One assertion per test

### Non-Negotiable

1. No code without tests — test first, see it fail, then implement
2. No bypassing the API — test through API edges only
3. No raw SQL in tests — use models and helpers
4. No deleting tests — fix them
5. No skipping refactor — clean code after green
6. Run `cargo test` before commit

---

## Debugging Discipline

Learned the hard way from a Turbopack CPU loop that burned 250% for hours before anyone noticed. See [postmortem](docs/status/TURBOPACK_CPU_LOOP_POSTMORTEM.md).

### Never suppress warnings or errors

Warnings exist to surface problems. Disabling them (`devIndicators: false`, `ignoreBuildErrors: true`, `// @ts-ignore`, `eslint-disable`) without understanding the root cause is forbidden. If a warning is genuinely irrelevant, document *why* before suppressing it.

### One variable at a time

When debugging, change exactly one thing, test it, then revert before testing the next hypothesis. Changing two things simultaneously (e.g., migrating a file AND restarting a container) makes it impossible to know which change fixed the problem. If you can't isolate the cause, you haven't found it.

### Restart processes after filesystem changes to watched files

Dev servers (Next.js, cargo-watch) cache file watcher state in memory. Removing or renaming a file that the watcher tracks (middleware.ts, layout.tsx, Cargo.toml) may not take effect without a process restart. Always restart the dev server after structural file changes, then test.

### Say "I don't know" instead of fabricating a narrative

If the fix works but you can't explain why, say so. Don't construct a confident-sounding root cause analysis around a coincidence. "Container restart fixed it, cause unknown" is more useful than a wrong explanation that prevents future investigation.

### Docker dev container recovery

```bash
# If a Next.js container is stuck (high CPU, "Compiling..." loop):
docker compose down <service> && docker compose up -d <service>

# If Turbopack is suspect, switch to webpack temporarily:
# In package.json: "dev": "next dev --webpack"
```

---

## Commit Discipline

### Don't commit until the user has tested and confirmed

Programmatic verification — simulated clicks, synthetic pointer events,
`curl` hitting an endpoint, a typecheck passing — is **not** the same as
the user actually performing the interaction in their browser. An
automated "it works" has been wrong enough times on this project that
it's no longer trusted.

When a change is ready:

1. Push it to the working tree and tell the user what to check.
2. Stop. Wait for their confirmation.
3. Only commit after they say it works.

If the user explicitly says "commit," that's the green light. Otherwise
a clean typecheck + a passing automated probe is a *starting point* for
their test, not a substitute for it.

---

## Documentation Organization

All documentation goes in `docs/`, not the project root. `README.md` is the only exception.

```
docs/
├── admin/           # Admin-specific guides and setup
├── architecture/    # System architecture and design documents
├── guides/          # Implementation guides, tutorials, and reference
├── prompts/         # LLM prompts used in the codebase
├── security/        # Security policies and authentication
├── setup/           # Setup and deployment instructions
└── status/          # Implementation status, postmortems, and progress reports
```

Before creating any `.md` file, place it in the appropriate `docs/` subdirectory and update `docs/README.md` if it's a major document.
