# Emergency Resource Aggregator - Rust Project Structure

## Overview

This document shows the complete file/folder organization for the Rust backend following seesaw-rs event-driven architecture.

**Architecture Philosophy:**
- Events (facts) → Commands (intent) → Effects (IO handlers)
- Machines make pure decisions (no IO, no async)
- Effects are stateless (commands carry all data)
- One Command = One Effect = One Transaction

---

## Project Layout

```
mndigitalaid/
├── Cargo.toml                       # Workspace root
├── .env                             # Environment variables (DATABASE_URL, OPENAI_API_KEY)
├── .sqlx/                           # SQLx compile-time query cache
├── migrations/                      # SQLx database migrations
│   ├── 001_create_extensions.sql
│   ├── 002_create_volunteers.sql
│   ├── 003_create_needs.sql
│   ├── 004_create_notifications.sql
│   └── 005_create_indexes.sql
├── crates/                          # Workspace members
│   ├── api/                         # GraphQL API server (Juniper + Axum)
│   ├── core/                        # Domain logic, events, machines
│   ├── matching/                    # Relevance notifier engine
│   ├── scraper/                     # Website scraping + AI extraction
│   └── db/                          # Database models and queries (SQLx)
├── frontend/                        # Frontend applications (not Rust)
│   ├── expo-app/                    # Public volunteer app (Expo)
│   └── admin-spa/                   # Admin panel (React + Apollo)
└── docs/                            # Documentation
    ├── PROBLEM_SOLUTION.md
    ├── RUST_IMPLEMENTATION.md
    └── RUST_PROJECT_STRUCTURE.md (this file)
```

---

## Cargo Workspace Configuration

**File:** `Cargo.toml`

```toml
[workspace]
members = [
    "crates/api",
    "crates/core",
    "crates/matching",
    "crates/scraper",
    "crates/db",
]
resolver = "2"

[workspace.dependencies]
# Shared dependencies across all crates
tokio = { version = "1", features = ["full"] }
anyhow = "1"
thiserror = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
uuid = { version = "1", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
tracing = "0.1"
tracing-subscriber = "0.3"

# seesaw-rs (event-driven architecture)
seesaw = { path = "../shay/packages/seesaw-rs" }

# Database
sqlx = { version = "0.8", features = ["runtime-tokio", "tls-native-tls", "postgres", "uuid", "chrono", "json"] }
pgvector = { version = "0.4", features = ["sqlx"] }

# AI / LLM
rig-core = "0.4"

# GraphQL
juniper = "0.16"
juniper_axum = "0.1"

# HTTP server
axum = "0.7"
tower = "0.5"
tower-http = { version = "0.6", features = ["cors", "trace"] }

# Environment
dotenvy = "0.15"
```

---

## Crate: `api` (GraphQL Server)

**Purpose:** Exposes GraphQL API for mobile app and admin SPA.

**File:** `crates/api/Cargo.toml`

```toml
[package]
name = "api"
version = "0.1.0"
edition = "2021"

[dependencies]
# Workspace dependencies
tokio.workspace = true
anyhow.workspace = true
serde.workspace = true
serde_json.workspace = true
tracing.workspace = true
tracing-subscriber.workspace = true
dotenvy.workspace = true

# GraphQL
juniper.workspace = true
juniper_axum.workspace = true

# HTTP server
axum.workspace = true
tower.workspace = true
tower-http.workspace = true

# Internal crates
core = { path = "../core" }
db = { path = "../db" }
matching = { path = "../matching" }
scraper = { path = "../scraper" }

# Authentication (Clerk)
clerk-rs = "0.3"
```

**File Structure:**

```
crates/api/
├── Cargo.toml
└── src/
    ├── main.rs                      # Server entry point
    ├── lib.rs                       # Re-exports
    ├── graphql/
    │   ├── mod.rs                   # GraphQL root schema
    │   ├── context.rs               # Request context (DB pool, auth)
    │   ├── query.rs                 # Query root
    │   ├── mutation.rs              # Mutation root
    │   └── types/
    │       ├── mod.rs
    │       ├── volunteer.rs         # Volunteer GraphQL type
    │       ├── need.rs              # OrganizationNeed GraphQL type
    │       ├── notification.rs      # Notification GraphQL type
    │       └── csv_import.rs        # CSV import types
    ├── auth/
    │   ├── mod.rs
    │   └── clerk.rs                 # Clerk authentication middleware
    └── routes/
        ├── mod.rs
        └── graphql.rs               # GraphQL endpoint handler
```

**Example:** `crates/api/src/main.rs`

```rust
use anyhow::Result;
use axum::{routing::post, Router};
use dotenvy::dotenv;
use sqlx::PgPool;
use tower_http::cors::CorsLayer;
use tracing_subscriber;

mod auth;
mod graphql;
mod routes;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    tracing_subscriber::fmt::init();

    let database_url = std::env::var("DATABASE_URL")?;
    let pool = PgPool::connect(&database_url).await?;

    let schema = graphql::create_schema();
    let context = graphql::Context::new(pool);

    let app = Router::new()
        .route("/graphql", post(routes::graphql::graphql_handler))
        .layer(CorsLayer::permissive())
        .with_state((schema, context));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
    tracing::info!("GraphQL server running on http://localhost:8080/graphql");

    axum::serve(listener, app).await?;
    Ok(())
}
```

---

## Crate: `core` (Domain Logic)

**Purpose:** Pure domain logic with seesaw-rs events, commands, and machines.

**File:** `crates/core/Cargo.toml`

```toml
[package]
name = "core"
version = "0.1.0"
edition = "2021"

[dependencies]
seesaw.workspace = true
serde.workspace = true
serde_json.workspace = true
uuid.workspace = true
chrono.workspace = true
anyhow.workspace = true
thiserror.workspace = true
```

**File Structure:**

```
crates/core/
├── Cargo.toml
└── src/
    ├── lib.rs                       # Re-exports
    ├── events/
    │   ├── mod.rs                   # Event enum
    │   ├── volunteer.rs             # VolunteerRegistered, VolunteerUpdated
    │   ├── need.rs                  # NeedExtracted, NeedApproved, NeedExpired
    │   ├── notification.rs          # NotificationSent, NotificationClicked
    │   └── csv_import.rs            # CsvUploaded, RowParsed
    ├── commands/
    │   ├── mod.rs                   # Command enum
    │   ├── volunteer.rs             # RegisterVolunteer, UpdateVolunteer
    │   ├── need.rs                  # ExtractNeed, ApproveNeed
    │   ├── notification.rs          # SendNotification
    │   └── csv_import.rs            # ParseCsvRow, ImportOrganization
    └── machines/
        ├── mod.rs                   # Machine implementations
        ├── csv_import_machine.rs    # CSV import state machine
        ├── need_extraction_machine.rs # AI extraction state machine
        └── notification_machine.rs  # Notification throttling machine
```

**Example:** `crates/core/src/events/volunteer.rs`

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolunteerRegistered {
    pub volunteer_id: Uuid,
    pub name: String,
    pub email: String,
    pub phone: Option<String>,
    pub searchable_text: String,
    pub registered_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolunteerEmbeddingGenerated {
    pub volunteer_id: Uuid,
    pub embedding: Vec<f32>,
    pub generated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolunteerDeactivated {
    pub volunteer_id: Uuid,
    pub deactivated_at: DateTime<Utc>,
}
```

**Example:** `crates/core/src/commands/need.rs`

```rust
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractNeed {
    pub organization_name: String,
    pub source_url: String,
    pub scraped_content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApproveNeed {
    pub need_id: Uuid,
    pub approved_by: Uuid, // Admin user ID
    pub searchable_text: String, // Admin can edit before approval
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RejectNeed {
    pub need_id: Uuid,
    pub rejected_by: Uuid,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateNeedEmbedding {
    pub need_id: Uuid,
    pub searchable_text: String,
}
```

**Example:** `crates/core/src/machines/notification_machine.rs`

```rust
use crate::events::*;
use crate::commands::*;
use seesaw::Machine;

pub struct NotificationMachine {
    weekly_count: i32,
    max_per_week: i32,
}

impl NotificationMachine {
    pub fn new() -> Self {
        Self {
            weekly_count: 0,
            max_per_week: 3,
        }
    }
}

impl Machine for NotificationMachine {
    type Event = NotificationEvent;
    type Command = NotificationCommand;

    fn decide(&mut self, event: &Self::Event) -> Option<Self::Command> {
        match event {
            NotificationEvent::WeekStarted => {
                // Reset counter
                self.weekly_count = 0;
                None
            }
            NotificationEvent::RelevanceEvaluated { volunteer_id, need_id, is_relevant, why } => {
                // Check throttle
                if *is_relevant && self.weekly_count < self.max_per_week {
                    self.weekly_count += 1;
                    Some(NotificationCommand::SendNotification {
                        volunteer_id: *volunteer_id,
                        need_id: *need_id,
                        why_relevant: why.clone(),
                    })
                } else {
                    None
                }
            }
            _ => None
        }
    }
}
```

---

## Crate: `db` (Database Layer)

**Purpose:** SQLx models, queries, and database operations.

**File:** `crates/db/Cargo.toml`

```toml
[package]
name = "db"
version = "0.1.0"
edition = "2021"

[dependencies]
sqlx.workspace = true
pgvector.workspace = true
uuid.workspace = true
chrono.workspace = true
serde.workspace = true
serde_json.workspace = true
anyhow.workspace = true
thiserror.workspace = true

core = { path = "../core" }
```

**File Structure:**

```
crates/db/
├── Cargo.toml
└── src/
    ├── lib.rs                       # Re-exports
    ├── models/
    │   ├── mod.rs
    │   ├── volunteer.rs             # Volunteer model
    │   ├── need.rs                  # OrganizationNeed model
    │   ├── notification.rs          # Notification model
    │   └── csv_import.rs            # CsvImport model
    ├── queries/
    │   ├── mod.rs
    │   ├── volunteer.rs             # Volunteer CRUD queries
    │   ├── need.rs                  # Need CRUD queries
    │   ├── notification.rs          # Notification queries
    │   └── vector_search.rs         # Vector similarity queries
    └── effects/
        ├── mod.rs                   # Effect handlers (seesaw-rs effects)
        ├── volunteer_effects.rs     # Handle RegisterVolunteer command
        ├── need_effects.rs          # Handle ExtractNeed, ApproveNeed commands
        └── notification_effects.rs  # Handle SendNotification command
```

**Example:** `crates/db/src/models/volunteer.rs`

```rust
use chrono::{DateTime, Utc};
use pgvector::Vector;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Volunteer {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub phone: Option<String>,

    // Just searchable text - no rigid structure
    pub searchable_text: String,

    // Minimal metadata for operations
    pub embedding: Option<Vector>,
    pub active: bool,
    pub notification_count_this_week: i32,
    pub last_notified_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}
```

**Example:** `crates/db/src/queries/vector_search.rs`

```rust
use anyhow::Result;
use pgvector::Vector;
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::Volunteer;

pub async fn find_similar_volunteers(
    pool: &PgPool,
    need_embedding: &Vector,
    top_k: i64,
) -> Result<Vec<Volunteer>> {
    let volunteers = sqlx::query_as::<_, Volunteer>(
        r#"
        SELECT *
        FROM volunteers
        WHERE embedding IS NOT NULL
          AND active = true
        ORDER BY embedding <=> $1
        LIMIT $2
        "#,
    )
    .bind(need_embedding)
    .bind(top_k)
    .fetch_all(pool)
    .await?;

    Ok(volunteers)
}
```

**Example:** `crates/db/src/effects/volunteer_effects.rs`

```rust
use anyhow::Result;
use sqlx::PgPool;
use seesaw::Effect;

use core::commands::RegisterVolunteer;
use core::events::VolunteerRegistered;

pub struct RegisterVolunteerEffect {
    pool: PgPool,
}

impl RegisterVolunteerEffect {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl Effect for RegisterVolunteerEffect {
    type Command = RegisterVolunteer;
    type Event = VolunteerRegistered;

    async fn execute(&self, cmd: Self::Command) -> Result<Self::Event> {
        let volunteer_id = uuid::Uuid::new_v4();
        let registered_at = chrono::Utc::now();

        sqlx::query(
            r#"
            INSERT INTO volunteers (id, name, email, phone, searchable_text, created_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
        )
        .bind(volunteer_id)
        .bind(&cmd.name)
        .bind(&cmd.email)
        .bind(&cmd.phone)
        .bind(&cmd.searchable_text)
        .bind(registered_at)
        .execute(&self.pool)
        .await?;

        Ok(VolunteerRegistered {
            volunteer_id,
            name: cmd.name,
            email: cmd.email,
            phone: cmd.phone,
            searchable_text: cmd.searchable_text,
            registered_at,
        })
    }
}
```

---

## Crate: `matching` (Relevance Notifier)

**Purpose:** Core matching engine - vector search + AI relevance evaluation.

**File:** `crates/matching/Cargo.toml`

```toml
[package]
name = "matching"
version = "0.1.0"
edition = "2021"

[dependencies]
tokio.workspace = true
anyhow.workspace = true
serde.workspace = true
serde_json.workspace = true
uuid.workspace = true
chrono.workspace = true
tracing.workspace = true

# AI client
rig-core.workspace = true

# Database
sqlx.workspace = true
pgvector.workspace = true

# Internal crates
core = { path = "../core" }
db = { path = "../db" }
```

**File Structure:**

```
crates/matching/
├── Cargo.toml
└── src/
    ├── lib.rs                       # Re-exports
    ├── engine.rs                    # NotificationEngine (main logic)
    ├── relevance.rs                 # AI relevance evaluation
    ├── limits.rs                    # Notification throttling
    └── messages.rs                  # Notification message formatting
```

**Example:** `crates/matching/src/engine.rs`

```rust
use anyhow::Result;
use sqlx::PgPool;
use uuid::Uuid;

use crate::relevance::RigClient;
use db::models::{OrganizationNeed, Volunteer};

pub struct NotificationEngine {
    pool: PgPool,
    rig: RigClient,
}

impl NotificationEngine {
    pub fn new(pool: PgPool, rig: RigClient) -> Self {
        Self { pool, rig }
    }

    pub async fn process_need(&self, need_id: Uuid) -> Result<Vec<Uuid>> {
        // Fetch need
        let need = self.fetch_need(need_id).await?;

        // Vector search: get top 20 potentially relevant volunteers
        let candidates = self.find_candidates(&need, 20).await?;

        if candidates.is_empty() {
            tracing::info!("No candidates found for need {}", need_id);
            return Ok(vec![]);
        }

        // AI: Quick relevance check (generous)
        let evaluations = self.evaluate_relevance(&need, &candidates).await?;

        // Filter to relevant only
        let relevant: Vec<RelevanceEvaluation> = evaluations
            .into_iter()
            .filter(|e| e.is_relevant)
            .collect();

        // Simple notification logic: notify top 5, respect weekly limit
        let to_notify = self.apply_notification_limits(relevant, 5).await?;

        // Send notifications
        for eval in &to_notify {
            self.send_notification(need_id, eval).await?;
        }

        Ok(to_notify.iter().map(|e| e.volunteer_id).collect())
    }

    // ... (rest of implementation from RUST_IMPLEMENTATION.md)
}
```

---

## Crate: `scraper` (Website Scraping + AI Extraction + Discovery)

**Purpose:** Scrape organization websites, use AI to extract needs, and discover opportunities via Tavily.

**File:** `crates/scraper/Cargo.toml`

```toml
[package]
name = "scraper"
version = "0.1.0"
edition = "2021"

[dependencies]
tokio.workspace = true
anyhow.workspace = true
serde.workspace = true
serde_json.workspace = true
tracing.workspace = true

# AI client
rig-core.workspace = true

# HTTP client
reqwest = { version = "0.12", features = ["json"] }

# Cron scheduling
tokio-cron-scheduler = "0.10"

# Internal crates
core = { path = "../core" }
db = { path = "../db" }
```

**File Structure:**

```
crates/scraper/
├── Cargo.toml
└── src/
    ├── lib.rs                       # Re-exports
    ├── firecrawl.rs                 # Firecrawl API client
    ├── tavily.rs                    # Tavily API client (discovery)
    ├── discovery.rs                 # Automated discovery engine
    ├── extractor.rs                 # AI need extraction
    ├── embeddings.rs                # Embedding generation
    └── jobs/
        └── discovery_job.rs         # Cron job for automated discovery
```

**Example:** `crates/scraper/src/tavily.rs`

```rust
use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
struct TavilyRequest {
    api_key: String,
    query: String,
    search_depth: String,
    include_domains: Vec<String>,
    max_results: u32,
}

#[derive(Debug, Deserialize)]
pub struct TavilyResponse {
    pub results: Vec<TavilyResult>,
}

#[derive(Debug, Deserialize)]
pub struct TavilyResult {
    pub title: String,
    pub url: String,
    pub content: String,
    pub score: f32,
}

pub struct TavilyClient {
    api_key: String,
    client: Client,
}

impl TavilyClient {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            client: Client::new(),
        }
    }

    pub async fn search(&self, query: &str) -> Result<Vec<TavilyResult>> {
        let request = TavilyRequest {
            api_key: self.api_key.clone(),
            query: query.to_string(),
            search_depth: "advanced".to_string(),
            include_domains: vec![
                "facebook.com".to_string(),
                "nextdoor.com".to_string(),
                "minneapolis.org".to_string(),
            ],
            max_results: 10,
        };

        let response = self
            .client
            .post("https://api.tavily.com/search")
            .json(&request)
            .send()
            .await?;

        let data: TavilyResponse = response.json().await?;
        Ok(data.results)
    }
}
```

**Example:** `crates/scraper/src/extractor.rs`

```rust
use anyhow::Result;
use rig_core::{completion::Prompt, providers::openai};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ExtractedNeed {
    pub title: String,
    pub description: String,
    pub urgency: Option<String>,
}

pub struct NeedExtractor {
    client: openai::Client,
}

impl NeedExtractor {
    pub fn new(api_key: String) -> Self {
        let client = openai::Client::new(&api_key);
        Self { client }
    }

    pub async fn extract_needs(
        &self,
        scraped_content: &str,
        org_name: &str,
    ) -> Result<Vec<ExtractedNeed>> {
        let prompt = format!(
            r#"You are extracting volunteer needs from an organization's website.

Organization: {}

Website content:
{}

Extract SPECIFIC, ACTIONABLE volunteer needs. Look for:
- Direct requests for volunteers
- Skills or roles mentioned
- Services that need staffing
- Events needing helpers

Return ONLY valid JSON array:
[
  {{
    "title": "Brief title of need",
    "description": "Specific description of what volunteers would do",
    "urgency": "urgent|normal|low or null"
  }}
]

Only include needs with clear volunteer actions. If no needs found, return []."#,
            org_name, scraped_content
        );

        let response = self
            .client
            .agent("gpt-4o")
            .preamble("You extract volunteer needs from organization websites.")
            .temperature(0.3)
            .build()
            .prompt(&prompt)
            .await?;

        let json_str = response
            .trim()
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();

        let needs: Vec<ExtractedNeed> = serde_json::from_str(json_str)?;
        Ok(needs)
    }

    pub async fn generate_embedding(&self, text: &str) -> Result<Vec<f32>> {
        let embedding = self
            .client
            .embeddings("text-embedding-3-small")
            .embed_query(text)
            .await?;

        Ok(embedding)
    }
}
```

---

## Database Migrations

**Location:** `migrations/`

All SQLx migrations for database schema.

```
migrations/
├── 001_create_extensions.sql        # Enable pgvector, uuid-ossp
├── 002_create_volunteers.sql        # volunteers table
├── 003_create_needs.sql             # organization_needs table
├── 004_create_notifications.sql     # notifications table
└── 005_create_indexes.sql           # Vector + composite indexes
```

**Example:** `migrations/001_create_extensions.sql`

```sql
-- Enable required PostgreSQL extensions
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS vector;
```

**Example:** `migrations/002_create_volunteers.sql`

```sql
-- Volunteers: just text profiles
CREATE TABLE volunteers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL,
    email TEXT NOT NULL UNIQUE,
    phone TEXT,
    searchable_text TEXT NOT NULL,

    embedding vector(1536),
    active BOOLEAN DEFAULT true,
    notification_count_this_week INTEGER DEFAULT 0,
    last_notified_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ DEFAULT NOW()
);
```

**Example:** `migrations/005_create_indexes.sql`

```sql
-- Vector similarity indexes (HNSW for performance)
CREATE INDEX idx_volunteers_embedding ON volunteers
    USING ivfflat (embedding vector_cosine_ops);

CREATE INDEX idx_needs_embedding ON organization_needs
    USING ivfflat (embedding vector_cosine_ops);

-- Filter indexes
CREATE INDEX idx_volunteers_active ON volunteers(active) WHERE active = true;
CREATE INDEX idx_needs_status ON organization_needs(status) WHERE status = 'active';
```

---

## Frontend Applications

**Location:** `frontend/`

Frontend apps are NOT Rust - separate codebases.

```
frontend/
├── expo-app/                        # Public volunteer app (deployed separately)
│   ├── package.json
│   ├── app.json
│   ├── App.tsx
│   ├── src/
│   │   ├── screens/
│   │   │   ├── RegisterScreen.tsx
│   │   │   ├── NotificationsScreen.tsx
│   │   │   └── NeedDetailScreen.tsx
│   │   ├── graphql/
│   │   │   ├── client.ts            # Apollo Client setup
│   │   │   ├── queries.ts
│   │   │   └── mutations.ts
│   │   └── components/
│   └── expo-push-notifications.ts   # Expo push token registration
└── admin-spa/                       # Admin panel (EMBEDDED into Rust binary)
    ├── package.json
    ├── vite.config.ts
    ├── tailwind.config.js
    ├── index.html
    ├── dist/                        # Build output (embedded via rust-embed)
    │   ├── index.html
    │   ├── assets/
    │   │   ├── index-[hash].js
    │   │   └── index-[hash].css
    │   └── ...
    └── src/
        ├── main.tsx
        ├── App.tsx
        ├── pages/
        │   ├── Dashboard.tsx
        │   ├── CsvImport.tsx
        │   ├── NeedApproval.tsx
        │   └── Volunteers.tsx
        ├── graphql/
        │   ├── client.ts            # Apollo Client setup
        │   ├── queries.ts
        │   └── mutations.ts
        └── components/
            ├── CsvMapper.tsx
            └── NeedCard.tsx
```

**Deployment Strategy:**

1. **Expo App**: Deployed separately via Expo EAS
   - Mobile: iOS + Android native apps
   - Web: PWA hosted on Expo servers or Vercel

2. **Admin SPA**: Embedded into Rust binary via `rust-embed`
   - Built with `npm run build` → creates `dist/` folder
   - `rust-embed` includes `dist/` at compile time
   - Served from Rust server at `/` (fallback route)
   - **Result**: Single binary deployment to Fly.io

---

## Running the Project

### Prerequisites

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install sqlx-cli for migrations
cargo install sqlx-cli --no-default-features --features postgres

# Install Node.js for frontend apps
# (Install from nodejs.org or use nvm)
```

### Setup Database

```bash
# Create PostgreSQL database
createdb mndigitalaid

# Set environment variable
export DATABASE_URL="postgresql://localhost/mndigitalaid"

# Run migrations
cd /Users/crcn/Developer/fourthplaces/mndigitalaid
sqlx migrate run
```

### Run Backend API

```bash
# Build all workspace crates
cargo build

# Run API server
cargo run -p api

# Server starts at http://localhost:8080/graphql
```

### Run Frontend Apps

```bash
# Expo app
cd frontend/expo-app
npm install
npm start

# Admin SPA
cd frontend/admin-spa
npm install
npm run dev
```

---

## Development Workflow

### 1. Add a New Feature

**Event-driven flow:**

1. **Define Event** in `crates/core/src/events/`
2. **Define Command** in `crates/core/src/commands/`
3. **Implement Machine** in `crates/core/src/machines/` (pure logic)
4. **Implement Effect** in `crates/db/src/effects/` (database IO)
5. **Add GraphQL Mutation** in `crates/api/src/graphql/mutation.rs`

**Example: Adding "Volunteer Pause Notifications" feature**

```
1. Event: VolunteerPausedNotifications { volunteer_id, paused_until }
2. Command: PauseNotifications { volunteer_id, days }
3. Machine: Volunteer machine decides to emit event
4. Effect: Update database (active = false, resume_at = now + days)
5. GraphQL: mutation pauseNotifications(days: Int!): Volunteer
```

### 2. Add a Database Migration

```bash
# Create new migration file
sqlx migrate add create_new_table

# Edit migrations/[timestamp]_create_new_table.sql

# Run migration
sqlx migrate run

# Prepare SQLx metadata for compile-time checks
cargo sqlx prepare
```

### 3. Test the Matching Engine

```bash
# Unit test
cargo test -p matching

# Integration test with database
cargo test -p matching --features testing -- --test-threads=1
```

---

## Key Design Decisions

### 1. Cargo Workspace

**Why:** Separate concerns into focused crates. Each crate has single responsibility:
- `api`: HTTP/GraphQL interface
- `core`: Pure domain logic (no IO)
- `db`: Database operations (SQLx)
- `matching`: Relevance engine
- `scraper`: External data fetching

**Benefit:** Fast compile times (only rebuild changed crates), clear boundaries.

### 2. seesaw-rs Architecture

**Why:** Deterministic, testable, event-sourced system.

**Flow:**
```
Event → Machine.decide() → Command → Effect.execute() → Event
```

**Benefit:** Machines are pure functions (easy to test), effects handle all IO (easy to mock).

### 3. SQLx (Not ORM)

**Why:** Type-safe SQL with compile-time verification, no runtime query building overhead.

**Benefit:** Full SQL power, no N+1 problems, explicit queries.

### 4. Text-First Storage

**Why:** Anti-fragile - can re-embed with better models without migration.

**Benefit:** Evolvable system, zero data loss when AI improves.

### 5. GraphQL API

**Why:** Single endpoint for mobile + admin apps, clients request exactly what they need.

**Benefit:** Reduced over-fetching, strong typing, introspection.

---

## Security Considerations

### Authentication

- **Volunteers:** No authentication (anonymous, privacy-first)
- **Admins:** Clerk authentication (JWT tokens)

### Authorization Middleware

**File:** `crates/api/src/auth/clerk.rs`

```rust
use axum::{extract::Request, middleware::Next, response::Response};
use anyhow::Result;

pub async fn require_admin(req: Request, next: Next) -> Result<Response> {
    // Extract JWT from Authorization header
    // Verify with Clerk
    // Attach admin ID to request context
    // Call next middleware
}
```

### Rate Limiting

Implement at API layer for:
- Volunteer registration: 3 per IP per hour
- CSV imports: 1 per admin per 5 minutes
- Need extraction: 10 per admin per hour

---

## Performance Optimization

### 1. Vector Index (HNSW)

Use HNSW instead of IVFFlat for better recall:

```sql
-- Change in migration 005
CREATE INDEX idx_volunteers_embedding ON volunteers
    USING hnsw (embedding vector_cosine_ops);
```

### 2. Connection Pooling

SQLx automatically pools connections. Configure in `.env`:

```
DATABASE_URL=postgresql://localhost/mndigitalaid?pool_max_connections=20
```

### 3. Caching

Add Redis for:
- Embedding cache (avoid re-computing for same text)
- GraphQL query cache
- Rate limit tracking

---

## Deployment

### Fly.io Deployment

**Why Fly.io**: Simple Rust deployment, PostgreSQL included, global edge network, excellent for cron jobs.

**Key Innovation**: Automated discovery via Tavily - organizations never need to register, the system finds them automatically.

**File:** `fly.toml`

```toml
app = "mndigitalaid"
primary_region = "ord"  # Chicago (closest to Minnesota)

[build]
  builder = "paketobuildpacks/builder:base"
  buildpacks = ["gcr.io/paketo-buildpacks/rust"]

[env]
  PORT = "8080"

[[services]]
  http_checks = []
  internal_port = 8080
  processes = ["app"]
  protocol = "tcp"
  script_checks = []

  [services.concurrency]
    hard_limit = 25
    soft_limit = 20
    type = "connections"

  [[services.ports]]
    force_https = true
    handlers = ["http"]
    port = 80

  [[services.ports]]
    handlers = ["tls", "http"]
    port = 443

  [[services.tcp_checks]]
    grace_period = "1s"
    interval = "15s"
    restart_limit = 0
    timeout = "2s"

[mounts]
  source = "data"
  destination = "/data"
```

**Deploy Commands:**

```bash
# Install flyctl
curl -L https://fly.io/install.sh | sh

# Login
flyctl auth login

# Create app with Postgres
flyctl launch
flyctl postgres create --name mndigitalaid-db --region ord

# Attach database
flyctl postgres attach mndigitalaid-db

# Set secrets
flyctl secrets set OPENAI_API_KEY=sk-...
flyctl secrets set CLERK_SECRET_KEY=sk_live_...
flyctl secrets set FIRECRAWL_API_KEY=fc-...

# Deploy
flyctl deploy
```

### Embedding Admin SPA (Static Asset)

**Goal**: Bundle React admin SPA into Rust binary for single-deployment simplicity.

**File:** `crates/api/Cargo.toml`

```toml
[dependencies]
# ... other dependencies
rust-embed = "8.0"
```

**File:** `crates/api/src/static_files.rs`

```rust
use rust_embed::RustEmbed;
use axum::{
    body::Body,
    http::{header, StatusCode, Uri},
    response::{IntoResponse, Response},
};

#[derive(RustEmbed)]
#[folder = "../../frontend/admin-spa/dist"]
struct AdminAssets;

pub async fn serve_admin(uri: Uri) -> impl IntoResponse {
    let path = uri.path().trim_start_matches('/');

    // Try exact file first
    if let Some(content) = AdminAssets::get(path) {
        return serve_file(path, content);
    }

    // SPA fallback: serve index.html for all routes
    if let Some(content) = AdminAssets::get("index.html") {
        return serve_file("index.html", content);
    }

    (StatusCode::NOT_FOUND, "404 Not Found").into_response()
}

fn serve_file(path: &str, content: rust_embed::EmbeddedFile) -> Response {
    let mime_type = mime_guess::from_path(path)
        .first_or_octet_stream()
        .as_ref();

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, mime_type)
        .body(Body::from(content.data.to_vec()))
        .unwrap()
}
```

**File:** `crates/api/src/main.rs`

```rust
mod static_files;

use axum::{routing::get, Router};

#[tokio::main]
async fn main() -> Result<()> {
    // ... setup code

    let app = Router::new()
        .route("/graphql", post(routes::graphql::graphql_handler))
        // Serve admin SPA from embedded assets
        .fallback(static_files::serve_admin)
        .layer(CorsLayer::permissive());

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
    tracing::info!("Server running on http://localhost:8080");
    tracing::info!("GraphQL: http://localhost:8080/graphql");
    tracing::info!("Admin SPA: http://localhost:8080/ (embedded)");

    axum::serve(listener, app).await?;
    Ok(())
}
```

**Build Process:**

```bash
# 1. Build admin SPA
cd frontend/admin-spa
npm run build  # Creates dist/ folder

# 2. Build Rust API (embeds dist/ at compile time)
cd ../../
cargo build --release -p api

# Result: Single binary with admin SPA embedded
```

**Deployment Benefits:**
- ✅ Single binary deployment (no separate static hosting)
- ✅ Admin SPA served directly from Rust server
- ✅ No CORS issues (same origin)
- ✅ Simplified fly.toml (one service, not two)
- ✅ Embedded assets are cached at compile time

### Environment Variables

```bash
DATABASE_URL=postgresql://user:pass@host/mndigitalaid
OPENAI_API_KEY=sk-...
TAVILY_API_KEY=tvly-...              # Automated discovery
CLERK_SECRET_KEY=sk_live_...         # Admin auth only
FIRECRAWL_API_KEY=fc-...             # Website scraping
EXPO_PUSH_TOKEN=ExponentPushToken[...] # Push notifications
```

---

## Next Steps

1. **Initialize Cargo workspace:** `cargo init --lib crates/core` (repeat for each crate)
2. **Set up SQLx migrations:** `sqlx migrate add create_extensions`
3. **Implement core events/commands:** Start with volunteer registration flow
4. **Build GraphQL schema:** Define Volunteer, Need, Notification types
5. **Test matching engine:** Unit tests with mock embeddings

---

## Questions?

This structure follows:
- ✅ seesaw-rs event-driven patterns
- ✅ SQLx type-safe database queries
- ✅ Juniper GraphQL schema
- ✅ rig.rs AI client integration
- ✅ Separation of concerns (API, domain, data)
- ✅ Text-first storage for evolvability

Ready to start implementation? Begin with `crates/core` to define events and commands.
