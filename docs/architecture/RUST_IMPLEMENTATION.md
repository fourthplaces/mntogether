# Root Editorial — Rust Implementation

## Core Philosophy

**We are building a CMS for community journalism.**

Root Editorial is an open CMS that helps non-technical editors curate and publish community-focused content. The Rust server provides durable workflows for content lifecycle management, editorial tooling, and AI-assisted content processing.

---

## Architecture

The server is a single Rust crate (`packages/server/`) using **Restate SDK 0.4.0** for durable workflow execution.

```
Next.js Frontend (admin-app :3000, web-app :3001)
    ↓ HTTP / GraphQL
Restate Runtime (port 8180 ingress, 9070 admin)
    ↓ HTTP
Rust Workflow Server (port 9080)
    ├── Services   — stateless request handlers
    ├── Workflows  — durable multi-step pipelines
    └── Objects    — keyed stateful entities
    ↓
PostgreSQL + pgvector
```

### Key Components

- **Business domains** in `src/domains/` (auth, member, posts, organization, notes, tags, etc.)
- **ServerDeps** — central dependency container (`Arc<ServerDeps>`)
- **Activities** — pure async functions with business logic
- **Models** — SQL persistence with `sqlx::query_as::<_, Self>()`
- **impl_restate_serde!** — macro bridging serde and Restate SDK serialization

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
| Runtime           | Restate SDK 0.4.0        |
| Database          | PostgreSQL + pgvector     |
| LLM               | OpenAI (GPT)             |
| Auth              | Twilio Verify (phone/email OTP) |
| Frontend          | Next.js (App Router)     |
| Vector search     | pgvector (cosine similarity) |
| GraphQL           | Shared schema (packages/shared) |

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
