# Emergency Resource Aggregator - Package Structure

## Overview

This document defines the complete package structure for mndigitalaid, following the organizational patterns from `~/Developer/fourthplaces/shay/packages/api-core`.

**Key Architectural Decisions:**
- 🔒 **Privacy-First**: Zero PII stored (no names, emails, phone numbers)
- 📦 **Single Crate**: Simplified to `api-core` (NOT 5 separate crates)
- 🎯 **Domain-Driven**: Each domain (volunteer, need, notification) is self-contained
- ⚡ **Event-Driven**: seesaw-rs architecture (events → machines → commands → effects)

---

## Project Root Structure

```
mndigitalaid/
├── Cargo.toml                       # Workspace root (single member: api-core)
├── .env                             # Environment variables
├── .sqlx/                           # SQLx compile-time query cache
├── rustfmt.toml                     # Code formatting rules
├── dev.toml                         # Development configuration
│
├── migrations/                      # Database migrations (SQLx)
│   ├── 001_create_extensions.sql
│   ├── 002_create_volunteers.sql   # ⚠️ Privacy-first schema (expo_push_token only)
│   ├── 003_create_needs.sql        # With markdown support
│   ├── 004_create_notifications.sql
│   ├── 005_create_indexes.sql      # HNSW vector indexes
│   └── 006_create_system_settings.sql # Global kill switch
│
├── src/                             # Main crate source (api-core)
│   ├── lib.rs                       # Public API exports
│   ├── config.rs                    # Configuration loading
│   ├── otel.rs                      # OpenTelemetry setup (optional)
│   │
│   ├── common/                      # Shared utilities (cross-domain)
│   │   ├── mod.rs
│   │   ├── ai/                      # AI client setup
│   │   │   ├── mod.rs
│   │   │   ├── rig_client.rs        # rig.rs wrapper (OpenAI)
│   │   │   └── embeddings.rs        # Embedding generation
│   │   ├── auth/                    # Authentication (admin only)
│   │   │   ├── mod.rs
│   │   │   └── clerk.rs             # Clerk JWT verification
│   │   ├── cache/                   # Caching layer (optional - Redis)
│   │   │   └── mod.rs
│   │   ├── sql/                     # Database utilities
│   │   │   ├── mod.rs
│   │   │   └── pool.rs              # Connection pool setup
│   │   ├── types/                   # Shared types
│   │   │   ├── mod.rs
│   │   │   ├── ids.rs               # Uuid wrappers
│   │   │   └── timestamps.rs        # DateTime utilities
│   │   └── utils/                   # Generic utilities
│   │       ├── mod.rs
│   │       └── content_hash.rs      # SHA256 content hashing
│   │
│   ├── domains/                     # Business domains
│   │   ├── mod.rs
│   │   │
│   │   ├── volunteer/               # Volunteer domain
│   │   │   ├── mod.rs
│   │   │   ├── commands/            # Command handlers
│   │   │   │   ├── mod.rs
│   │   │   │   ├── register.rs      # RegisterVolunteer handler
│   │   │   │   └── pause.rs         # PauseNotifications handler
│   │   │   ├── data/                # Domain data types
│   │   │   │   ├── mod.rs
│   │   │   │   └── types.rs         # VolunteerInput, etc.
│   │   │   ├── edges/               # GraphQL resolvers
│   │   │   │   ├── mod.rs
│   │   │   │   ├── query.rs         # Query resolvers
│   │   │   │   └── mutation.rs      # Mutation resolvers
│   │   │   ├── effects/             # Side effects (DB, external APIs)
│   │   │   │   ├── mod.rs
│   │   │   │   ├── db_effects.rs    # Database operations
│   │   │   │   └── push_effects.rs  # Expo push notifications
│   │   │   ├── events/              # Domain events
│   │   │   │   ├── mod.rs
│   │   │   │   └── types.rs         # VolunteerRegistered, etc.
│   │   │   ├── machines/            # State machines (pure logic)
│   │   │   │   ├── mod.rs
│   │   │   │   └── throttle.rs      # Notification throttling
│   │   │   ├── models/              # Database models
│   │   │   │   ├── mod.rs
│   │   │   │   └── volunteer.rs     # Volunteer struct
│   │   │   ├── errors.rs            # Domain-specific errors
│   │   │   └── registry.rs          # Domain registration (seesaw-rs)
│   │   │
│   │   ├── need/                    # Organization need domain
│   │   │   ├── mod.rs
│   │   │   ├── commands/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── extract.rs       # ExtractNeed (AI extraction)
│   │   │   │   ├── approve.rs       # ApproveNeed (admin approval)
│   │   │   │   └── expire.rs        # ExpireNeed (auto-expiry)
│   │   │   ├── data/
│   │   │   │   ├── mod.rs
│   │   │   │   └── types.rs         # NeedInput, ExtractedNeed
│   │   │   ├── edges/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── query.rs
│   │   │   │   └── mutation.rs
│   │   │   ├── effects/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── db_effects.rs
│   │   │   │   ├── scraper_effects.rs # Firecrawl API
│   │   │   │   └── ai_effects.rs      # AI extraction via rig.rs
│   │   │   ├── events/
│   │   │   │   ├── mod.rs
│   │   │   │   └── types.rs         # NeedExtracted, NeedApproved
│   │   │   ├── machines/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── extraction.rs    # AI extraction flow
│   │   │   │   └── deduplication.rs # Content hash matching
│   │   │   ├── models/
│   │   │   │   ├── mod.rs
│   │   │   │   └── need.rs          # OrganizationNeed struct
│   │   │   ├── prompts/             # AI prompts
│   │   │   │   ├── mod.rs
│   │   │   │   ├── extraction.rs    # Need extraction prompt
│   │   │   │   └── relevance.rs     # Relevance evaluation prompt
│   │   │   ├── errors.rs
│   │   │   └── registry.rs
│   │   │
│   │   ├── notification/            # Notification domain
│   │   │   ├── mod.rs
│   │   │   ├── commands/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── send.rs          # SendNotification
│   │   │   │   ├── preview.rs       # PreviewNotification (admin)
│   │   │   │   └── feedback.rs      # RecordFeedback (silent negative)
│   │   │   ├── data/
│   │   │   │   ├── mod.rs
│   │   │   │   └── types.rs         # NotificationInput, RelevanceEval
│   │   │   ├── edges/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── query.rs
│   │   │   │   └── mutation.rs
│   │   │   ├── effects/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── db_effects.rs
│   │   │   │   ├── expo_effects.rs  # Expo push API
│   │   │   │   └── matching_effects.rs # Vector search + AI eval
│   │   │   ├── events/
│   │   │   │   ├── mod.rs
│   │   │   │   └── types.rs         # NotificationSent, FeedbackRecorded
│   │   │   ├── machines/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── relevance.rs     # Relevance evaluation flow
│   │   │   │   └── throttle.rs      # Weekly notification limits
│   │   │   ├── models/
│   │   │   │   ├── mod.rs
│   │   │   │   └── notification.rs  # Notification struct
│   │   │   ├── errors.rs
│   │   │   └── registry.rs
│   │   │
│   │   ├── csv_import/              # CSV import domain (admin)
│   │   │   ├── mod.rs
│   │   │   ├── commands/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── parse.rs         # ParseCsvRow
│   │   │   │   └── import.rs        # ImportOrganization
│   │   │   ├── data/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── mapper.rs        # Column mapping logic
│   │   │   │   └── types.rs         # CsvImportInput
│   │   │   ├── edges/
│   │   │   │   ├── mod.rs
│   │   │   │   └── mutation.rs
│   │   │   ├── effects/
│   │   │   │   ├── mod.rs
│   │   │   │   └── db_effects.rs
│   │   │   ├── events/
│   │   │   │   ├── mod.rs
│   │   │   │   └── types.rs         # CsvUploaded, RowParsed
│   │   │   ├── machines/
│   │   │   │   ├── mod.rs
│   │   │   │   └── parser.rs        # CSV parsing flow
│   │   │   ├── models/
│   │   │   │   ├── mod.rs
│   │   │   │   └── csv_import.rs
│   │   │   ├── errors.rs
│   │   │   └── registry.rs
│   │   │
│   │   └── discovery/               # Automated discovery via Tavily
│   │       ├── mod.rs
│   │       ├── commands/
│   │       │   ├── mod.rs
│   │       │   └── search.rs        # SearchOpportunities
│   │       ├── data/
│   │       │   ├── mod.rs
│   │       │   └── types.rs         # SearchResult
│   │       ├── effects/
│   │       │   ├── mod.rs
│   │       │   ├── tavily_effects.rs # Tavily API client
│   │       │   └── db_effects.rs
│   │       ├── events/
│   │       │   ├── mod.rs
│   │       │   └── types.rs         # OpportunityDiscovered
│   │       ├── machines/
│   │       │   ├── mod.rs
│   │       │   └── discovery.rs     # Discovery workflow
│   │       ├── models/
│   │       │   └── mod.rs
│   │       ├── errors.rs
│   │       └── registry.rs
│   │
│   ├── kernel/                      # Core infrastructure (cross-domain)
│   │   ├── mod.rs
│   │   ├── jobs/                    # Background jobs
│   │   │   ├── mod.rs
│   │   │   ├── scheduler.rs         # Cron scheduler setup
│   │   │   ├── expire_needs.rs      # Auto-expiry job (daily)
│   │   │   ├── reset_counters.rs    # Weekly notification counter reset
│   │   │   └── discovery.rs         # Automated discovery (3/day max)
│   │   ├── search_engine/           # Vector search abstraction
│   │   │   ├── mod.rs
│   │   │   └── pgvector.rs          # pgvector implementation
│   │   └── verification/            # Need verification workflow
│   │       ├── mod.rs
│   │       └── admin_queue.rs       # Admin approval queue
│   │
│   └── server/                      # HTTP server setup
│       ├── mod.rs
│       ├── app.rs                   # Axum app builder
│       ├── graphql/                 # GraphQL schema
│       │   ├── mod.rs
│       │   ├── schema.rs            # Root schema (Query, Mutation)
│       │   ├── context.rs           # Request context (DB pool, auth)
│       │   └── scalars.rs           # Custom scalars (DateTime, JSON)
│       ├── routes/
│       │   ├── mod.rs
│       │   ├── graphql.rs           # /graphql endpoint
│       │   └── health.rs            # /health endpoint
│       ├── middleware/
│       │   ├── mod.rs
│       │   ├── auth.rs              # Clerk JWT middleware
│       │   ├── rate_limit.rs        # Rate limiting
│       │   └── logging.rs           # Request logging
│       └── static_files.rs          # Embedded admin SPA (rust-embed)
│
├── frontend/                        # Frontend applications (NOT Rust)
│   ├── expo-app/                    # Public volunteer app
│   │   ├── package.json
│   │   ├── app.json
│   │   ├── App.tsx
│   │   └── src/
│   │       ├── screens/
│   │       │   ├── RegisterScreen.tsx     # ⚠️ Only collects searchable_text + push token
│   │       │   ├── NotificationsScreen.tsx
│   │       │   └── NeedDetailScreen.tsx
│   │       ├── graphql/
│   │       │   ├── client.ts
│   │       │   ├── queries.ts
│   │       │   └── mutations.ts
│   │       └── components/
│   │           ├── WhyRelevantPanel.tsx   # "Why am I seeing this?" UI
│   │           └── FeedbackButton.tsx     # Silent negative feedback
│   │
│   └── admin-spa/                   # Admin panel (React + Vite)
│       ├── package.json
│       ├── vite.config.ts
│       ├── index.html
│       ├── dist/                    # Build output (embedded into Rust)
│       └── src/
│           ├── main.tsx
│           ├── App.tsx
│           ├── pages/
│           │   ├── Dashboard.tsx
│           │   ├── CsvImport.tsx
│           │   ├── NeedApproval.tsx      # Review AI-suggested needs
│           │   ├── NotificationPreview.tsx # Preview before sending
│           │   └── Settings.tsx          # Global kill switch
│           ├── graphql/
│           │   ├── client.ts
│           │   ├── queries.ts
│           │   └── mutations.ts
│           └── components/
│               ├── CsvMapper.tsx
│               └── NeedCard.tsx
│
├── tests/                           # Integration tests
│   ├── common/
│   │   └── setup.rs                 # Test database setup
│   ├── volunteer_test.rs
│   ├── need_extraction_test.rs
│   ├── notification_test.rs
│   └── discovery_test.rs
│
└── docs/                            # Documentation
    ├── PROBLEM_SOLUTION.md
    ├── RUST_IMPLEMENTATION.md
    ├── RUST_PROJECT_STRUCTURE.md    # ⚠️ DEPRECATED (replaced by this file)
    ├── PACKAGE_STRUCTURE.md         # ✅ THIS FILE (follows shay pattern)
    └── plans/
        └── 2026-01-27-feat-emergency-resource-aggregator-mvp-plan.md
```

---

## Cargo.toml (Workspace Root)

```toml
[package]
name = "api-core"
version = "0.1.0"
edition = "2021"

[lib]
name = "api_core"
path = "src/lib.rs"

[[bin]]
name = "api"
path = "src/server/main.rs"

[dependencies]
# Async runtime
tokio = { version = "1", features = ["full"] }
async-trait = "0.1"

# Error handling
anyhow = "1"
thiserror = "2"

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# IDs and timestamps
uuid = { version = "1", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# Event-driven architecture
seesaw = { path = "../shay/packages/seesaw-rs" }

# Database
sqlx = { version = "0.8", features = [
    "runtime-tokio",
    "tls-native-tls",
    "postgres",
    "uuid",
    "chrono",
    "json"
] }
pgvector = { version = "0.4", features = ["sqlx"] }

# AI / LLM
rig-core = "0.4"

# GraphQL
juniper = "0.16"
juniper_axum = "0.1"

# HTTP server
axum = { version = "0.7", features = ["multipart"] }
tower = "0.5"
tower-http = { version = "0.6", features = ["cors", "trace", "limit"] }

# HTTP client (for Firecrawl, Tavily, Expo)
reqwest = { version = "0.12", features = ["json"] }

# Authentication (admin only)
clerk-rs = "0.3"

# Cron jobs
tokio-cron-scheduler = "0.10"

# Static file embedding
rust-embed = "8.0"
mime_guess = "2.0"

# Environment
dotenvy = "0.15"

# Content hashing (duplicate detection)
sha2 = "0.10"

# Markdown rendering (optional - for display_markdown)
pulldown-cmark = "0.9"

[dev-dependencies]
# Testing
tokio-test = "0.4"
```

---

## Key Differences from Original Plan

### 1. Single Crate vs. 5 Crates

**Original Plan:**
```
crates/
├── api/
├── core/
├── db/
├── matching/
└── scraper/
```

**Updated (Following Shay):**
```
src/
├── common/        # Shared utilities
├── domains/       # Business domains
├── kernel/        # Infrastructure
└── server/        # HTTP server
```

**Why?**
- ✅ Follows shay's proven pattern
- ✅ Simpler for MVP (avoid over-engineering)
- ✅ Faster compile times (single crate)
- ✅ Easier to refactor (no crate boundaries)

### 2. Domain-Driven Structure

Each domain is self-contained with:
- `commands/` - Command handlers
- `data/` - Domain data types
- `edges/` - GraphQL resolvers
- `effects/` - Side effects (DB, external APIs)
- `events/` - Domain events
- `machines/` - State machines (pure logic)
- `models/` - Database models
- `prompts/` (if AI-heavy) - AI prompts
- `errors.rs` - Domain-specific errors
- `registry.rs` - seesaw-rs registration

### 3. Privacy-First Architecture

**Volunteer Domain:**
- ❌ NO `name`, `email`, `phone` fields
- ✅ ONLY `searchable_text` + `expo_push_token`
- ✅ Zero PII stored, zero data leak risk

**Need Domain:**
- ✅ `searchable_text` (plain text for AI embedding)
- ✅ `display_markdown` (optional rich text for display)
- ✅ `content_hash` (duplicate detection via SHA256)
- ✅ `expires_at` (auto-expiry)

**Notification Domain:**
- ✅ `why_relevant` field (transparency)
- ✅ Silent negative feedback tracking
- ✅ Atomic throttling (UPDATE...RETURNING)

---

## Domain Responsibilities

### `domains/volunteer/`

**Owns:**
- Volunteer registration (anonymous, push token only)
- Notification pause/snooze
- Embedding generation
- Weekly notification counter

**Entities:**
- `Volunteer` model (with `expo_push_token`)
- `VolunteerRegistered` event
- `PauseNotifications` command

### `domains/need/`

**Owns:**
- AI need extraction from websites
- Admin approval workflow
- Content hash deduplication
- Auto-expiry (urgent = 7 days, normal = 30 days)
- Markdown display support

**Entities:**
- `OrganizationNeed` model (with `display_markdown`)
- `NeedExtracted`, `NeedApproved`, `NeedExpired` events
- `ExtractNeed`, `ApproveNeed`, `ExpireNeed` commands

### `domains/notification/`

**Owns:**
- Vector search (top 20 candidates)
- AI relevance evaluation (generous)
- Notification throttling (3/week max)
- Expo push notification sending
- Silent negative feedback

**Entities:**
- `Notification` model (with `why_relevant`, `feedback` fields)
- `NotificationSent`, `FeedbackRecorded` events
- `SendNotification`, `PreviewNotification`, `RecordFeedback` commands

### `domains/csv_import/`

**Owns:**
- CSV upload and parsing
- Generic column mapper
- Organization import
- Admin-initiated workflow

**Entities:**
- `CsvImport` model
- `CsvUploaded`, `RowParsed` events
- `ParseCsvRow`, `ImportOrganization` commands

### `domains/discovery/`

**Owns:**
- Tavily API integration (3 searches/day max)
- Automated opportunity discovery
- Cron-triggered search jobs
- Minneapolis-focused queries

**Entities:**
- `DiscoveredOpportunity` (transient, not stored directly)
- `OpportunityDiscovered` event
- `SearchOpportunities` command

---

## Infrastructure Components

### `common/ai/`

**Provides:**
- rig.rs client setup (OpenAI)
- Embedding generation (text-embedding-3-small)
- Prompt injection protection
- Token usage tracking

### `kernel/jobs/`

**Provides:**
- Cron scheduler setup
- `expire_needs.rs` - Runs daily, expires stale needs
- `reset_counters.rs` - Runs weekly, resets notification counters
- `discovery.rs` - Runs 3x/day, searches Tavily for new opportunities

### `kernel/search_engine/`

**Provides:**
- Vector search abstraction
- pgvector implementation (HNSW indexes)
- Similarity scoring

### `server/graphql/`

**Provides:**
- Root schema (Query, Mutation)
- Request context (DB pool, Clerk auth)
- Custom scalars (DateTime, JSON, Upload)

---

## Frontend Integration

### Expo App (Public)

**Key Screens:**
1. **RegisterScreen** - Collects `searchable_text` + `expo_push_token` (ZERO PII)
2. **NotificationsScreen** - Shows why_relevant for each notification
3. **NeedDetailScreen** - Shows display_markdown + contact info

**GraphQL Mutations:**
```graphql
mutation RegisterVolunteer($input: RegisterVolunteerInput!) {
  registerVolunteer(input: $input) {
    id
    searchableText
    expoPushToken
    createdAt
  }
}

mutation PauseNotifications($days: Int!) {
  pauseNotifications(days: $days) {
    id
    pausedUntil
  }
}
```

### Admin SPA (Private)

**Key Pages:**
1. **Dashboard** - Overview metrics
2. **CsvImport** - Upload CSV, map columns, import orgs
3. **NeedApproval** - Review AI-suggested needs, edit before approval
4. **NotificationPreview** - See sample volunteers + message before sending
5. **Settings** - Global kill switch (discovery_enabled, notifications_enabled)

**GraphQL Mutations:**
```graphql
mutation ImportCsv($file: Upload!, $columnMapping: JSON!) {
  importCsv(file: $file, columnMapping: $columnMapping) {
    id
    filename
    rowCount
    status
  }
}

mutation ApproveNeed($needId: ID!, $searchableText: String, $displayMarkdown: String) {
  approveNeed(needId: $needId, searchableText: $searchableText, displayMarkdown: $displayMarkdown) {
    id
    status
    expiresAt
  }
}
```

---

## Database Schema Updates

### Volunteers Table (Privacy-First)

```sql
CREATE TABLE volunteers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- 🔒 PRIVACY-FIRST: Zero PII stored
    searchable_text TEXT NOT NULL,
    expo_push_token TEXT UNIQUE,  -- Format: ExponentPushToken[xxxxx]

    -- Embeddings
    embedding vector(1536),
    embedding_model_version TEXT DEFAULT 'text-embedding-3-small-2024-01',
    embedding_generated_at TIMESTAMPTZ,

    -- Operations
    active BOOLEAN DEFAULT true,
    notification_count_this_week INTEGER DEFAULT 0,
    last_notified_at TIMESTAMPTZ,
    paused_until TIMESTAMPTZ,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_volunteers_embedding ON volunteers
    USING hnsw (embedding vector_cosine_ops);
CREATE INDEX idx_volunteers_active ON volunteers(active) WHERE active = true;
CREATE INDEX idx_volunteers_expo_token ON volunteers(expo_push_token) WHERE expo_push_token IS NOT NULL;
```

### Organization Needs Table (With Markdown)

```sql
CREATE TABLE organization_needs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_name TEXT NOT NULL,

    -- Plain text for AI (REQUIRED)
    searchable_text TEXT NOT NULL,

    -- Optional rich text for display
    display_markdown TEXT,

    -- Contact + metadata
    contact_info TEXT,
    source_url TEXT,
    urgency TEXT,
    status TEXT DEFAULT 'active',
    expires_at TIMESTAMPTZ,

    -- Embeddings
    embedding vector(1536),
    embedding_model_version TEXT DEFAULT 'text-embedding-3-small-2024-01',
    embedding_generated_at TIMESTAMPTZ,

    -- Duplicate detection
    content_hash TEXT,

    -- Discovery tracking
    discovered_via TEXT DEFAULT 'csv',  -- csv | tavily | manual

    scraped_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_needs_embedding ON organization_needs
    USING hnsw (embedding vector_cosine_ops);
CREATE INDEX idx_needs_status ON organization_needs(status) WHERE status = 'active';
CREATE INDEX idx_needs_content_hash ON organization_needs(content_hash);
CREATE INDEX idx_needs_expires ON organization_needs(expires_at) WHERE status = 'active';
```

### Notifications Table (With Feedback)

```sql
CREATE TABLE notifications (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    need_id UUID REFERENCES organization_needs(id),
    volunteer_id UUID REFERENCES volunteers(id),

    -- Transparency + learning
    why_relevant TEXT NOT NULL,

    -- Tracking
    notified_at TIMESTAMPTZ DEFAULT NOW(),
    clicked BOOLEAN DEFAULT false,
    responded BOOLEAN DEFAULT false,

    -- Silent negative feedback
    feedback TEXT,  -- 'not_relevant' | 'already_helping' | 'not_available'
    feedback_at TIMESTAMPTZ
);

CREATE INDEX idx_notifications_volunteer ON notifications(volunteer_id);
CREATE INDEX idx_notifications_need ON notifications(need_id);
```

### System Settings Table (Global Kill Switch)

```sql
CREATE TABLE system_settings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    key TEXT UNIQUE NOT NULL,
    value JSONB NOT NULL,
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Seed initial settings
INSERT INTO system_settings (key, value) VALUES
    ('discovery_enabled', 'true'::jsonb),
    ('notifications_enabled', 'true'::jsonb),
    ('max_notifications_per_week', '3'::jsonb),
    ('max_tavily_searches_per_day', '3'::jsonb);
```

---

## Development Workflow

### 1. Create New Feature

**Example: Add "Volunteer Pause Notifications" feature**

1. **Define Event** in `src/domains/volunteer/events/types.rs`:
   ```rust
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct VolunteerPausedNotifications {
       pub volunteer_id: Uuid,
       pub paused_until: DateTime<Utc>,
   }
   ```

2. **Define Command** in `src/domains/volunteer/commands/pause.rs`:
   ```rust
   #[derive(Debug, Clone)]
   pub struct PauseNotifications {
       pub volunteer_id: Uuid,
       pub days: i32,
   }
   ```

3. **Implement Machine** in `src/domains/volunteer/machines/` (pure logic, no IO)

4. **Implement Effect** in `src/domains/volunteer/effects/db_effects.rs`:
   ```rust
   pub async fn pause_notifications(
       pool: &PgPool,
       cmd: PauseNotifications,
   ) -> Result<VolunteerPausedNotifications> {
       let paused_until = Utc::now() + Duration::days(cmd.days as i64);

       sqlx::query!(
           "UPDATE volunteers SET paused_until = $1 WHERE id = $2",
           paused_until,
           cmd.volunteer_id
       )
       .execute(pool)
       .await?;

       Ok(VolunteerPausedNotifications {
           volunteer_id: cmd.volunteer_id,
           paused_until,
       })
   }
   ```

5. **Add GraphQL Mutation** in `src/domains/volunteer/edges/mutation.rs`:
   ```rust
   pub async fn pause_notifications(
       ctx: &Context,
       days: i32,
   ) -> FieldResult<Volunteer> {
       let volunteer_id = ctx.current_volunteer_id()?;

       let cmd = PauseNotifications { volunteer_id, days };
       let event = pause_notifications_effect(&ctx.pool, cmd).await?;

       // Fetch updated volunteer
       let volunteer = fetch_volunteer(&ctx.pool, volunteer_id).await?;
       Ok(volunteer)
   }
   ```

### 2. Add Database Migration

```bash
sqlx migrate add add_paused_until_to_volunteers

# Edit migrations/[timestamp]_add_paused_until_to_volunteers.sql
# ALTER TABLE volunteers ADD COLUMN paused_until TIMESTAMPTZ;

sqlx migrate run
cargo sqlx prepare
```

### 3. Run Tests

```bash
# Unit tests (fast, no DB)
cargo test --lib

# Integration tests (with DB)
cargo test --test volunteer_test -- --test-threads=1

# All tests
cargo test
```

---

## Deployment

### Build Process

```bash
# 1. Build admin SPA
cd frontend/admin-spa
npm run build  # Creates dist/

# 2. Build Rust binary (embeds dist/)
cd ../..
cargo build --release

# Result: target/release/api (single binary with admin SPA embedded)
```

### Fly.io Deployment

```bash
# Initial setup
flyctl launch
flyctl postgres create --name mndigitalaid-db --region ord
flyctl postgres attach mndigitalaid-db

# Set secrets
flyctl secrets set \
    OPENAI_API_KEY=sk-... \
    TAVILY_API_KEY=tvly-... \
    CLERK_SECRET_KEY=sk_live_... \
    FIRECRAWL_API_KEY=fc-...

# Deploy
flyctl deploy
```

---

## Summary

This structure follows shay's proven domain-driven pattern while being simplified for MVP:

✅ **Single crate** (not 5 separate crates)
✅ **Domain-driven** (volunteer, need, notification, csv_import, discovery)
✅ **Privacy-first** (zero PII, expo_push_token only)
✅ **Event-driven** (seesaw-rs: events → machines → commands → effects)
✅ **Text-first storage** (searchable_text for AI, optional display_markdown for humans)
✅ **Embedded admin SPA** (single binary deployment)

**Next Steps:**
1. Initialize project structure: `mkdir -p src/domains/volunteer/{commands,data,edges,effects,events,machines,models}`
2. Create migrations: `sqlx migrate add create_volunteers`
3. Implement volunteer domain: Start with registration flow
4. Build GraphQL schema: Define types and resolvers
5. Test matching engine: Unit tests with mock embeddings
