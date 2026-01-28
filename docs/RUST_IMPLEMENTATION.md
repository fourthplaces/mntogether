# Emergency Resource Aggregator - Rust Implementation (Relevance Notifier)

## Core Philosophy

**We are building a relevance notifier, not a matcher.**

### What Success Looks Like:
- ✅ Volunteers hear about opportunities they might help with
- ✅ Organizations get connected with potential helpers
- ✅ Discovery shifts from pull → push (people don't have to search)

### What We're NOT Optimizing For:
- ❌ Perfect matching accuracy
- ❌ Preventing all mismatches
- ❌ Guarantee of qualification

### Key Insight:
**Cost of false positive:** 2 seconds of attention, one ignored notification
**Cost of false negative:** Someone never hears about a need they could have helped with

→ **Bias toward recall. Let humans self-select.**

---

## Simplified Architecture

```
Scraped Need
    ↓
AI: Generate searchable description
    ↓
Create embedding (OpenAI)
    ↓
Vector Search (top 20 volunteers)
    ↓
AI: Quick relevance judgment (generous)
    ↓
Apply simple limits (3/week max)
    ↓
Notification (invitational tone)
    ↓
Human decides: "Do I reach out?"
```

---

## Simplified Data Models

```rust
// src/db/models.rs

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
    pub notification_count_this_week: i32, // Simple throttling
    pub last_notified_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct OrganizationNeed {
    pub id: Uuid,
    pub organization_name: String,

    // Just searchable text
    pub searchable_text: String,

    // Minimal metadata
    pub source_url: Option<String>,
    pub urgency: Option<String>, // Just for notification phrasing, not filtering
    pub status: String, // active, filled, expired
    pub embedding: Option<Vector>,
    pub scraped_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelevanceEvaluation {
    pub volunteer_id: Uuid,
    pub is_relevant: bool, // Simple: yes or no
    pub why: String, // Brief explanation for the notification
}
```

---

## Database Schema

```sql
-- migrations/001_simple_schema.sql

CREATE EXTENSION IF NOT EXISTS vector;

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

-- Needs: just text descriptions
CREATE TABLE organization_needs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_name TEXT NOT NULL,
    searchable_text TEXT NOT NULL,

    source_url TEXT,
    urgency TEXT, -- just for notification tone
    status TEXT DEFAULT 'active',

    embedding vector(1536),
    scraped_at TIMESTAMPTZ DEFAULT NOW()
);

-- Track who was notified (for learning, not enforcement)
CREATE TABLE notifications (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    need_id UUID REFERENCES organization_needs(id),
    volunteer_id UUID REFERENCES volunteers(id),

    why_relevant TEXT, -- What we told them
    notified_at TIMESTAMPTZ DEFAULT NOW(),

    -- Did they engage? (optional - can add later)
    clicked BOOLEAN DEFAULT false,
    responded BOOLEAN DEFAULT false
);

-- Indexes
CREATE INDEX idx_volunteers_embedding ON volunteers
    USING ivfflat (embedding vector_cosine_ops);
CREATE INDEX idx_needs_embedding ON organization_needs
    USING ivfflat (embedding vector_cosine_ops);
CREATE INDEX idx_volunteers_active ON volunteers(active) WHERE active = true;
```

---

## Core Matching Logic

```rust
// src/matching/mod.rs

use anyhow::Result;
use sqlx::PgPool;
use uuid::Uuid;

use crate::ai::rig_client::RigClient;
use crate::db::models::{OrganizationNeed, RelevanceEvaluation, Volunteer};

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

    async fn find_candidates(
        &self,
        need: &OrganizationNeed,
        top_k: i64,
    ) -> Result<Vec<Volunteer>> {
        let need_embedding = need.embedding.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Need has no embedding"))?;

        let candidates = sqlx::query_as::<_, Volunteer>(
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
        .fetch_all(&self.pool)
        .await?;

        Ok(candidates)
    }

    async fn evaluate_relevance(
        &self,
        need: &OrganizationNeed,
        candidates: &[Volunteer],
    ) -> Result<Vec<RelevanceEvaluation>> {
        let candidates_text: String = candidates
            .iter()
            .enumerate()
            .map(|(i, v)| format!("{}. {}\n", i + 1, v.searchable_text))
            .collect();

        let prompt = format!(
            r#"A volunteer opportunity has come up:

{}

For each person below, decide if this opportunity is RELEVANT to them.

Be generous - if there's a reasonable chance they'd want to know about this, mark it relevant.

Consider:
- Do their skills/interests align?
- Would they likely be able to help?
- Is this worth their attention?

Don't worry about perfect matching - they'll decide if they actually want to help.

People:
{}

Return ONLY valid JSON:
[
  {{
    "candidate_number": 1,
    "is_relevant": true,
    "why": "Brief reason they might be interested"
  }}
]

Only include people where is_relevant is true."#,
            need.searchable_text, candidates_text
        );

        let response = self.rig.complete(&prompt).await?;

        let json_str = response
            .trim()
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();

        #[derive(serde::Deserialize)]
        struct RawEval {
            candidate_number: usize,
            is_relevant: bool,
            why: String,
        }

        let raw: Vec<RawEval> = serde_json::from_str(json_str)?;

        let evaluations = raw
            .into_iter()
            .filter_map(|e| {
                if e.candidate_number == 0 || e.candidate_number > candidates.len() {
                    return None;
                }

                Some(RelevanceEvaluation {
                    volunteer_id: candidates[e.candidate_number - 1].id,
                    is_relevant: e.is_relevant,
                    why: e.why,
                })
            })
            .collect();

        Ok(evaluations)
    }

    async fn apply_notification_limits(
        &self,
        relevant: Vec<RelevanceEvaluation>,
        max_to_notify: usize,
    ) -> Result<Vec<RelevanceEvaluation>> {
        // Simple throttling: max 3 notifications per volunteer per week
        let mut filtered = Vec::new();

        for eval in relevant {
            let count: i32 = sqlx::query_scalar(
                "SELECT notification_count_this_week FROM volunteers WHERE id = $1"
            )
            .bind(eval.volunteer_id)
            .fetch_one(&self.pool)
            .await?;

            if count < 3 {
                filtered.push(eval);
                if filtered.len() >= max_to_notify {
                    break;
                }
            }
        }

        Ok(filtered)
    }

    async fn send_notification(
        &self,
        need_id: Uuid,
        eval: &RelevanceEvaluation,
    ) -> Result<()> {
        // Store notification record
        sqlx::query(
            r#"
            INSERT INTO notifications (need_id, volunteer_id, why_relevant)
            VALUES ($1, $2, $3)
            "#,
        )
        .bind(need_id)
        .bind(eval.volunteer_id)
        .bind(&eval.why)
        .execute(&self.pool)
        .await?;

        // Increment notification count
        sqlx::query(
            r#"
            UPDATE volunteers
            SET notification_count_this_week = notification_count_this_week + 1,
                last_notified_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(eval.volunteer_id)
        .execute(&self.pool)
        .await?;

        // TODO: Actually send push notification via Expo
        tracing::info!(
            "Would notify volunteer {} about need {}: {}",
            eval.volunteer_id,
            need_id,
            eval.why
        );

        Ok(())
    }

    async fn fetch_need(&self, need_id: Uuid) -> Result<OrganizationNeed> {
        let need = sqlx::query_as::<_, OrganizationNeed>(
            "SELECT * FROM organization_needs WHERE id = $1",
        )
        .bind(need_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(need)
    }
}
```

---

## Notification Message (Critical: Tone Matters)

```rust
// src/notifications/messages.rs

pub fn format_notification_message(
    volunteer_name: &str,
    org_name: &str,
    need_description: &str,
    why_relevant: &str,
) -> (String, String) {
    let title = format!("Thought you might be interested");

    let body = format!(
        r#"{} is looking for help with:

{}

We thought of you because: {}

No pressure - just wanted to make sure you knew about it.

Tap to see contact info."#,
        org_name,
        need_description,
        why_relevant
    );

    (title, body)
}

pub fn format_notification_email(
    volunteer_name: &str,
    org_name: &str,
    need_description: &str,
    why_relevant: &str,
    contact_info: &str,
) -> String {
    format!(
        r#"Hi {},

There's a volunteer opportunity that might interest you:

{} is looking for help with:
{}

We thought of you because: {}

If this interests you, you can reach out to them directly at: {}

No pressure - we just wanted to make sure you knew about it.

---
If you'd prefer fewer notifications, you can adjust your preferences here: [link]
        "#,
        volunteer_name,
        org_name,
        need_description,
        why_relevant,
        contact_info
    )
}
```

**Key points:**
- ✅ Invites, doesn't pressure
- ✅ Explains why they were notified (transparency)
- ✅ Direct contact info (we don't mediate)
- ✅ Easy opt-down (respects attention)

---

## MVP Feature Set

### Week 1: Core Loop
```
✅ Scrape 5 sources (Firecrawl)
✅ Convert to searchable text (rig.rs + GPT-4o)
✅ Create embeddings (text-embedding-3-small)
✅ Store in Postgres with pgvector
✅ Vector search (top 20)
✅ Relevance check (rig.rs + GPT-4o)
✅ Send notifications (Expo push)
```

### Week 2: Observability
```
✅ Track who was notified
✅ Track who clicked/responded (optional)
✅ Simple dashboard showing:
   - Needs processed
   - Volunteers notified
   - Response rate (if tracking)
```

### Week 3: Refinement
```
✅ Adjust notification limits based on feedback
✅ Improve relevance prompts
✅ Add more sources
```

---

## What We're NOT Building (Yet)

- ❌ Complex matching rules
- ❌ Multi-stage filtering
- ❌ Confidence scoring
- ❌ Volunteer profiles with 20 fields
- ❌ Organization accounts
- ❌ In-app messaging
- ❌ Match tracking/analytics beyond clicks
- ❌ A/B testing infrastructure

**Why not?** Because we don't know what matters yet. Let usage teach us.

---

## Evolution Path

```
MVP (Month 1):
→ Text profiles + embeddings + simple relevance

Learn (Month 2-3):
→ Which notifications get responses?
→ What do people ignore?
→ Where are the gaps?

Iterate (Month 4+):
→ Add structure where it helps
→ Remove noise sources
→ Tune notification frequency
→ Consider reranking layer
```

---

## Final Architecture Diagram

```
┌─────────────────────────────────────┐
│  Scraped Content (Web/Social)        │
└──────────────┬──────────────────────┘
               ↓
┌──────────────────────────────────────┐
│  rig.rs: "Make this searchable"     │
└──────────────┬───────────────────────┘
               ↓
┌──────────────────────────────────────┐
│  Create Embedding (OpenAI)           │
└──────────────┬───────────────────────┘
               ↓
┌──────────────────────────────────────┐
│  Postgres + pgvector                 │
│  (Text + Embeddings)                 │
└──────────────┬───────────────────────┘
               ↓
┌──────────────────────────────────────┐
│  Vector Search (Top 20)              │
└──────────────┬───────────────────────┘
               ↓
┌──────────────────────────────────────┐
│  rig.rs: "Is this relevant?"         │
│  (Generous threshold)                │
└──────────────┬───────────────────────┘
               ↓
┌──────────────────────────────────────┐
│  Apply Simple Limits (3/week)        │
└──────────────┬───────────────────────┘
               ↓
┌──────────────────────────────────────┐
│  Push Notification (Expo)            │
│  "Thought you might be interested"   │
└──────────────┬───────────────────────┘
               ↓
┌──────────────────────────────────────┐
│  Human: "Do I reach out?"            │
│  (Real Decision Point)               │
└───────────────────────────────────────┘
```

**This is the right MVP.** Simple, evolvable, human-centered.
