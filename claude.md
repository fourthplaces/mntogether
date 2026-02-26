# Root Editorial — Claude Code Rules

## Database Safety

Enforced by hooks in `.claude/hooks/` (registered in `.claude/settings.json`):

- **DB commands** → `guard-db-commands.sh` blocks migrations, write SQL, and file imports. Read-only queries pass through.
- **Migration files** → `guard-migrations.sh` blocks edits/overwrites of existing files. New migrations are allowed.
- **Code patterns** → `guard-code-patterns.sh` warns on `query_as!` macro and raw SQL in tests.

---

## Rust Conventions

### SQLx: Use `query_as` function, never the macro

```rust
// ✅ Always
sqlx::query_as::<_, Self>("SELECT * FROM posts WHERE id = $1")
    .bind(id)
    .fetch_one(pool)
    .await
    .map_err(Into::into)

// ❌ Never
sqlx::query_as!(Self, "SELECT * FROM posts WHERE id = $1", id)
```

Derive `FromRow` on structs. Use `.bind()` for params. `Option<T>` for nullable fields.

### SQL queries live in models

All database queries go in `domains/*/models/`. Never in Restate handlers or activities.

### Avoid JSONB

Normalize into relational tables. JSONB only for truly unstructured data (external API responses, arbitrary metadata).

### Activities are pure functions

Take `&ServerDeps` explicitly, return simple data types. No held state.

```rust
pub async fn approve_post(post_id: PostId, deps: &ServerDeps) -> Result<PostId> { ... }
```

---

## Restate Workflows

Architecture: `Next.js → Restate Ingress → Rust Server → PostgreSQL`

### Ports

| Service | Port | Notes |
|---|---|---|
| Admin app (Next.js) | 3000 | `packages/admin-app` — default `next dev` port |
| Web app (Next.js) | 3001 | `packages/web-app` — runs with `--port 3001` |
| Restate Ingress | 8180 | HTTP API for service calls (mapped from container 8080) |
| Restate Admin | 9070 | Service discovery, deployments, introspection |
| Rust Server | 9080 | Restate endpoint (h2c, not directly callable via curl) |
| PostgreSQL | 5432 | Docker container |
| Redis | 6379 | Docker container |

The Rust server auto-registers with Restate on startup — no manual `register-workflows.sh` needed after `docker compose up -d --build server`.

### Pattern

```rust
// 1. Request type with Restate serde
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MyRequest { pub id: Uuid }
impl_restate_serde!(MyRequest);

// 2. Trait (no &self or ctx in signature)
#[restate_sdk::workflow]
pub trait MyWorkflow {
    async fn run(req: MyRequest) -> Result<MyResponse, HandlerError>;
}

// 3. Impl with Arc<ServerDeps>
pub struct MyWorkflowImpl { deps: Arc<ServerDeps> }
impl MyWorkflowImpl {
    pub fn with_deps(deps: Arc<ServerDeps>) -> Self { Self { deps } }
}

// 4. Implementation adds &self and ctx
impl MyWorkflow for MyWorkflowImpl {
    async fn run(&self, ctx: WorkflowContext<'_>, req: MyRequest) -> Result<MyResponse, HandlerError> {
        ctx.run(|| async {
            activities::do_thing(req.id, &self.deps).await.map_err(Into::into)
        }).await
    }
}

// 5. Register in server.rs
.bind(MyWorkflowImpl::with_deps(deps.clone()).serve())
```

Key rules:
- `impl_restate_serde!` on all request/response types (bridges Restate SDK serde ≠ serde)
- Wrap external calls in `ctx.run()` for durability
- Keep workflows thin — delegate to activities
- Import both trait and impl for `.serve()` to compile

### Dev Setup

```bash
docker compose up -d                          # All infrastructure + Rust server
# Server auto-registers with Restate on startup

# Frontend (run from repo root):
cd packages/admin-app && yarn dev             # Admin on :3000
cd packages/web-app && yarn dev --port 3001   # Public site on :3001
```

After Rust code changes: `docker compose up -d --build server`
After new migrations: `make migrate` then rebuild server
