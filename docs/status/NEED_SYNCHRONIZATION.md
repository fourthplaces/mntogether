# Need Synchronization - Automated Website Monitoring

## Overview

Automatically re-scrape organization websites to detect new, changed, or removed needs.

**Key Features:**
- ✅ **Periodic re-scraping** of stored organization URLs (daily)
- ✅ **Content hash comparison** to detect changes/removals
- ✅ **Automatic status updates** (new → active, unchanged → active, missing → expired)
- ✅ **Admin notifications** when significant changes detected
- ✅ **Smart scheduling** (prioritize high-activity orgs, back off on stale ones)

---

## Database Schema Updates

### Organization Sources Table (New)

Track websites we monitor for needs.

```sql
-- Organization sources: Websites we periodically scrape
CREATE TABLE organization_sources (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_name TEXT NOT NULL,
    source_url TEXT NOT NULL UNIQUE,

    -- Scraping metadata
    last_scraped_at TIMESTAMPTZ,
    last_successful_scrape_at TIMESTAMPTZ,
    scrape_frequency_hours INTEGER DEFAULT 24,  -- How often to check

    -- Activity tracking
    needs_found_last_scrape INTEGER DEFAULT 0,
    total_needs_found INTEGER DEFAULT 0,
    consecutive_failures INTEGER DEFAULT 0,

    -- Status
    active BOOLEAN DEFAULT true,
    disabled_reason TEXT,  -- If admin manually disabled

    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_org_sources_active ON organization_sources(active) WHERE active = true;
CREATE INDEX idx_org_sources_scrape_due ON organization_sources(last_scraped_at)
    WHERE active = true;
```

### Organization Needs Table (Update)

Add synchronization tracking to existing table.

```sql
-- Add to existing organization_needs table
ALTER TABLE organization_needs ADD COLUMN IF NOT EXISTS source_id UUID REFERENCES organization_sources(id);
ALTER TABLE organization_needs ADD COLUMN IF NOT EXISTS last_seen_at TIMESTAMPTZ DEFAULT NOW();
ALTER TABLE organization_needs ADD COLUMN IF NOT EXISTS disappeared_at TIMESTAMPTZ;

CREATE INDEX idx_needs_source ON organization_needs(source_id);
CREATE INDEX idx_needs_last_seen ON organization_needs(last_seen_at);
```

**Updated Schema:**
```sql
CREATE TABLE organization_needs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_name TEXT NOT NULL,

    -- Content (plain text for AI, optional markdown for display)
    searchable_text TEXT NOT NULL,
    display_markdown TEXT,

    -- Contact + metadata
    contact_info TEXT,
    source_url TEXT,
    urgency TEXT,
    status TEXT DEFAULT 'active',  -- active, expired, disappeared
    expires_at TIMESTAMPTZ,

    -- Embeddings
    embedding vector(1536),
    embedding_model_version TEXT DEFAULT 'text-embedding-3-small-2024-01',
    embedding_generated_at TIMESTAMPTZ,

    -- Duplicate detection (SHA256 hash of normalized text)
    content_hash TEXT,

    -- Discovery tracking
    discovered_via TEXT DEFAULT 'csv',  -- csv | tavily | manual | sync
    source_id UUID REFERENCES organization_sources(id),  -- NEW

    -- Synchronization tracking
    last_seen_at TIMESTAMPTZ DEFAULT NOW(),  -- NEW: Last time we saw this need
    disappeared_at TIMESTAMPTZ,  -- NEW: When we noticed it was gone

    scraped_at TIMESTAMPTZ DEFAULT NOW()
);
```

---

## Synchronization Logic

### Algorithm

```
FOR each active organization_source:
    IF last_scraped_at + scrape_frequency_hours < NOW():
        1. Scrape website (Firecrawl)
        2. Extract needs (rig.rs + GPT-4o)
        3. Generate content hashes for extracted needs
        4. Compare with existing needs from this source

        FOR each extracted need:
            IF content_hash exists in DB:
                - UPDATE last_seen_at = NOW()
                - Keep status = 'active'
            ELSE:
                - INSERT new need (status = 'pending_approval')
                - Admin gets notification

        FOR each existing need from this source:
            IF content_hash NOT in extracted needs:
                IF last_seen_at < NOW() - 7 days:
                    - UPDATE status = 'disappeared'
                    - UPDATE disappeared_at = NOW()
                    - Admin gets notification (optional)

        5. UPDATE organization_source metrics
```

### Content Hash Generation (Normalized)

To avoid false positives from minor text changes:

```rust
use sha2::{Sha256, Digest};

/// Generate normalized content hash for duplicate detection.
///
/// Normalizes text by:
/// - Lowercase
/// - Remove punctuation (except spaces)
/// - Collapse whitespace
/// - Sort words (optional - catches reordering)
pub fn generate_content_hash(text: &str) -> String {
    // Normalize: lowercase, remove punctuation, collapse whitespace
    let normalized = text
        .to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    // Hash
    let mut hasher = Sha256::new();
    hasher.update(normalized.as_bytes());
    format!("{:x}", hasher.finalize())
}
```

**Example:**
```
Input:  "We need Spanish-speaking legal volunteers! Contact: maria@example.org"
Input:  "We need spanish speaking legal volunteers contact mariaexampleorg"
Hash:   "a1b2c3..." (same for both - punctuation/case ignored)
```

---

## Package Structure

### New Domain: `sync`

```
src/domains/sync/
├── commands/
│   ├── mod.rs
│   ├── scrape_source.rs        # ScrapeOrganizationSource
│   ├── register_source.rs      # RegisterOrganizationSource (from CSV)
│   └── disable_source.rs       # DisableOrganizationSource
├── data/
│   └── types.rs                # OrganizationSourceInput
├── edges/
│   ├── query.rs                # Query resolvers
│   └── mutation.rs             # Mutation resolvers
├── effects/
│   ├── db_effects.rs           # Database operations
│   ├── scraper_effects.rs      # Firecrawl scraping
│   ├── ai_effects.rs           # Need extraction
│   └── comparison_effects.rs   # Hash comparison
├── events/
│   └── types.rs                # SourceScraped, NeedsUpdated
├── machines/
│   ├── mod.rs
│   └── sync_machine.rs         # Synchronization workflow
├── models/
│   ├── mod.rs
│   ├── organization_source.rs  # OrganizationSource struct
│   └── sync_result.rs          # SyncResult (new, updated, removed counts)
├── errors.rs
├── mod.rs
└── registry.rs
```

### Background Job

```
src/kernel/jobs/
├── sync_sources.rs             # NEW: Cron job for source syncing
```

---

## Rust Implementation

### Models

```rust
// src/domains/sync/models/organization_source.rs

use uuid::Uuid;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct OrganizationSource {
    pub id: Uuid,
    pub organization_name: String,
    pub source_url: String,

    // Scraping metadata
    pub last_scraped_at: Option<DateTime<Utc>>,
    pub last_successful_scrape_at: Option<DateTime<Utc>>,
    pub scrape_frequency_hours: i32,

    // Activity tracking
    pub needs_found_last_scrape: i32,
    pub total_needs_found: i32,
    pub consecutive_failures: i32,

    // Status
    pub active: bool,
    pub disabled_reason: Option<String>,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// src/domains/sync/models/sync_result.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResult {
    pub source_id: Uuid,
    pub organization_name: String,
    pub new_needs: Vec<Uuid>,
    pub updated_needs: Vec<Uuid>,
    pub disappeared_needs: Vec<Uuid>,
    pub total_scraped: usize,
    pub scrape_duration_ms: u64,
}
```

### Synchronization Effect

```rust
// src/domains/sync/effects/comparison_effects.rs

use crate::domains::sync::models::{OrganizationSource, SyncResult};
use crate::domains::need::models::OrganizationNeed;
use crate::common::utils::content_hash::generate_content_hash;
use anyhow::Result;
use sqlx::PgPool;
use uuid::Uuid;
use chrono::Utc;
use std::collections::HashSet;

pub async fn sync_organization_source(
    pool: &PgPool,
    source_id: Uuid,
    extracted_needs: Vec<ExtractedNeed>,
) -> Result<SyncResult> {
    let mut tx = pool.begin().await?;

    // Fetch source
    let source = sqlx::query_as!(
        OrganizationSource,
        "SELECT * FROM organization_sources WHERE id = $1",
        source_id
    )
    .fetch_one(&mut *tx)
    .await?;

    // Fetch existing needs from this source
    let existing_needs = sqlx::query_as!(
        OrganizationNeed,
        "SELECT * FROM organization_needs WHERE source_id = $1 AND status = 'active'",
        source_id
    )
    .fetch_all(&mut *tx)
    .await?;

    // Build hash sets for comparison
    let extracted_hashes: HashSet<String> = extracted_needs
        .iter()
        .map(|need| generate_content_hash(&need.description))
        .collect();

    let existing_hashes: std::collections::HashMap<String, Uuid> = existing_needs
        .iter()
        .map(|need| (need.content_hash.clone().unwrap_or_default(), need.id))
        .collect();

    let mut new_needs = Vec::new();
    let mut updated_needs = Vec::new();
    let mut disappeared_needs = Vec::new();

    // Process extracted needs
    for extracted in &extracted_needs {
        let hash = generate_content_hash(&extracted.description);

        if let Some(existing_id) = existing_hashes.get(&hash) {
            // Need still exists - update last_seen_at
            sqlx::query!(
                "UPDATE organization_needs SET last_seen_at = NOW() WHERE id = $1",
                existing_id
            )
            .execute(&mut *tx)
            .await?;

            updated_needs.push(*existing_id);
        } else {
            // New need - insert as pending approval
            let new_id = Uuid::new_v4();

            sqlx::query!(
                r#"
                INSERT INTO organization_needs (
                    id, organization_name, searchable_text, content_hash,
                    source_url, urgency, status, discovered_via, source_id,
                    last_seen_at
                )
                VALUES ($1, $2, $3, $4, $5, $6, 'pending_approval', 'sync', $7, NOW())
                "#,
                new_id,
                source.organization_name,
                extracted.description,
                hash,
                source.source_url,
                extracted.urgency,
                source_id
            )
            .execute(&mut *tx)
            .await?;

            new_needs.push(new_id);
        }
    }

    // Check for disappeared needs (not seen in latest scrape)
    for need in &existing_needs {
        let hash = need.content_hash.clone().unwrap_or_default();
        if !extracted_hashes.contains(&hash) {
            // Need disappeared - mark as disappeared if not seen for 7 days
            let last_seen = need.last_seen_at.unwrap_or(need.scraped_at);
            let days_missing = (Utc::now() - last_seen).num_days();

            if days_missing >= 7 {
                sqlx::query!(
                    r#"
                    UPDATE organization_needs
                    SET status = 'disappeared', disappeared_at = NOW()
                    WHERE id = $1
                    "#,
                    need.id
                )
                .execute(&mut *tx)
                .await?;

                disappeared_needs.push(need.id);
            }
        }
    }

    // Update source metrics
    sqlx::query!(
        r#"
        UPDATE organization_sources
        SET last_scraped_at = NOW(),
            last_successful_scrape_at = NOW(),
            needs_found_last_scrape = $1,
            total_needs_found = total_needs_found + $1,
            consecutive_failures = 0,
            updated_at = NOW()
        WHERE id = $2
        "#,
        extracted_needs.len() as i32,
        source_id
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(SyncResult {
        source_id,
        organization_name: source.organization_name,
        new_needs,
        updated_needs,
        disappeared_needs,
        total_scraped: extracted_needs.len(),
        scrape_duration_ms: 0, // Set by caller
    })
}

#[derive(Debug, Clone)]
pub struct ExtractedNeed {
    pub description: String,
    pub urgency: Option<String>,
}
```

---

## Background Job (Cron)

### Sync Job

```rust
// src/kernel/jobs/sync_sources.rs

use crate::domains::sync::effects::comparison_effects::{sync_organization_source, ExtractedNeed};
use crate::domains::need::effects::ai_effects::extract_needs_from_content;
use crate::common::ai::RigClient;
use anyhow::Result;
use sqlx::PgPool;
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::{info, warn};
use chrono::Utc;

pub async fn setup_sync_job(
    scheduler: &JobScheduler,
    pool: PgPool,
    rig: RigClient,
) -> Result<()> {
    // Run every hour, check for sources due for scraping
    let job = Job::new_async("0 0 * * * *", move |_uuid, _lock| {
        let pool = pool.clone();
        let rig = rig.clone();

        Box::pin(async move {
            if let Err(e) = sync_all_sources(&pool, &rig).await {
                warn!(err = %e, "Source sync job failed");
            }
        })
    })?;

    scheduler.add(job).await?;
    info!("Sync sources job scheduled (hourly)");

    Ok(())
}

async fn sync_all_sources(pool: &PgPool, rig: &RigClient) -> Result<()> {
    // Find sources due for scraping
    let sources = sqlx::query!(
        r#"
        SELECT id, organization_name, source_url, scrape_frequency_hours
        FROM organization_sources
        WHERE active = true
          AND (
            last_scraped_at IS NULL
            OR last_scraped_at + (scrape_frequency_hours || ' hours')::INTERVAL < NOW()
          )
        LIMIT 10
        "#
    )
    .fetch_all(pool)
    .await?;

    info!(count = sources.len(), "Found sources due for syncing");

    for source in sources {
        info!(
            source_id = %source.id,
            org_name = %source.organization_name,
            url = %source.source_url,
            "Syncing organization source"
        );

        match sync_single_source(pool, rig, source.id, &source.source_url).await {
            Ok(result) => {
                info!(
                    source_id = %source.id,
                    new = result.new_needs.len(),
                    updated = result.updated_needs.len(),
                    disappeared = result.disappeared_needs.len(),
                    "Source sync completed"
                );
            }
            Err(e) => {
                warn!(
                    source_id = %source.id,
                    err = %e,
                    "Source sync failed"
                );

                // Increment failure counter
                sqlx::query!(
                    r#"
                    UPDATE organization_sources
                    SET consecutive_failures = consecutive_failures + 1,
                        last_scraped_at = NOW()
                    WHERE id = $1
                    "#,
                    source.id
                )
                .execute(pool)
                .await?;
            }
        }
    }

    Ok(())
}

async fn sync_single_source(
    pool: &PgPool,
    rig: &RigClient,
    source_id: uuid::Uuid,
    url: &str,
) -> Result<crate::domains::sync::models::SyncResult> {
    use std::time::Instant;
    let start = Instant::now();

    // 1. Scrape website (Firecrawl)
    let content = crate::domains::need::effects::scraper_effects::scrape_url(url).await?;

    // 2. Extract needs (rig.rs + GPT-4o)
    let extracted = extract_needs_from_content(rig, &content, "").await?;

    // 3. Convert to ExtractedNeed
    let extracted_needs: Vec<ExtractedNeed> = extracted
        .into_iter()
        .map(|need| ExtractedNeed {
            description: need.description,
            urgency: need.urgency,
        })
        .collect();

    // 4. Sync with database
    let mut result = sync_organization_source(pool, source_id, extracted_needs).await?;
    result.scrape_duration_ms = start.elapsed().as_millis() as u64;

    Ok(result)
}
```

---

## GraphQL Schema

```graphql
# ═══════════════════════════════════════════════════════════════════════
#                      TYPES
# ═══════════════════════════════════════════════════════════════════════

type OrganizationSource {
  id: ID!
  organizationName: String!
  sourceUrl: String!

  lastScrapedAt: DateTime
  lastSuccessfulScrapeAt: DateTime
  scrapeFrequencyHours: Int!

  needsFoundLastScrape: Int!
  totalNeedsFound: Int!
  consecutiveFailures: Int!

  active: Boolean!
  disabledReason: String

  createdAt: DateTime!
  updatedAt: DateTime!
}

type SyncResult {
  sourceId: ID!
  organizationName: String!
  newNeedsCount: Int!
  updatedNeedsCount: Int!
  disappearedNeedsCount: Int!
  totalScraped: Int!
  scrapeDurationMs: Int!
}

# ═══════════════════════════════════════════════════════════════════════
#                      QUERIES
# ═══════════════════════════════════════════════════════════════════════

extend type Query {
  # Admin - list organization sources
  organizationSources(active: Boolean): [OrganizationSource!]!

  # Admin - view sync history for source
  syncHistory(sourceId: ID!, limit: Int = 20): [SyncResult!]!
}

# ═══════════════════════════════════════════════════════════════════════
#                      MUTATIONS
# ═══════════════════════════════════════════════════════════════════════

extend type Mutation {
  # Admin - register organization source (from CSV import)
  registerOrganizationSource(input: RegisterOrganizationSourceInput!): OrganizationSource!

  # Admin - manually trigger sync for a source
  syncOrganizationSource(sourceId: ID!): SyncResult!

  # Admin - disable source
  disableOrganizationSource(sourceId: ID!, reason: String!): OrganizationSource!

  # Admin - re-enable source
  enableOrganizationSource(sourceId: ID!): OrganizationSource!

  # Admin - adjust scrape frequency
  updateScrapeFrequency(sourceId: ID!, hours: Int!): OrganizationSource!
}

# ═══════════════════════════════════════════════════════════════════════
#                      INPUTS
# ═══════════════════════════════════════════════════════════════════════

input RegisterOrganizationSourceInput {
  organizationName: String!
  sourceUrl: String!
  scrapeFrequencyHours: Int = 24
}
```

---

## Smart Scheduling Strategy

### Adaptive Scrape Frequency

```rust
/// Adjust scrape frequency based on activity patterns
pub fn calculate_next_scrape_frequency(
    needs_found_last_scrape: i32,
    consecutive_failures: i32,
    current_frequency_hours: i32,
) -> i32 {
    match (needs_found_last_scrape, consecutive_failures) {
        // High activity - scrape more frequently
        (n, 0) if n > 5 => 6,     // Every 6 hours
        (n, 0) if n > 2 => 12,    // Every 12 hours
        (n, 0) if n > 0 => 24,    // Daily

        // Low activity - scrape less frequently
        (0, 0) => {
            // No new needs - gradually back off
            std::cmp::min(current_frequency_hours * 2, 168) // Max 1 week
        }

        // Failures - exponential backoff
        (_, f) if f > 0 => {
            std::cmp::min(current_frequency_hours * 2_i32.pow(f as u32), 168)
        }

        _ => current_frequency_hours,
    }
}
```

**Example progression:**
```
Day 1:  Scrape → Found 10 needs → Next: 6 hours
Day 1:  Scrape → Found 3 needs → Next: 12 hours
Day 2:  Scrape → Found 0 needs → Next: 24 hours
Day 3:  Scrape → Found 0 needs → Next: 48 hours
Day 5:  Scrape → Found 0 needs → Next: 96 hours
Day 9:  Scrape → Found 2 needs → Next: 12 hours (back to frequent)
```

---

## CSV Import Integration

### Updated CSV Import

When importing organizations from CSV, automatically register as sources:

```rust
// src/domains/csv_import/effects/import_effects.rs

pub async fn import_organization_from_csv(
    pool: &PgPool,
    row: &CsvRow,
) -> Result<OrganizationSource> {
    // Insert organization source
    let source = sqlx::query_as!(
        OrganizationSource,
        r#"
        INSERT INTO organization_sources (
            organization_name, source_url, scrape_frequency_hours, active
        )
        VALUES ($1, $2, 24, true)
        ON CONFLICT (source_url) DO UPDATE
        SET organization_name = EXCLUDED.organization_name,
            updated_at = NOW()
        RETURNING *
        "#,
        row.organization_name,
        row.website_url
    )
    .fetch_one(pool)
    .await?;

    Ok(source)
}
```

---

## Admin Notifications

### Notification Rules

Send admin notifications when:
- ✅ **New needs discovered** (high count in single scrape)
- ✅ **Source consistently failing** (3+ consecutive failures)
- ✅ **Needs disappeared** (significant count removed)

```rust
// src/domains/sync/effects/notification_effects.rs

pub async fn notify_admin_of_sync_result(
    sync_result: &SyncResult,
) -> Result<()> {
    // Notify if many new needs discovered
    if sync_result.new_needs.len() >= 5 {
        send_admin_notification(&format!(
            "🔍 {} new needs discovered from {}",
            sync_result.new_needs.len(),
            sync_result.organization_name
        )).await?;
    }

    // Notify if many needs disappeared
    if sync_result.disappeared_needs.len() >= 3 {
        send_admin_notification(&format!(
            "⚠️ {} needs disappeared from {}",
            sync_result.disappeared_needs.len(),
            sync_result.organization_name
        )).await?;
    }

    Ok(())
}
```

---

## Summary

This synchronization system provides:

✅ **Automated monitoring** - Periodic re-scraping of organization websites
✅ **Content hash comparison** - Detects new, updated, removed needs
✅ **Smart scheduling** - Adapts scrape frequency based on activity
✅ **Failure handling** - Exponential backoff for failing sources
✅ **Admin notifications** - Alerts for significant changes
✅ **CSV integration** - Auto-registers sources from imports
✅ **Need lifecycle** - active → disappeared (7 days unseen)

**Cost Impact:**
- Additional scraping: ~$10-20/month (Firecrawl API)
- Storage: Minimal (organization_sources table is small)
- Compute: Negligible (hourly cron job)

**Next Steps:**
1. Add `organization_sources` migration
2. Update `organization_needs` schema with sync fields
3. Implement `sync` domain with comparison logic
4. Add sync cron job to `kernel/jobs/`
5. Integrate source registration into CSV import
6. Build admin panel for source management
