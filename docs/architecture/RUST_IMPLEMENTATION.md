# Root Editorial — Rust Implementation

## Core Philosophy

**We are building a CMS for community journalism.**

Root Editorial is an open CMS that helps non-technical editors curate
and publish community-focused content. The Rust server provides HTTP
endpoints for content lifecycle management, editorial tooling, and
AI-assisted content processing.

---

## Architecture

The server is a single Rust crate (`packages/server/`) built with
**Axum** as a plain HTTP/JSON service.

```
Next.js Frontend (admin-app :3000, web-app :3001)
    ↓ HTTPS + GraphQL
GraphQL resolvers (in-process in Next.js API routes)
    ↓ HTTP/JSON
Rust Axum Server (port 9080)
    ├── src/api/routes/   — one file per service (posts, editions, widgets, …)
    ├── src/domains/      — business domains, each with models/ + activities/
    └── src/kernel/       — ServerDeps (db pool, AI client, storage adapter)
    ↓
PostgreSQL + pgvector
```

### Key Components

- **Business domains** in `src/domains/` (auth, member, posts,
  editions, organization, notes, tags, widgets, media, …).
- **HTTP routes** in `src/api/routes/{domain}.rs` — thin `async fn`
  handlers that delegate to activity functions.
- **Activities** — pure async functions with business logic, take
  `&ServerDeps`.
- **Models** — SQL persistence with `sqlx::query_as::<_, Self>()`.
- **ServerDeps** — central dependency container (`Arc<ServerDeps>`)
  holding the DB pool, AI client, and storage adapter.

---

## Core Data Models

```rust
// domains/organization/models/organization.rs
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Organization {
    pub id: OrganizationId,
    pub name: String,
    pub description: Option<String>,
    pub status: String,          // pending_review, approved, rejected, suspended
    pub submitted_by: Option<MemberId>,
    pub last_extracted_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// domains/posts/models/post.rs
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Post {
    pub id: PostId,
    pub title: String,
    pub description: String,
    pub post_type: String,       // service, opportunity, business (expanding)
    pub category: String,
    pub status: String,          // pending_approval, active, filled, rejected, expired
    pub urgency: Option<String>, // low, medium, high, urgent
    pub location: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub source_url: Option<String>,
    pub embedding: Option<pgvector::Vector>,
    pub created_at: DateTime<Utc>,
}

// domains/source/models/source.rs
pub struct Source {
    pub id: SourceId,
    pub organization_id: OrganizationId,
    pub source_type: String,     // website, newsletter, etc.
    pub status: String,
    pub last_crawled_at: Option<DateTime<Utc>>,
}
```

---

## Technical Stack

| Component         | Technology                |
|-------------------|--------------------------|
| HTTP              | Axum 0.7 + Tokio         |
| Database          | PostgreSQL + pgvector + sqlx |
| LLM               | OpenAI (GPT)             |
| Auth              | Twilio Verify (phone/email OTP) + JWT |
| Storage           | S3-compatible (MinIO in dev, AWS S3 in prod) |
| Frontend          | Next.js (App Router)     |
| Vector search     | pgvector (cosine similarity) |
| GraphQL           | Shared schema + resolvers (packages/shared) |

---

## Evolution Path

```
Current:
→ CMS: Post lifecycle, org management, editorial notes, admin panel

Next:
→ Root Signal integration: consume discovered content
→ Broadsheet: 3-column edition layout engine
→ Post type expansion: 12+ content types

Future:
→ Email newsletter generation
→ Weather widget
→ Multi-instance theming
```
