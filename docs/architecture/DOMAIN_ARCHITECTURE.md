# Domain Architecture — Activities, Models, and HTTP Handlers

## Overview

Each domain is self-contained and follows a layered architecture:
**Models** (SQL only) → **Activities** (business logic) → **HTTP
handlers** (thin wrappers that expose activities over the Axum API).

> **History:** The project briefly used Restate SDK 0.4.0 for durable
> workflow execution. Restate was removed on 2026-03-17 — our
> workloads are short request/response and didn't justify the
> runtime overhead. See `ARCHITECTURE_DECISIONS.md` Decision 4.

## Directory Structure

```
packages/server/src/domains/{domain}/
├── models/     # SQL queries + row structs (always present)
├── data/       # Shared data types (if the domain has an API surface)
├── activities/ # Pure async business-logic functions
├── loader.rs   # (optional) DataLoader for N+1 avoidance
└── mod.rs      # Domain module exports

packages/server/src/api/routes/{domain}.rs
  # HTTP handlers for the domain. Request/response types + thin
  # handler fns that delegate to activities.
```

HTTP routes live **outside** the domain, in `src/api/routes/`. Routes
are a cross-cutting concern (authorization, path shape, serialization)
and keeping them separate lets the same activity be called from
multiple handlers or from tests.

## Layer Responsibilities

### 1. Models (`models/`)

**Purpose**: SQL persistence — database queries *only*.

**Rules**:
- ALL SQL queries live in this directory.
- No queries outside `models/`.
- No business logic (auth checks, branching on state, etc.).
- Use `sqlx::FromRow` for row mapping.
- Always use `sqlx::query_as::<_, Self>()` — never the `query_as!`
  macro.

**Example** (from `organization/models/organization.rs`):

```rust
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Organization {
    pub id: OrganizationId,
    pub name: String,
    pub description: Option<String>,
    pub status: String,
    pub submitted_by: Option<MemberId>,
    pub last_extracted_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Organization {
    pub async fn find_by_id(id: OrganizationId, pool: &PgPool) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM organizations WHERE id = $1"
        )
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    pub async fn create(
        name: &str,
        description: Option<&str>,
        submitter_type: &str,
        pool: &PgPool,
    ) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            "INSERT INTO organizations (name, description, submitter_type, status)
             VALUES ($1, $2, $3, 'pending_review') RETURNING *",
        )
        .bind(name)
        .bind(description)
        .bind(submitter_type)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }
}
```

### 2. Data (`data/`)

**Purpose**: Serializable shared types used by multiple places in the
domain — typically the `ExtractedX` input types that feed
`create_extracted_*` activities, or request/response DTOs shared
across handlers.

**Rules**:
- Implement `Serialize + Deserialize`.
- Convert from models via `From<Model>` where useful.

### 3. Activities (`activities/`)

**Purpose**: Business logic. Pure async functions taking
`&ServerDeps` explicitly.

**Rules**:
- Take `&ServerDeps` as a parameter — never hold it as state.
- Return plain data types (`Result<Post>`, `Result<Uuid>`, etc.).
- Can call models for database access.
- Can call external services via `deps.*` (LLM, Twilio, storage).
- Use `deps.memo()` for caching expensive LLM calls.
- Stateless — no fields, no `&mut self`. Just `pub async fn`.

**Example**:

```rust
// domains/posts/activities/core.rs
pub async fn admin_create_post(
    title: String,
    body_raw: String,
    post_type: String,
    member_id: Uuid,
    deps: &ServerDeps,
) -> Result<Post> {
    let post = Post::create(
        CreatePost::builder()
            .title(title)
            .body_raw(body_raw)
            .status("draft".to_string())
            .submission_type(Some("admin".to_string()))
            .submitted_by_id(Some(member_id))
            .post_type(Some(post_type))
            .build(),
        &deps.db_pool,
    )
    .await?;
    Ok(post)
}
```

### 4. HTTP Handlers (`src/api/routes/{domain}.rs`)

**Purpose**: Expose activities over HTTP. Handle auth, request
parsing, response serialization, and URL routing.

**Rules**:
- Handlers are **thin** — parse input, call an activity, serialize
  output.
- Use auth extractors (`AdminUser`, `AuthenticatedUser`,
  `OptionalUser`) for authorization.
- Return `ApiResult<Json<T>>`. `ApiError` handles the error shape.
- No business logic in handlers — delegate to activities.

**Example**:

```rust
// src/api/routes/posts.rs
#[derive(Debug, Deserialize)]
pub struct AdminCreatePostRequest {
    pub title: String,
    pub body_raw: String,
    pub post_type: String,
}

async fn admin_create(
    State(state): State<AppState>,
    user: AdminUser,
    Json(req): Json<AdminCreatePostRequest>,
) -> ApiResult<Json<PostResult>> {
    let post = activities::admin_create_post(
        req.title,
        req.body_raw,
        req.post_type,
        user.0.member_id.into_uuid(),
        &state.deps,
    )
    .await?;
    Ok(Json(PostResult::from(post)))
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/Posts/admin_create", post(admin_create))
        // ... other handlers
}
```

### URL Conventions

- `/{Service}/{action}` — collection-level actions. e.g. `POST
  /Posts/list_posts`, `POST /Posts/admin_create`.
- `/{Object}/{id}/{action}` — instance-level actions. e.g. `POST
  /Post/{id}/approve`.

## Data Flow

### Writes

```
1. Next.js component dispatches a GraphQL mutation.
     ↓
2. GraphQL resolver (packages/shared/graphql/resolvers/) translates
   the mutation into an HTTP call via ctx.server.callService(...).
     ↓ HTTP/JSON
3. Axum handler in src/api/routes/{domain}.rs parses the request,
   runs the auth extractor, calls into the domain's activity.
     ↓
4. Activity runs business logic, calls model methods for persistence,
   returns a plain data type.
     ↓
5. Model executes sqlx queries against PostgreSQL.
     ↓
6. Handler serializes the activity's result and returns JSON.
```

### Reads

Same flow, often without the activity layer — simple reads can go
handler → model directly when there's no meaningful logic to put in
an activity.

## Registration

All domain routers are mounted in the root router. See
`src/bin/server.rs` (or `src/lib.rs`):

```rust
let app = Router::new()
    .nest("/", crate::api::routes::auth::router())
    .nest("/", crate::api::routes::posts::router())
    .nest("/", crate::api::routes::editions::router())
    .nest("/", crate::api::routes::widgets::router())
    // ... all other domains
    .with_state(AppState { deps: server_deps });
```

## Summary

| Layer        | Responsibility         | Can Do                          | Cannot Do                 |
|--------------|------------------------|---------------------------------|---------------------------|
| `models`     | Database persistence    | SQL queries, FromRow            | Business logic, IO beyond SQL |
| `data`       | Shared DTOs             | Serialization, From<Model>      | SQL queries               |
| `activities` | Business logic          | Call models, external IO, auth  | Hold state                |
| `api/routes` | HTTP surface + auth     | Parse requests, call activities | Business logic, direct SQL |

## Anti-Patterns to Avoid

- SQL queries outside `models/`.
- Business logic in HTTP handlers — keep them thin, delegate to
  activities.
- Using the `sqlx::query_as!` macro — always use the function version
  (`sqlx::query_as::<_, Self>(...)`). Enforced by a pre-commit hook.
- Fat handlers that inline what should be an activity.
- Activities that hold `ServerDeps` in a struct field instead of
  taking it as a parameter.
