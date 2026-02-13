# MN Together - Rust Implementation

## Core Philosophy

**We are building a relevance notifier, not a matcher.**

### What Success Looks Like:
- Volunteers hear about opportunities they might help with
- Organizations get connected with potential helpers
- Discovery shifts from pull to push (people don't have to search)

### What We're NOT Optimizing For:
- Perfect matching accuracy
- Preventing all mismatches
- Guarantee of qualification

### Key Insight:
**Cost of false positive:** 2 seconds of attention, one ignored notification
**Cost of false negative:** Someone never hears about a need they could have helped with

> **Bias toward recall. Let humans self-select.**

---

## Architecture

The server is a single Rust crate (`packages/server/`) using **Restate SDK 0.4.0** for durable workflow execution.

```
Next.js Frontend (port 3000)
    ↓ HTTP
Restate Runtime (port 9070)
    ↓ HTTP
Rust Workflow Server (port 9080)
    ├── Services   — stateless request handlers
    ├── Workflows  — durable multi-step pipelines
    └── Objects    — keyed stateful entities
    ↓
PostgreSQL + pgvector + NATS
```

### Key Components

- **22 business domains** in `src/domains/`
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
    pub post_type: String,       // service, opportunity, business
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
    pub source_type: String,     // website, instagram, facebook, etc.
    pub status: String,
    pub last_crawled_at: Option<DateTime<Utc>>,
}
```

---

## Curator Pipeline (Main Workflow)

The curator is the core content pipeline — it processes organizations' web presence into actionable posts:

```
1. Load org + sources
   ↓
2. Gather crawled pages (via extraction service)
   ↓
3. Extract page briefs (LLM, memo-cached)
   ↓
4. Compile org document (deterministic)
   ↓
5. Run curator (single LLM call — the "reduce" step)
   ↓
5.5. Writer pass (rewrite post copy in parallel)
   ↓
5.7. Safety review (check eligibility restrictions)
   ↓
6. Stage actions as sync proposals
   ↓
7. Human review in admin panel
```

This is implemented as a Restate workflow in `domains/curator/restate/workflows/curate_org.rs`.

See [CURATOR_PIPELINE.md](./CURATOR_PIPELINE.md) for the full pipeline documentation.

---

## Notification Design (Critical: Tone Matters)

Notifications are invitational, not demanding:

- **Title**: "Thought you might be interested"
- **Explains why** they were notified (transparency)
- **Direct contact info** (we don't mediate)
- **Easy opt-down** (respects attention)
- **No pressure** — "just wanted to make sure you knew about it"

---

## What We're NOT Building (Yet)

- Complex matching rules
- Multi-stage filtering
- Confidence scoring
- Volunteer profiles with 20 fields
- In-app messaging
- A/B testing infrastructure

**Why not?** Because we don't know what matters yet. Let usage teach us.

---

## Evolution Path

```
MVP (Current):
→ Curator pipeline: scrape → brief → curate → propose → review

Learn (Next):
→ Which posts get engagement?
→ What do people ignore?
→ Where are the gaps?

Iterate (Future):
→ Add structure where it helps
→ Remove noise sources
→ Tune notification frequency
→ Consider reranking layer
```

---

## Technical Stack

| Component         | Technology                |
|-------------------|--------------------------|
| Runtime           | Restate SDK 0.4.0        |
| Database          | PostgreSQL + pgvector     |
| Messaging         | NATS                     |
| LLM               | OpenAI (GPT-4o, GPT-5-mini), Claude (Sonnet 4.5) |
| Web scraping      | Firecrawl, HTTP fallback  |
| Web search        | Tavily                   |
| Social scraping   | Apify                    |
| Auth              | Twilio Verify (phone OTP) |
| Push notifications | Expo                    |
| Frontend          | Next.js (App Router)     |
| Vector search     | pgvector (cosine similarity) |

**This is the right architecture.** Simple, evolvable, human-centered.
