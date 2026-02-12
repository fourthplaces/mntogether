---
title: "feat: AI Consultant Pipeline"
type: feat
date: 2026-02-12
---

# AI Consultant Pipeline

## Overview

Replace the current rigid post extraction pipeline (3-pass extraction → LLM sync → scoring → notes) with an **AI consultant** that runs per-org using a map-reduce architecture. The consultant sees all sources holistically, reasons about what actions to take, and proposes them through a unified review surface with comment-driven refinement.

**Core insight:** Instead of a pipeline where each step has a narrow lens (extract → dedupe → investigate → sync → score → notes), a single "consultant" LLM call receives a compiled document of everything known about an org and proposes all actions at once — create posts, update existing ones, add notes, flag contradictions, merge duplicates, archive stale content.

## Problem Statement

The current pipeline has three structural problems:

1. **Too rigid** — Cross-source reasoning (e.g., website says "accepting donations" but Instagram says "not accepting") requires seeing all sources at once, but the pipeline processes them in isolation or in narrow batches.
2. **Too many review touchpoints** — Admins review sync proposals, separately check notes, separately assess enrichment quality. No single surface for all AI recommendations.
3. **AI can't think broadly enough** — Each pass is scoped narrowly. The AI never gets the full picture of "here's everything about this org, what should we do?"

## Proposed Solution

```
Crawled Pages (150k+ tokens each)
    ↓ map (parallel, per-page)
Page Briefs (~1-2k each)
    ↓ compile (deterministic, prioritized)
Org Document (all briefs + existing posts + notes)
    ↓ reduce (single LLM call, the "consultant")
Proposed Actions (create, update, note, merge, archive, flag)
    ↓
Review Surface (approve / comment → AI revises / edit directly)
```

**Replaces:** 3-pass extraction pipeline (including agentic investigation/enrichment), LLM sync, relevance scoring, notes generation
**Keeps:** Crawling/page extraction (Firecrawl), embeddings, org/source management, dedup workflow

**Explicitly dropped:** External investigation (Pass 3 web search/fetch). Contact info, locations, and schedules should be available on the org's website and social media. If they're not, that's a signal about source quality — we don't compensate for incomplete sources.

## Technical Approach

### Architecture

The consultant pipeline is a Restate workflow with 4 durable phases:

```
┌─────────────────────────────────────────────────────────────────┐
│                     EXISTING (unchanged)                        │
├─────────────────────────────────────────────────────────────────┤
│  CrawlWebsiteWorkflow → extraction_pages                       │
│  CrawlSocialSourceWorkflow → extraction_pages                  │
│  IngestSourceWorkflow → extraction_pages                       │
│                                                                 │
│  Each stores raw content. No extraction, no sync.               │
└──────────────────────┬──────────────────────────────────────────┘
                       │ triggers
┌──────────────────────▼──────────────────────────────────────────┐
│                     NEW: ConsultOrgWorkflow                      │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Phase 1: Page Briefs (map)                                     │
│    - For each extraction_page, LLM extracts brief               │
│    - Parallel, per-page, GPT-5-mini                             │
│    - Output: Vec<PageBrief>                                     │
│                                                                 │
│  Phase 2: Org Document (compile)                                │
│    - Deterministic: briefs + existing posts + notes             │
│    - Priority: website first, then recent social                │
│    - Token budget: truncate oldest social if over limit         │
│    - Output: String (the compiled document)                     │
│                                                                 │
│  Phase 3: Consultant (reduce)                                   │
│    - Single LLM call with full org document                     │
│    - Structured output: Vec<ConsultantAction>                   │
│    - Actions: create_post, update_post, add_note,               │
│              merge_posts, archive_post, flag_contradiction       │
│    - Output: Vec<ConsultantAction>                              │
│                                                                 │
│  Phase 4: Stage Proposals                                       │
│    - Convert actions to sync_proposals (reuse existing system)  │
│    - Create draft entities where needed                         │
│    - Generate embeddings for new posts                          │
│    - Output: SyncBatch with proposals                           │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
                       │
┌──────────────────────▼──────────────────────────────────────────┐
│                     Review Surface                               │
├─────────────────────────────────────────────────────────────────┤
│  Admin reviews batch of proposals:                              │
│    - Approve → execute action (existing ProposalHandler)        │
│    - Reject → cleanup draft (existing ProposalHandler)          │
│    - Comment → triggers RefineProposalWorkflow                  │
│    - Edit directly → modify draft, then approve                 │
│                                                                 │
│  RefineProposalWorkflow:                                        │
│    - Takes: proposal + comment + original context               │
│    - LLM revises proposal content                               │
│    - Updates draft entity in-place                              │
│    - Admin sees revised version                                 │
│    - Max 3 revision rounds                                      │
└─────────────────────────────────────────────────────────────────┘
```

### Implementation Phases

#### Phase 1: Schema — Proposal Comments + Consultant Fields

New database tables and extensions to support the consultant pipeline.

**No `page_briefs` table needed** — Page briefs are cached via the existing `deps.memo()` infrastructure (backed by `memo_cache` table). The cache key is the page content itself, so briefs automatically invalidate when page content changes. A 30-day TTL handles garbage collection for pages that are no longer crawled.

**New migration: `NNNNNN_create_proposal_comments.sql`**

```sql
CREATE TABLE proposal_comments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    proposal_id UUID NOT NULL REFERENCES sync_proposals(id) ON DELETE CASCADE,
    author_id UUID NOT NULL REFERENCES members(id),
    content TEXT NOT NULL,
    -- AI revision triggered by this comment
    revision_number INTEGER NOT NULL DEFAULT 0,
    ai_revised BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_proposal_comments_proposal ON proposal_comments(proposal_id);
```

**Extend sync_proposals — new migration: `NNNNNN_extend_sync_proposals_for_consultant.sql`**

```sql
ALTER TABLE sync_proposals ADD COLUMN consultant_reasoning TEXT;
ALTER TABLE sync_proposals ADD COLUMN revision_count INTEGER NOT NULL DEFAULT 0;
ALTER TABLE sync_proposals ADD COLUMN confidence TEXT; -- 'high', 'medium', 'low'
ALTER TABLE sync_proposals ADD COLUMN source_urls TEXT[];
```

**Unified draft status — new migration: `NNNNNN_add_draft_status.sql`**

```sql
-- Notes: add status column with draft support
ALTER TABLE notes ADD COLUMN status TEXT NOT NULL DEFAULT 'active';
-- 'draft' for consultant-proposed notes awaiting review
-- 'active' for live notes (all existing notes default to active)

-- Posts: add 'draft' as a unified status for consultant-proposed posts
-- Existing 'pending_approval' stays valid for legacy pipeline during migration.
-- Consultant uses 'draft'. On approval, both 'draft' and 'pending_approval' → 'active'.
-- Eventually deprecate 'pending_approval' once old pipeline is removed.
```

**Unified pattern:** Both posts and notes use `status = 'draft'` for consultant-created entities. On approval → `active`. On rejection → delete. Same lifecycle, same handlers, same concept.

**New files:**

```
packages/server/src/domains/consultant/
├── mod.rs
├── models/
│   ├── mod.rs
│   └── proposal_comment.rs     → ProposalComment struct + CRUD
├── activities/
│   ├── mod.rs
│   ├── brief_extraction.rs     → extract_page_brief()
│   ├── org_document.rs         → compile_org_document()
│   ├── consultant.rs           → run_consultant()
│   ├── stage_actions.rs        → stage_consultant_actions()
│   └── refine_proposal.rs      → refine_proposal_from_comment()
└── restate/
    ├── mod.rs
    └── workflows/
        ├── mod.rs
        ├── consult_org.rs      → ConsultOrgWorkflow
        └── refine_proposal.rs  → RefineProposalWorkflow
```

**Acceptance criteria:**

- [ ] `proposal_comments` table created linking comments to proposals
- [ ] `sync_proposals` extended with `consultant_reasoning`, `revision_count`, `confidence`, `source_urls`
- [ ] `ProposalComment` model with `FromRow`, standard CRUD methods in `models/proposal_comment.rs`
- [ ] `SyncProposal` model updated with new fields
- [ ] New `consultant` domain registered in server module tree

---

#### Phase 2: Page Brief Extraction (Map Step)

Extract a compressed brief from each crawled page. This is the "map" step — runs in parallel per page. Uses `deps.memo()` for caching.

**`packages/server/src/domains/consultant/activities/brief_extraction.rs`**

```rust
use crate::kernel::{GPT_5_MINI, ServerDeps};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, StructuredOutput)]
pub struct PageBriefExtraction {
    pub summary: String,                        // 2-3 sentence overview
    pub locations: Vec<String>,                  // Full addresses found
    pub calls_to_action: Vec<String>,            // Urgent needs, donation requests, volunteer asks
    pub critical_info: Option<String>,           // Hours, eligibility, deadlines, closures
    pub services: Vec<String>,                   // Programs, services, opportunities
    pub contacts: Vec<BriefContact>,             // ALL contact methods found
    pub schedules: Vec<BriefSchedule>,           // Operating hours, event times, recurring patterns
    pub languages_mentioned: Vec<String>,        // Languages services are offered in
    pub populations_mentioned: Vec<String>,      // Target populations (refugees, seniors, etc.)
    pub capacity_info: Option<String>,           // "accepting", "waitlist", "at capacity", etc.
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, StructuredOutput)]
pub struct BriefContact {
    pub contact_type: String, // "phone", "email", "website", "booking_url", "intake_form", "address"
    pub value: String,
    pub label: Option<String>, // "Main office", "After-hours", "Intake", etc.
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, StructuredOutput)]
pub struct BriefSchedule {
    pub schedule_type: String,   // "operating_hours", "event", "recurring", "seasonal"
    pub description: String,     // "Monday-Friday 9am-5pm", "Every 2nd Tuesday at 6pm"
    pub days: Option<String>,    // "monday,wednesday,friday" or "weekdays"
    pub times: Option<String>,   // "09:00-17:00" or "6:00 PM"
    pub date: Option<String>,    // "2026-03-15" (for one-off events)
    pub frequency: Option<String>, // "weekly", "biweekly", "monthly"
    pub seasonal_notes: Option<String>, // "September through May", "Summer only"
    pub exceptions: Option<String>,     // "Closed holidays", "1st and 3rd week only"
}

const PAGE_BRIEF_PROMPT: &str = r#"
You are extracting critical information from a web page belonging to a community organization.

Extract ONLY factual information present on the page. Do not infer or fabricate.

## Fields to Extract

- **summary**: 2-3 sentence overview of what this page tells us about the organization.

- **locations**: All physical addresses mentioned. Include full addresses.

- **calls_to_action**: Urgent needs and requests — donation drives, volunteer asks,
  sign-up opportunities, supply needs. Be specific about what's needed and deadlines.

- **critical_info**: Operating hours, eligibility requirements, deadlines, closures,
  capacity limits, waitlist info. Anything someone needs to know before showing up.

- **services**: Programs, services, or opportunities offered by name.

- **contacts**: EVERY contact method found on the page. Be thorough:
  - Phone numbers (with labels: "main", "after-hours", "hotline")
  - Email addresses (with labels: "info", "intake", "referrals")
  - Website URLs for specific actions (intake forms, booking pages, sign-up links)
  - Physical addresses for in-person visits
  - Include the label/context for each (e.g., "Booking: https://...")

- **schedules**: ALL temporal information. Be precise about the pattern:
  - **Operating hours**: "Monday-Friday 9am-5pm" → schedule_type: "operating_hours"
  - **Recurring events**: "Every 2nd Tuesday at 6pm" → schedule_type: "recurring"
  - **One-off events**: "March 15, 2026 at 2pm" → schedule_type: "event"
  - **Seasonal patterns**: "September through May" → schedule_type: "seasonal"
  - Include days, times, dates, frequency, and any exceptions
    ("closed holidays", "1st and 3rd week only", "by appointment only")

- **languages_mentioned**: Languages services are offered in (e.g., "Spanish",
  "Somali", "Karen", "Hmong"). Look for "multilingual", "interpreter available", etc.

- **populations_mentioned**: Target populations served — "refugees", "immigrants",
  "asylum seekers", "seniors", "youth", "families", "unaccompanied minors", etc.

- **capacity_info**: Current capacity status if mentioned — "accepting new clients",
  "waitlist", "at capacity", "not accepting donations at this time", etc.

If a field has no relevant information on the page, return an empty list/null.
Be concise but thorough. Capture all specifics, especially for schedules and contacts.
"#;

/// Extract a brief from a single page, memoized by content.
/// If the same page content was briefed before, returns cached result.
pub async fn extract_page_brief(
    page_url: &str,
    page_content: &str,
    organization_name: &str,
    deps: &ServerDeps,
) -> Result<Option<PageBriefExtraction>> {
    if page_content.trim().len() < 100 {
        return Ok(None);
    }

    let content = &page_content[..page_content.len().min(50_000)];
    let user_prompt = format!(
        "Organization: {}\nPage URL: {}\n\n---\n\n{}",
        organization_name, page_url, content
    );

    // Memo key includes system prompt + user prompt (page content).
    // Same content + same prompt → cache hit.
    // Content changes OR prompt changes → cache miss → new LLM call.
    // 30-day TTL is just garbage collection for pages no longer crawled.
    let brief: PageBriefExtraction = deps
        .memo("page_brief_v1", (PAGE_BRIEF_PROMPT, &user_prompt))
        .ttl(2_592_000_000) // 30 days
        .get_or(|| async {
            deps.ai.extract::<PageBriefExtraction>(
                GPT_5_MINI,
                PAGE_BRIEF_PROMPT,
                &user_prompt,
            ).await
        })
        .await?;

    // Filter out empty briefs
    if brief.summary.trim().is_empty()
        && brief.calls_to_action.is_empty()
        && brief.services.is_empty()
    {
        return Ok(None);
    }

    Ok(Some(brief))
}

/// Extract briefs for all pages in parallel, with memo-based caching.
pub async fn extract_briefs_for_org(
    org_name: &str,
    pages: &[CachedPage],
    deps: &ServerDeps,
) -> Result<Vec<(String, PageBriefExtraction)>> {
    let futures = pages.iter().map(|page| {
        let org_name = org_name.to_string();
        let url = page.url.clone();
        async move {
            let brief = extract_page_brief(
                &url, &page.content, &org_name, deps
            ).await?;
            Ok::<_, anyhow::Error>(brief.map(|b| (url, b)))
        }
    });

    let results = futures::future::join_all(futures).await;
    Ok(results
        .into_iter()
        .filter_map(|r| r.ok().flatten())
        .collect())
}
```

**Key design decisions:**

- **`deps.memo()` caching** — The memo key includes both the system prompt and the user prompt (which contains page content). Same content + same prompt = cache hit. If either the page content changes OR the extraction prompt is updated, it's a cache miss and a fresh LLM call runs. No separate `page_briefs` table needed.
- **30-day TTL** — Since the key is content-based, the TTL is just garbage collection for pages no longer being crawled. A page that changes gets a new cache entry automatically.
- **50k char truncation** — Individual pages over 50k chars are truncated. The brief captures the most important info.
- **Parallel execution** — All pages briefed concurrently via `join_all()`.
- **PII consideration** — Page content should be passed through `pii_detector.scrub()` before briefing, same as the current pipeline.
- **No separate table** — Briefs are ephemeral intermediate data, not independently queryable entities.

**Acceptance criteria:**

- [ ] `extract_page_brief()` uses `deps.memo()` with content-based key for caching
- [ ] `extract_briefs_for_org()` runs in parallel across all pages
- [ ] Cache hit returns instantly for unchanged pages; cache miss triggers LLM call
- [ ] Pages under 100 chars or with empty briefs are filtered out
- [ ] PII scrubbing applied to page content before LLM call

---

#### Phase 3: Org Document Compilation

Compile all page briefs + existing posts + notes into a single document for the consultant.

**`packages/server/src/domains/consultant/activities/org_document.rs`**

```rust
/// Maximum token budget for the org document (chars, roughly 4 chars/token)
const MAX_ORG_DOCUMENT_CHARS: usize = 200_000; // ~50k tokens

pub struct OrgDocument {
    pub content: String,
    pub token_estimate: usize,
    pub briefs_included: usize,
    pub posts_included: usize,
    pub notes_included: usize,
}

/// Compile the org document from all available data.
/// Priority: website briefs → recent social briefs → existing posts → notes
pub async fn compile_org_document(
    org_id: Uuid,
    org_name: &str,
    briefs: &[(String, PageBriefExtraction)], // (page_url, brief)
    pool: &PgPool,
) -> Result<OrgDocument> {
    let mut doc = String::new();
    let mut budget = MAX_ORG_DOCUMENT_CHARS;

    // Header
    let header = format!("# Organization: {}\n\n", org_name);
    doc.push_str(&header);
    budget -= header.len();

    // Section 1: Website briefs (highest priority)
    let website_briefs: Vec<_> = briefs.iter()
        .filter(|b| b.source_type == "website")
        .collect();
    let social_briefs: Vec<_> = briefs.iter()
        .filter(|b| b.source_type != "website")
        .collect();

    doc.push_str("## Website Content\n\n");
    let mut briefs_count = 0;
    for brief in &website_briefs {
        let section = format_brief(brief);
        if section.len() > budget { break; }
        doc.push_str(&section);
        budget -= section.len();
        briefs_count += 1;
    }

    // Section 2: Social media briefs (recent first)
    doc.push_str("\n## Social Media\n\n");
    for brief in &social_briefs {
        let section = format_brief(brief);
        if section.len() > budget { break; }
        doc.push_str(&section);
        budget -= section.len();
        briefs_count += 1;
    }

    // Section 3: Existing posts in the system
    let existing_posts = Post::find_active_by_organization(org_id, pool).await?;
    doc.push_str("\n## Existing Posts in System\n\n");
    let mut posts_count = 0;
    for post in &existing_posts {
        let section = format_existing_post(post);
        if section.len() > budget { break; }
        doc.push_str(&section);
        budget -= section.len();
        posts_count += 1;
    }

    // Section 4: Existing notes
    let notes = Note::find_by_organization(org_id, pool).await?;
    doc.push_str("\n## Existing Notes\n\n");
    let mut notes_count = 0;
    for note in &notes {
        let section = format_note(note);
        if section.len() > budget { break; }
        doc.push_str(&section);
        budget -= section.len();
        notes_count += 1;
    }

    Ok(OrgDocument {
        token_estimate: doc.len() / 4,
        content: doc,
        briefs_included: briefs_count,
        posts_included: posts_count,
        notes_included: notes_count,
    })
}

fn format_brief(url: &str, brief: &PageBriefExtraction) -> String {
    let mut s = format!("### {}\n", url);
    s.push_str(&format!("{}\n", brief.summary));
    if !brief.locations.is_empty() {
        s.push_str(&format!("- Locations: {}\n", brief.locations.join(", ")));
    }
    if !brief.calls_to_action.is_empty() {
        s.push_str(&format!("- Calls to action: {}\n", brief.calls_to_action.join("; ")));
    }
    if let Some(info) = &brief.critical_info {
        s.push_str(&format!("- Critical: {}\n", info));
    }
    if !brief.services.is_empty() {
        s.push_str(&format!("- Services: {}\n", brief.services.join(", ")));
    }
    if !brief.contacts.is_empty() {
        let contact_strs: Vec<_> = brief.contacts.iter()
            .map(|c| {
                let label = c.label.as_deref().unwrap_or(&c.contact_type);
                format!("{}: {}", label, c.value)
            })
            .collect();
        s.push_str(&format!("- Contacts: {}\n", contact_strs.join(", ")));
    }
    if !brief.schedules.is_empty() {
        for sched in &brief.schedules {
            let mut line = format!("- Schedule ({}): {}", sched.schedule_type, sched.description);
            if let Some(exc) = &sched.exceptions {
                line.push_str(&format!(" [{}]", exc));
            }
            if let Some(seasonal) = &sched.seasonal_notes {
                line.push_str(&format!(" ({})", seasonal));
            }
            s.push_str(&format!("{}\n", line));
        }
    }
    if !brief.languages_mentioned.is_empty() {
        s.push_str(&format!("- Languages: {}\n", brief.languages_mentioned.join(", ")));
    }
    if !brief.populations_mentioned.is_empty() {
        s.push_str(&format!("- Populations: {}\n", brief.populations_mentioned.join(", ")));
    }
    if let Some(cap) = &brief.capacity_info {
        s.push_str(&format!("- Capacity: {}\n", cap));
    }
    s.push('\n');
    s
}

fn format_existing_post(post: &Post, contacts: &[Contact], schedules: &[Schedule], tags: &[Taggable]) -> String {
    let mut s = format!(
        "### [POST-{}] {} (status: {}, type: {})\n{}\n",
        post.id, post.title, post.status,
        post.post_type.as_deref().unwrap_or("unknown"),
        post.summary.as_deref().unwrap_or("No summary"),
    );
    if let Some(loc) = &post.location {
        s.push_str(&format!("- Location: {}\n", loc));
    }
    if let Some(urgency) = &post.urgency {
        s.push_str(&format!("- Urgency: {}\n", urgency));
    }
    if !contacts.is_empty() {
        let contact_strs: Vec<_> = contacts.iter()
            .map(|c| format!("{}: {}", c.contact_type, c.contact_value))
            .collect();
        s.push_str(&format!("- Contacts: {}\n", contact_strs.join(", ")));
    }
    if !schedules.is_empty() {
        let sched_strs: Vec<_> = schedules.iter()
            .map(|sc| format_schedule_brief(sc))
            .collect();
        s.push_str(&format!("- Schedule: {}\n", sched_strs.join("; ")));
    }
    if !tags.is_empty() {
        let tag_strs: Vec<_> = tags.iter()
            .map(|t| format!("{}:{}", t.kind, t.value))
            .collect();
        s.push_str(&format!("- Tags: {}\n", tag_strs.join(", ")));
    }
    s.push('\n');
    s
}

fn format_note(note: &Note) -> String {
    format!(
        "### [NOTE-{}] (severity: {})\n{}\n\n",
        note.id,
        note.severity.as_deref().unwrap_or("info"),
        note.content
    )
}
```

**Key design decisions:**

- **Token budget with priority truncation** — Website content fills first, social media fills next, then existing posts, then notes. If budget runs out, older/lower-priority content is dropped.
- **200k char budget** (~50k tokens) — Leaves room for the consultant system prompt and structured output. Fits comfortably in GPT-5-mini's context window.
- **Existing posts referenced by ID** — The consultant can reference `POST-{uuid}` in its actions, enabling precise updates.
- **Deterministic** — No LLM calls. Pure data assembly.

**Acceptance criteria:**

- [ ] `compile_org_document()` assembles briefs + posts + notes into a single string
- [ ] Website briefs prioritized over social media briefs
- [ ] Token budget enforced with graceful truncation
- [ ] Existing posts and notes include their IDs for reference
- [ ] Output includes metadata (briefs/posts/notes count, token estimate)

---

#### Phase 4: Consultant Reasoning (Reduce Step)

The core intelligence — a single LLM call that reads the org document and proposes actions.

**`packages/server/src/domains/consultant/activities/consultant.rs`**

```rust
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, StructuredOutput)]
pub struct ConsultantResponse {
    pub actions: Vec<ConsultantAction>,
    pub org_summary: String, // Brief assessment of the org's current state
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, StructuredOutput)]
pub struct ConsultantAction {
    pub action_type: String, // "create_post", "update_post", "add_note", "merge_posts", "archive_post", "flag_contradiction"
    pub reasoning: String,   // Why this action is recommended
    pub confidence: String,  // "high", "medium", "low"
    pub source_urls: Vec<String>, // Which source pages support this action

    // For create_post / update_post — narrative content
    pub title: Option<String>,
    pub summary: Option<String>,          // 2-3 sentences, ~250 chars
    pub description: Option<String>,       // Comprehensive markdown
    pub description_markdown: Option<String>, // Rich formatted version

    // For create_post / update_post — classification
    pub post_type: Option<String>,         // "service", "opportunity", "business", "professional"
    pub category: Option<String>,          // "food-assistance", "legal-aid", "housing", etc.
    pub urgency: Option<String>,           // "low", "medium", "high", "urgent"
    pub capacity_status: Option<String>,   // "accepting", "paused", "at_capacity"

    // For create_post / update_post — location
    pub location: Option<LocationData>,

    // For create_post / update_post — contacts
    pub contacts: Option<Vec<ContactData>>,

    // For create_post / update_post — schedule (supports 3 modes)
    pub schedule: Option<Vec<ScheduleData>>,

    // For create_post / update_post — service areas
    pub service_areas: Option<Vec<ServiceAreaData>>,

    // For create_post / update_post — tags (dynamic, loaded from tag_kinds)
    pub tags: Option<HashMap<String, Vec<String>>>,
    // Keys are tag_kind slugs from DB: "audience_role", "population",
    // "community_served", "service_offered", "post_type", "service_language", etc.

    // For update_post / archive_post / add_note targeting an existing post
    pub target_post_id: Option<String>, // References POST-{uuid} from org document

    // For merge_posts
    pub merge_post_ids: Option<Vec<String>>,

    // For add_note
    pub note_content: Option<String>,
    pub note_severity: Option<String>, // "urgent", "notice", "info"

    // For flag_contradiction
    pub contradiction_details: Option<String>,
}

// --- Structured data types matching the full database schema ---

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, StructuredOutput)]
pub struct LocationData {
    pub address: Option<String>,            // Full street address
    pub address_line_2: Option<String>,     // Suite, apartment, etc.
    pub city: Option<String>,               // "Minneapolis"
    pub state: Option<String>,              // "MN" (2-letter)
    pub postal_code: Option<String>,        // "55401" (5-digit)
    pub location_type: Option<String>,      // "physical", "virtual", "postal"
    pub accessibility_notes: Option<String>, // Wheelchair access, parking
    pub transportation_notes: Option<String>, // Transit, bus routes
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, StructuredOutput)]
pub struct ContactData {
    pub contact_type: String, // "phone", "email", "website", "address", "booking_url", "social"
    pub value: String,        // The actual phone/email/URL
    pub label: Option<String>, // "Main", "Booking", "Support", "Intake Form"
}

/// Supports three schedule modes:
/// 1. One-off event: date + start_time + end_time (or is_all_day)
/// 2. Recurring: frequency + day_of_week + start_time + end_time
/// 3. Operating hours: day_of_week + opens_at + closes_at
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, StructuredOutput)]
pub struct ScheduleData {
    // Mode 1: One-off events
    pub date: Option<String>,               // "YYYY-MM-DD" for one-time events
    pub date_end: Option<String>,           // "YYYY-MM-DD" if multi-day event

    // Mode 2: Recurring events
    pub frequency: Option<String>,          // "weekly", "biweekly", "monthly", "one_time"
    pub day_of_week: Option<String>,        // "monday", "tuesday", etc. (for recurring)
    pub rrule: Option<String>,              // iCalendar RRULE (e.g., "FREQ=WEEKLY;BYDAY=MO,WE,FR")

    // Mode 3: Operating hours (also uses day_of_week)
    pub opens_at: Option<String>,           // "HH:MM" 24h format (e.g., "09:00")
    pub closes_at: Option<String>,          // "HH:MM" 24h format (e.g., "17:00")

    // Common fields
    pub start_time: Option<String>,         // "HH:MM" 24h (for events, not operating hours)
    pub end_time: Option<String>,           // "HH:MM" 24h
    pub is_all_day: Option<bool>,           // All-day event
    pub duration_minutes: Option<i32>,      // Duration for recurring events
    pub timezone: Option<String>,           // Default "America/Chicago"
    pub valid_from: Option<String>,         // "YYYY-MM-DD" seasonal start
    pub valid_to: Option<String>,           // "YYYY-MM-DD" seasonal end
    pub notes: Option<String>,              // "By appointment only", "1st and 3rd week only"
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, StructuredOutput)]
pub struct ServiceAreaData {
    pub area_type: String,   // "county", "city", "state", "zip", "custom"
    pub area_name: String,   // "Hennepin County", "Minneapolis", "MN"
    pub area_code: Option<String>, // FIPS code, ZIP code, state abbreviation
}

const CONSULTANT_SYSTEM_PROMPT: &str = r#"
You are a social media consultant for a community organization that serves immigrant
communities in Minnesota. You're reviewing everything known about this organization
and recommending actions to keep their presence on our community platform accurate
and helpful.

## Your Role

You act like a thoughtful social media manager would:
- Look at all sources (website, social media) for the full picture
- Identify what's new, what's changed, what's contradictory, what's stale
- Recommend specific actions with clear reasoning

## Available Actions

1. **create_post** — A new service, event, or opportunity that should be listed.
   Must pass the litmus test: connected to immigrant communities AND actionable.

   **Required:** title, summary (2-3 sentences, ~250 chars), description (comprehensive markdown)

   **Structured data — ALWAYS include when available in sources:**
   - **location**: Full address, city, state, postal_code, location_type (physical/virtual/postal)
   - **contacts**: ALL found contact methods — phone, email, website, booking_url, intake form URLs.
     Include a label for each (e.g., "Main", "Intake Form", "Booking").
   - **schedule**: Use the correct mode:
     - One-off events: date (YYYY-MM-DD) + start_time/end_time or is_all_day
     - Recurring: frequency (weekly/biweekly/monthly) + day_of_week + times.
       Use rrule for complex patterns (e.g., "FREQ=WEEKLY;BYDAY=MO,WE,FR").
       Include valid_from/valid_to for seasonal services.
     - Operating hours: day_of_week + opens_at/closes_at
     - Add notes for exceptions: "1st and 3rd week only", "by appointment", "closed holidays"
   - **service_areas**: Geographic coverage (county, city, state, zip, custom)
   - **tags**: Classify using available tag kinds:
     - audience_role: "participant", "volunteer", "donor"
     - population: "refugees", "seniors", "youth", "families", etc.
     - community_served: Cultural communities (e.g., "somali", "hmong", "latino")
     - service_offered: "legal-aid", "food-assistance", "housing", etc.
     - service_language: Languages offered (e.g., "spanish", "somali", "karen")
   - **post_type**: "service", "opportunity", "business", "professional"
   - **category**: "food-assistance", "legal-aid", "housing", "education", etc.
   - **urgency**: "low", "medium", "high", "urgent"
   - **capacity_status**: "accepting", "paused", "at_capacity" (if mentioned in source)

2. **update_post** — An existing post needs changes. Reference it by its POST-{id}.
   Include only the fields that need updating. Can update any field: narrative content,
   schedule, contacts, location, tags, urgency, capacity_status, etc.

3. **add_note** — Important context that doesn't warrant a full post update.
   Example: "Their social media says they're not accepting donations right now,
   but their website still lists donation drop-off hours."
   Include note_content and severity (urgent/notice/info).

4. **merge_posts** — Two or more existing posts describe the same thing.
   List the POST-{id}s that should be merged.

5. **archive_post** — An existing post is stale, outdated, or no longer relevant.
   Reference by POST-{id} with reasoning.

6. **flag_contradiction** — Sources disagree about something important.
   Describe what's contradictory and which sources conflict.

## Rules

- ONLY propose actions grounded in the source material. Never fabricate information.
- Every action MUST have at least one source_url backing it.
- Prefer fewer, higher-quality actions over many low-confidence ones.
- If nothing needs to change, return an empty actions array. That's fine.
- For create_post: title should be action-focused, 5-10 words, no org name.
  Summary should be 2-3 sentences emphasizing the human need.
- Do NOT create posts for: regular worship services, job postings, governance,
  "about us" content, past events.
- Set confidence to "low" if you're unsure. Admins can reject low-confidence actions.
"#;

pub async fn run_consultant(
    org_document: &str,
    deps: &ServerDeps,
) -> Result<ConsultantResponse> {
    let response = deps.ai.extract::<ConsultantResponse>(
        GPT_5_MINI,
        CONSULTANT_SYSTEM_PROMPT,
        org_document,
    ).await?;

    // Validate: every action must have source_urls
    let validated_actions: Vec<_> = response.actions
        .into_iter()
        .filter(|a| !a.source_urls.is_empty() || a.action_type == "merge_posts")
        .collect();

    Ok(ConsultantResponse {
        actions: validated_actions,
        org_summary: response.org_summary,
    })
}
```

**Key design decisions:**

- **Single structured output call** — One LLM call returns all proposed actions. The consultant sees everything and reasons holistically.
- **GPT-5-mini** — Consistent with the codebase pattern. Upgrade to GPT-5 only if quality is insufficient.
- **Source URL grounding** — Every action must reference source pages. Actions without sources are filtered out (hallucination guard).
- **Confidence field** — Maps to the `confidence` column on `sync_proposals`. Admins can filter by confidence.
- **Empty actions is valid** — If nothing needs to change, the consultant returns an empty array. No forced output.
- **Same quality standards** — The litmus test (immigration relevance + actionable) carries over from the current pipeline.

**Acceptance criteria:**

- [ ] `run_consultant()` takes org document string, returns structured `ConsultantResponse`
- [ ] System prompt enforces source grounding, quality standards, litmus test
- [ ] Actions without source URLs are filtered out (except merges which reference existing posts)
- [ ] Confidence levels (high/medium/low) included in each action
- [ ] Empty actions array accepted when nothing needs to change

---

#### Phase 5: Stage Consultant Actions as Proposals

Convert consultant actions into sync proposals using the existing proposal system.

**`packages/server/src/domains/consultant/activities/stage_actions.rs`**

```rust
/// Convert consultant actions into sync proposals.
pub async fn stage_consultant_actions(
    org_id: Uuid,
    actions: &[ConsultantAction],
    org_summary: &str,
    deps: &ServerDeps,
) -> Result<StagingResult> {
    let pool = &deps.db_pool;

    // Expire any existing pending batches for this org
    SyncBatch::expire_pending_for_resource("organization", org_id, pool).await?;

    // Create new batch
    let batch = SyncBatch::create(
        "organization",
        org_id,
        &format!("AI Consultant: {}", org_summary),
        actions.len() as i32,
        pool,
    ).await?;

    for action in actions {
        match action.action_type.as_str() {
            "create_post" => {
                // Create draft post with status = 'draft'
                let draft = Post::create_draft(
                    action.title.as_deref().unwrap_or("Untitled"),
                    action.summary.as_deref(),
                    action.description.as_deref(),
                    action.location.as_deref(),
                    action.urgency.as_deref(),
                    action.post_type.as_deref(),
                    org_id,
                    pool,
                ).await?;

                // Create structured data: location, contacts, schedule, tags, service areas
                if let Some(loc) = &action.location {
                    let location = Location::create_from_data(org_id, loc, pool).await?;
                    PostLocation::link(draft.id, location.id, true, pool).await?;
                }
                if let Some(contacts) = &action.contacts {
                    for c in contacts {
                        Contact::create(
                            "post", draft.id.into_uuid(),
                            &c.contact_type, &c.value, c.label.as_deref(),
                            pool,
                        ).await?;
                    }
                }
                if let Some(schedule) = &action.schedule {
                    for s in schedule {
                        Schedule::create_from_data(
                            "post", draft.id.into_uuid(), s, pool
                        ).await?;
                    }
                }
                if let Some(tags) = &action.tags {
                    for (kind_slug, values) in tags {
                        for value in values {
                            Tag::apply_to("post", draft.id.into_uuid(), kind_slug, value, pool).await?;
                        }
                    }
                }
                if let Some(areas) = &action.service_areas {
                    for area in areas {
                        ServiceArea::create(draft.id, area, pool).await?;
                    }
                }

                // Generate embedding for the draft
                if let Ok(embedding) = deps.embedding_service
                    .create_embedding(&draft.to_embedding_text())
                    .await
                {
                    Post::update_embedding(draft.id, &embedding, pool).await?;
                }

                SyncProposal::create_with_consultant_fields(
                    batch.id, "insert", "post",
                    Some(draft.id.into_uuid()), None,
                    &action.reasoning, &action.confidence,
                    &action.source_urls,
                    pool,
                ).await?;
            }
            "update_post" => {
                let target_id = parse_post_id(&action.target_post_id)?;

                // Create revision post
                let revision = Post::create_revision(
                    target_id,
                    action.title.as_deref(),
                    action.summary.as_deref(),
                    action.description.as_deref(),
                    pool,
                ).await?;

                SyncProposal::create_with_consultant_fields(
                    batch.id, "update", "post",
                    Some(revision.id.into_uuid()), Some(target_id.into_uuid()),
                    &action.reasoning, &action.confidence,
                    &action.source_urls,
                    pool,
                ).await?;
            }
            "add_note" => {
                // Create draft note (not yet attached)
                let note = Note::create_draft(
                    org_id,
                    action.note_content.as_deref().unwrap_or(""),
                    action.note_severity.as_deref().unwrap_or("info"),
                    action.target_post_id.as_deref()
                        .and_then(|id| parse_post_id_optional(id)),
                    pool,
                ).await?;

                SyncProposal::create_with_consultant_fields(
                    batch.id, "insert", "note",
                    Some(note.id.into_uuid()), None,
                    &action.reasoning, &action.confidence,
                    &action.source_urls,
                    pool,
                ).await?;
            }
            "merge_posts" => {
                let merge_ids = action.merge_post_ids.as_ref()
                    .map(|ids| ids.iter()
                        .filter_map(|id| parse_post_id_optional(id))
                        .collect::<Vec<_>>())
                    .unwrap_or_default();

                if merge_ids.len() >= 2 {
                    let proposal = SyncProposal::create_with_consultant_fields(
                        batch.id, "merge", "post",
                        None, Some(merge_ids[0].into_uuid()),
                        &action.reasoning, &action.confidence,
                        &action.source_urls,
                        pool,
                    ).await?;

                    // Record merge sources
                    for source_id in &merge_ids[1..] {
                        SyncProposalMergeSource::create(
                            proposal.id, *source_id, pool
                        ).await?;
                    }
                }
            }
            "archive_post" => {
                let target_id = parse_post_id(&action.target_post_id)?;

                SyncProposal::create_with_consultant_fields(
                    batch.id, "delete", "post",
                    None, Some(target_id.into_uuid()),
                    &action.reasoning, &action.confidence,
                    &action.source_urls,
                    pool,
                ).await?;
            }
            "flag_contradiction" => {
                // Create a note with the contradiction details
                let note = Note::create_draft(
                    org_id,
                    &format!(
                        "⚠️ Contradiction detected: {}",
                        action.contradiction_details.as_deref().unwrap_or("")
                    ),
                    "urgent",
                    action.target_post_id.as_deref()
                        .and_then(|id| parse_post_id_optional(id)),
                    pool,
                ).await?;

                SyncProposal::create_with_consultant_fields(
                    batch.id, "insert", "note",
                    Some(note.id.into_uuid()), None,
                    &action.reasoning, &action.confidence,
                    &action.source_urls,
                    pool,
                ).await?;
            }
            _ => {
                tracing::warn!("Unknown consultant action type: {}", action.action_type);
            }
        }
    }

    Ok(StagingResult {
        batch_id: batch.id,
        proposals_staged: actions.len(),
    })
}

fn parse_post_id(id_ref: &Option<String>) -> Result<PostId> {
    let id_str = id_ref.as_ref()
        .ok_or_else(|| anyhow::anyhow!("Missing target_post_id"))?;
    let uuid_str = id_str.strip_prefix("POST-").unwrap_or(id_str);
    let uuid = Uuid::parse_str(uuid_str)?;
    Ok(PostId::from_uuid(uuid))
}
```

**Key design decisions:**

- **Reuse existing proposal system** — Actions map directly to sync_proposals with existing operations (insert/update/delete/merge). The `ProposalHandler` trait handles approval/rejection.
- **New "note" entity type** — `NoteProposalHandler` needed (new) to handle approve/reject for notes.
- **"archive" maps to "delete"** — Uses existing soft-delete mechanism. Same effect, friendlier label.
- **"flag_contradiction" creates an urgent note** — No new entity type needed. Contradictions become notes with `urgent` severity.
- **Embeddings generated on draft creation** — So semantic search works for pending posts.

**Acceptance criteria:**

- [ ] Each consultant action type maps to appropriate sync_proposal operations
- [ ] Draft posts created in `pending_approval` status with embeddings
- [ ] Revision posts created for updates (existing pattern)
- [ ] Notes created as drafts, linked to proposals
- [ ] Merge proposals track all source post IDs
- [ ] `consultant_reasoning`, `confidence`, `source_urls` populated on each proposal
- [ ] Old pending batches expired before new batch created

---

#### Phase 6: ConsultOrgWorkflow (Restate)

The orchestrating workflow that ties phases 2-5 together.

**`packages/server/src/domains/consultant/restate/workflows/consult_org.rs`**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsultOrgRequest {
    pub organization_id: Uuid,
}
impl_restate_serde!(ConsultOrgRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsultOrgResult {
    pub batch_id: Option<Uuid>,
    pub actions_proposed: usize,
    pub pages_briefed: usize,
    pub status: String,
}
impl_restate_serde!(ConsultOrgResult);

#[restate_sdk::workflow]
#[name = "ConsultOrgWorkflow"]
pub trait ConsultOrgWorkflow {
    async fn run(req: ConsultOrgRequest) -> Result<ConsultOrgResult, HandlerError>;
    #[shared]
    async fn get_status(req: EmptyRequest) -> Result<String, HandlerError>;
}

impl ConsultOrgWorkflow for ConsultOrgWorkflowImpl {
    async fn run(
        &self,
        ctx: WorkflowContext<'_>,
        req: ConsultOrgRequest,
    ) -> Result<ConsultOrgResult, HandlerError> {
        let pool = &self.deps.db_pool;
        let org_id = req.organization_id;

        // 1. Load org + sources
        ctx.set("status", "Loading organization...");
        let (org, sources, source_map) = ctx.run(|| async {
            let org = Organization::find_by_id(org_id.into(), pool).await?;
            let sources = Source::find_active_by_organization(org_id.into(), pool).await?;
            let mut source_map = HashMap::new();
            for s in &sources {
                let site_url = s.site_url(pool).await?;
                source_map.insert(site_url, (s.id.into_uuid(), s.source_type.clone()));
            }
            Ok((org, sources, source_map))
        }).await?;

        if sources.is_empty() {
            return Ok(ConsultOrgResult {
                status: "no_sources".into(), actions_proposed: 0,
                pages_briefed: 0, batch_id: None,
            });
        }

        // 2. Load all extraction pages for this org's sources
        ctx.set("status", "Gathering crawled pages...");
        let pages = ctx.run(|| async {
            let extraction = self.deps.extraction.as_ref()
                .ok_or_else(|| anyhow::anyhow!("Extraction service not configured"))?;
            let mut all_pages = Vec::new();
            for (site_url, _) in &source_map {
                all_pages.extend(
                    extraction.get_pages_for_site(site_url).await.unwrap_or_default()
                );
            }
            Ok(all_pages)
        }).await?;

        if pages.is_empty() {
            return Ok(ConsultOrgResult {
                status: "no_pages".into(), actions_proposed: 0,
                pages_briefed: 0, batch_id: None,
            });
        }

        // 3. Extract page briefs (map step — parallel LLM calls, memo-cached)
        ctx.set("status", &format!("Briefing {} pages...", pages.len()));
        let briefs = ctx.run(|| async {
            extract_briefs_for_org(&org.name, &pages, &self.deps).await
        }).await?;

        if briefs.is_empty() {
            return Ok(ConsultOrgResult {
                status: "no_useful_content".into(), actions_proposed: 0,
                pages_briefed: 0, batch_id: None,
            });
        }

        // 4. Compile org document (deterministic)
        ctx.set("status", "Compiling org document...");
        let org_doc = ctx.run(|| async {
            compile_org_document(org_id, &org.name, &briefs, pool).await
        }).await?;

        // 5. Run consultant (single LLM call — the reduce step)
        ctx.set("status", "Consultant analyzing...");
        let response = ctx.run(|| async {
            run_consultant(&org_doc.content, &self.deps).await
        }).await?;

        if response.actions.is_empty() {
            // Update last_extracted_at even if no actions
            ctx.run(|| async {
                Organization::update_last_extracted(org_id.into(), pool).await
            }).await?;

            return Ok(ConsultOrgResult {
                status: "no_actions_needed".into(), actions_proposed: 0,
                pages_briefed: briefs.len(), batch_id: None,
            });
        }

        // 6. Stage actions as proposals
        ctx.set("status", &format!("Staging {} proposals...", response.actions.len()));
        let staging = ctx.run(|| async {
            stage_consultant_actions(
                org_id, &response.actions, &response.org_summary, &self.deps
            ).await
        }).await?;

        // 7. Update last_extracted_at
        ctx.run(|| async {
            Organization::update_last_extracted(org_id.into(), pool).await
        }).await?;

        Ok(ConsultOrgResult {
            batch_id: Some(staging.batch_id.into_uuid()),
            actions_proposed: staging.proposals_staged,
            pages_briefed: briefs.len(),
            status: "completed".into(),
        })
    }
}
```

**Register in `server.rs`:**
```rust
.bind(ConsultOrgWorkflowImpl::with_deps(server_deps.clone()).serve())
```

**Acceptance criteria:**

- [ ] `ConsultOrgWorkflow` registered in Restate, invocable by org_id
- [ ] All external calls wrapped in `ctx.run()` for durability
- [ ] Status tracking via `ctx.set("status", ...)` for UI polling
- [ ] Graceful handling of empty states (no sources, no pages, no briefs, no actions)
- [ ] `last_extracted_at` updated after completion
- [ ] Workflow key = org_id (Restate prevents concurrent runs for same org)

---

#### Phase 7: Comment Refinement Workflow

When an admin comments on a proposal, this workflow revises the proposal.

**`packages/server/src/domains/consultant/activities/refine_proposal.rs`**

```rust
const MAX_REVISIONS: i32 = 3;

const REFINEMENT_PROMPT: &str = r#"
You are revising a proposed action based on admin feedback.

The admin is a volunteer reviewer who knows the community. Their feedback should be
taken seriously and incorporated precisely.

Return the revised action with the same structure. Only change what the feedback asks for.
If the feedback contradicts the source material, note that in your reasoning but still
make the requested change — the admin has final say.
"#;

pub async fn refine_proposal_from_comment(
    proposal_id: Uuid,
    comment: &str,
    deps: &ServerDeps,
) -> Result<RefineResult> {
    let pool = &deps.db_pool;

    let proposal = SyncProposal::find_by_id(proposal_id.into(), pool).await?;

    if proposal.revision_count >= MAX_REVISIONS {
        return Ok(RefineResult::MaxRevisionsReached);
    }

    // Build context for the LLM: original proposal + all comments so far
    let comments = ProposalComment::find_by_proposal(proposal_id.into(), pool).await?;
    let draft = load_draft_content(&proposal, pool).await?;

    let user_prompt = format!(
        "## Original Proposal\nOperation: {}\nReasoning: {}\n\n## Current Draft\n{}\n\n## Comment History\n{}\n\n## Latest Comment\n{}",
        proposal.operation,
        proposal.consultant_reasoning.as_deref().unwrap_or(""),
        draft,
        format_comment_history(&comments),
        comment,
    );

    let revised = deps.ai.extract::<ConsultantAction>(
        GPT_5_MINI,
        REFINEMENT_PROMPT,
        &user_prompt,
    ).await?;

    // Apply revision to the draft entity
    apply_revision_to_draft(&proposal, &revised, pool).await?;

    // Update proposal metadata
    SyncProposal::increment_revision(proposal_id.into(), pool).await?;

    // Save the comment
    ProposalComment::create(
        proposal_id.into(),
        /* author_id from context */,
        comment,
        proposal.revision_count + 1,
        true, // ai_revised
        pool,
    ).await?;

    Ok(RefineResult::Revised)
}
```

**`packages/server/src/domains/consultant/restate/workflows/refine_proposal.rs`**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefineProposalRequest {
    pub proposal_id: Uuid,
    pub comment: String,
    pub author_id: Uuid,
}
impl_restate_serde!(RefineProposalRequest);

#[restate_sdk::workflow]
#[name = "RefineProposalWorkflow"]
pub trait RefineProposalWorkflow {
    async fn run(req: RefineProposalRequest) -> Result<String, HandlerError>;
}
```

**Acceptance criteria:**

- [ ] `refine_proposal_from_comment()` revises draft entity based on admin comment
- [ ] Comment history included in LLM context for multi-round refinement
- [ ] Max 3 revisions enforced — after that, admin must approve/reject/edit directly
- [ ] Comment stored in `proposal_comments` table with revision number
- [ ] `revision_count` incremented on `sync_proposals`
- [ ] `RefineProposalWorkflow` registered in Restate

---

#### Phase 8: NoteProposalHandler

New proposal handler for note entities (the existing system only handles posts).

**`packages/server/src/domains/consultant/activities/note_proposal_handler.rs`**

```rust
pub struct NoteProposalHandler;

impl ProposalHandler for NoteProposalHandler {
    fn entity_type(&self) -> &str { "note" }

    async fn approve(&self, proposal: &SyncProposal, _merge_sources: &[SyncProposalMergeSource], pool: &PgPool) -> Result<()> {
        match proposal.operation.as_str() {
            "insert" => {
                if let Some(draft_id) = proposal.draft_entity_id {
                    // Activate the draft note
                    Note::activate(draft_id.into(), pool).await?;
                    // Attach to relevant posts (by embedding similarity)
                    // Follows existing pattern from notes/activities/attachment.rs
                }
            }
            _ => {}
        }
        Ok(())
    }

    async fn reject(&self, proposal: &SyncProposal, _merge_sources: &[SyncProposalMergeSource], pool: &PgPool) -> Result<()> {
        if let Some(draft_id) = proposal.draft_entity_id {
            Note::delete(draft_id.into(), pool).await?;
        }
        Ok(())
    }
}
```

**Register handler alongside existing `PostProposalHandler`.**

**Acceptance criteria:**

- [ ] `NoteProposalHandler` implements `ProposalHandler` trait
- [ ] Approve on insert: activates draft note, attaches to relevant posts
- [ ] Reject on insert: deletes draft note
- [ ] Registered in the proposal handling dispatch logic

---

#### Phase 9: Triggers

Wire up the three trigger mechanisms.

**After-crawl trigger:**

In `crawl_website_full()` (or wherever crawl completion is handled), add a call to invoke the consultant:

```rust
// After crawl completes, trigger consultant for the org
if let Some(org_id) = website.organization_id {
    ctx.workflow_client::<ConsultOrgWorkflowClient>(org_id.to_string())
        .run(ConsultOrgRequest { organization_id: org_id.into_uuid() })
        .send();
}
```

**Scheduled trigger:**

Extend the existing `OrgExtractionScheduler` (or create equivalent) to invoke `ConsultOrgWorkflow` instead of `ExtractOrgPostsWorkflow`:

```rust
// In the scheduled extraction service
for org_id in &orgs_needing_extraction {
    ctx.workflow_client::<ConsultOrgWorkflowClient>(org_id.to_string())
        .run(ConsultOrgRequest { organization_id: org_id.into_uuid() })
        .send();
}
```

**On-demand trigger:**

Add a Restate service handler (or extend Organizations service) for admin-triggered consultant runs. Frontend button on org detail page calls this.

**Acceptance criteria:**

- [ ] After-crawl: consultant invoked automatically when crawl completes for an org
- [ ] Scheduled: periodic sweep finds orgs needing attention, invokes consultant
- [ ] On-demand: admin can trigger consultant from org detail page
- [ ] Restate workflow key = org_id prevents concurrent consultant runs
- [ ] Scheduled trigger uses same criteria as existing extraction scheduler

---

#### Phase 10: Frontend — Extended Proposals UI

Extend the existing proposals page to support comments and direct editing.

**Changes to `packages/web/app/admin/(app)/proposals/page.tsx`:**

1. **Comment input** — Add a text input on each proposal for admin comments
2. **Submit comment** — Calls `callService("Consultant", "refine_proposal", { proposal_id, comment })`
3. **Revision indicator** — Show revision count badge on proposals that have been revised
4. **Confidence badge** — Display confidence level (high/medium/low) alongside existing score badge
5. **Consultant reasoning** — Expandable section showing the AI's reasoning for each proposal
6. **Source URLs** — Clickable links to the source pages that support each action
7. **Direct edit** — For insert/update proposals, link to draft post edit page
8. **Note proposals** — Render note proposals with severity badge and content preview

**Changes to org detail page:**

- Add "Run Consultant" button alongside existing "Extract Org Posts" button
- Wire to `callService("ConsultOrgWorkflow", "run", { organization_id })`

**Acceptance criteria:**

- [ ] Comment input on each proposal with submit button
- [ ] "Revising..." loading state while refinement workflow runs
- [ ] Confidence badge (high = green, medium = yellow, low = orange)
- [ ] Consultant reasoning in expandable section
- [ ] Source URL links for each proposal
- [ ] Direct edit link to draft post/note
- [ ] "Run Consultant" button on org detail page
- [ ] Note proposals rendered with severity and content

---

### Migration Strategy

**Approach: Parallel operation with per-org flag.**

1. Add `use_consultant: boolean DEFAULT false` to `organizations` table
2. When `use_consultant = false`, existing `ExtractOrgPostsWorkflow` runs (current behavior)
3. When `use_consultant = true`, `ConsultOrgWorkflow` runs instead
4. Admin can toggle per org via org detail page
5. Gradually enable for orgs, validate quality, expand
6. Once all orgs migrated, remove old pipeline code

**Existing pending proposals** from the old pipeline are left untouched. The normal expiry mechanism handles them when the consultant creates new batches for the same org.

---

### ERD: New/Modified Models

```mermaid
erDiagram
    organizations {
        uuid id PK
        text name
        timestamp last_extracted_at
        boolean use_consultant "NEW - feature flag"
    }

    sync_batches ||--o{ sync_proposals : "contains"
    sync_proposals {
        uuid id PK
        uuid batch_id FK
        text operation
        text status
        text entity_type "post OR note"
        text consultant_reasoning "NEW"
        integer revision_count "NEW"
        text confidence "NEW"
        text[] source_urls "NEW"
    }

    sync_proposals ||--o{ proposal_comments : "has many"
    proposal_comments {
        uuid id PK
        uuid proposal_id FK
        uuid author_id FK
        text content
        integer revision_number
        boolean ai_revised
        timestamp created_at
    }
}
```

**Note:** Page briefs are cached in the existing `memo_cache` table via `deps.memo("page_brief_v1", ...)`. No dedicated table needed — content-based key means automatic invalidation on page changes.

## Acceptance Criteria

### Functional Requirements

- [ ] Page briefs extracted from crawled pages with `deps.memo()` caching
- [ ] Org document compiled with priority ordering and token budget
- [ ] Consultant proposes actions grounded in source material
- [ ] All 6 action types supported (create, update, note, merge, archive, flag)
- [ ] Admin can approve, reject, or comment on proposals
- [ ] Comments trigger AI revision (max 3 rounds)
- [ ] Admin can edit draft entities directly
- [ ] After-crawl, scheduled, and on-demand triggers all functional
- [ ] Per-org feature flag for gradual migration

### Non-Functional Requirements

- [ ] `deps.memo()` caching eliminates redundant LLM calls for unchanged pages
- [ ] Org document stays within 200k char budget
- [ ] Restate durability: workflow survives process restarts
- [ ] Concurrent consultant runs prevented by workflow key

### Quality Gates

- [ ] Consultant output validated: actions must have source URLs
- [ ] Revision count limit enforced (max 3)
- [ ] PII scrubbing applied to page content before briefing
- [ ] Prompt injection protection (sanitize_prompt_input) maintained

## Risk Analysis & Mitigation

| Risk | Impact | Mitigation |
|------|--------|------------|
| Consultant hallucination | High — fabricated info in posts | Source URL requirement, admin review, confidence levels |
| Large org exceeds token budget | Medium — incomplete context | Priority truncation, budget enforcement, ReadFullPage fallback |
| Revision loop frustration | Low — admin gets stuck | Max 3 revisions, direct edit escape hatch |
| Migration breaks existing flow | High — admins lose access to pending proposals | Per-org feature flag, parallel operation |
| Cost increase from page briefs | Medium — more LLM calls per page | `deps.memo()` caching — same content = cache hit, no LLM call |
| Prompt injection from crawled content | High — adversarial website content | sanitize_prompt_input(), [SYSTEM BOUNDARY] markers |

## References

### Internal References

- Current extraction pipeline: `packages/server/src/domains/crawling/activities/post_extraction.rs`
- Current LLM sync: `packages/server/src/domains/posts/activities/llm_sync.rs`
- Sync proposal model: `packages/server/src/domains/sync/models/sync_proposal.rs`
- ProposalHandler trait: `packages/server/src/domains/sync/activities/proposal_actions.rs`
- PostProposalHandler: `packages/server/src/domains/posts/activities/post_sync_handler.rs`
- Notes extraction: `packages/server/src/domains/notes/activities/extraction.rs`
- ExtractOrgPostsWorkflow: `packages/server/src/domains/organization/restate/workflows/extract_org_posts.rs`
- Org-level extraction plan: `docs/plans/2026-02-11-feat-org-level-post-extraction-pipeline-plan.md`
- Proposals UI: `packages/web/app/admin/(app)/proposals/page.tsx`
- Org detail page: `packages/web/app/admin/(app)/organizations/[id]/page.tsx`
- Restate client: `packages/web/lib/restate/client.ts`

### Brainstorm

- AI Consultant brainstorm: `docs/brainstorms/2026-02-12-ai-consultant-brainstorm.md`
