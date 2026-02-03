---
title: "Refactor: Extraction System Alignment"
type: refactor
date: 2026-02-03
---

# Refactor: Extraction System Alignment

## Overview

The codebase has **two parallel extraction systems** that evolved independently, causing confusion about responsibilities, duplicate type definitions, and inconsistent data flows. This plan aligns them into a coherent architecture with clear boundaries.

## Problem Statement

### Current State: Two Competing Systems

| System | Location | Philosophy | Output | Trigger |
|--------|----------|------------|--------|---------|
| **Extraction Library** | `packages/extraction/` | Query-driven, flexible | Markdown + metadata | GraphQL (manual) |
| **Agentic Posts** | `domains/posts/effects/` | Schema-based, tools | Rigid JSON (20+ fields) | Crawling cascade |

### Critical Inconsistencies

1. **Naming Collisions**
   - `PageExtraction` (crawling) ≠ `Extraction` (library) ≠ `EnrichedPost` (agentic)
   - No unified interface

2. **ContactInfo Defined 3 Ways**
   ```rust
   // agentic_extraction.rs
   pub struct ContactInfo { phone, email, intake_form_url, contact_name }

   // extraction_tools.rs
   pub struct ContactData { phone, email, intake_form_url, contact_name }

   // crawling/types.rs
   pub struct ContactInfo { phone, email, website, other }
   ```

3. **Dual Summarization Paths**
   - `PageSummary` (content_hash caching) → website display
   - `PageExtraction` (model tracking) → post extraction
   - Same concept, two mechanisms

4. **String-Based Type Constants**
   - `"summary"`, `"posts"`, `"contacts"`, `"hours"` - typos break silently

5. **Evidence Tracking Only in Library**
   - Library tracks: sources, conflicts, gaps, grounding
   - Agentic tracks: confidence float only
   - Schema-based tracks: nothing

## Proposed Solution

### Architecture: Clear Boundaries

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           USER ENTRY POINTS                                  │
├─────────────────────────────────────────────────────────────────────────────┤
│  GraphQL: submitUrl()     │  Crawling: crawlWebsite()   │  Query: extract() │
│  (ad-hoc URL submission)  │  (website discovery)        │  (search index)   │
└───────────┬───────────────┴───────────────┬─────────────┴────────┬──────────┘
            │                               │                      │
            ▼                               ▼                      │
┌───────────────────────────────────────────────────────────────────┐
│                    EXTRACTION ENGINE (unified)                     │
│  packages/extraction/ - THE source of truth for AI extraction      │
│  - Crawl pages → summarize → index                                 │
│  - Query index → extract structured data                           │
│  - Track: sources, gaps, conflicts, grounding                      │
└───────────────────────────────────────────────────────────────────┘
            │
            ▼
┌───────────────────────────────────────────────────────────────────┐
│                    POST-PROCESSING (server-side)                   │
│  domains/posts/actions/ - Transform extractions → domain models    │
│  - Map extraction results to Post records                          │
│  - Apply business rules (eligibility, dedup, PII scrubbing)        │
│  - Store in normalized tables                                      │
└───────────────────────────────────────────────────────────────────┘
```

### Key Decisions

1. **Extraction library is THE engine** - All AI extraction goes through `packages/extraction/`
2. **Server domain transforms results** - `domains/posts/` maps extractions to Posts
3. **Agentic enrichment moves to library** - Tool-calling for contact/location/schedule
4. **Unified types** - Single ContactInfo, LocationInfo, ScheduleInfo shared across domains
5. **Evidence tracking everywhere** - Sources and confidence on all extractions

## Technical Approach

### Phase 1: Unify Type Definitions

**Create shared types module**: `packages/server/src/common/extraction_types.rs`

```rust
// SINGLE source of truth for extraction-related types
// Used by: crawling domain, posts domain, extraction domain

/// Contact information extracted from content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactInfo {
    pub phone: Option<String>,
    pub email: Option<String>,
    pub website: Option<String>,
    pub intake_form_url: Option<String>,
    pub contact_name: Option<String>,
}

/// Location/address information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationInfo {
    pub address: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub service_area: Option<String>,
    pub is_virtual: bool,
}

/// Schedule/hours information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleInfo {
    pub hours: Option<String>,           // "Mon-Fri 9am-5pm"
    pub dates: Option<String>,           // Specific dates
    pub frequency: Option<String>,       // "weekly", "monthly"
    pub duration: Option<String>,        // "2 hours"
    pub by_day: Option<Vec<DayHours>>,   // Structured hours
}

/// Eligibility/requirements information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EligibilityInfo {
    pub who_qualifies: Option<String>,
    pub requirements: Vec<String>,
    pub restrictions: Option<String>,
}

/// Extraction type enum (replaces string constants)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ExtractionType {
    Summary,
    Posts,
    Contacts,
    Hours,
    Events,
    Services,
}
```

**Files to update:**
- [ ] `domains/posts/effects/agentic_extraction.rs` - Use shared ContactInfo
- [ ] `domains/posts/effects/extraction_tools.rs` - Use shared types
- [ ] `domains/crawling/effects/extraction/types.rs` - Use shared types
- [ ] `domains/crawling/models/page_extraction.rs` - Use ExtractionType enum

### Phase 2: Clarify Extraction Triggers

**Current confusion:** When does each system run?

| Trigger | Current Behavior | Target Behavior |
|---------|------------------|-----------------|
| `crawlWebsite` | Agentic extraction → posts | Library extraction → post mapping |
| `submitUrl` | Library extraction (raw) | Library extraction + post mapping |
| `triggerExtraction` | Library query (raw) | Library query + optional mapping |
| `regeneratePosts` | Agentic extraction | Library extraction + post mapping |

**Action items:**
- [ ] Move agentic extraction logic INTO extraction library as "enrichment tools"
- [ ] Create `domains/posts/actions/map_extraction_to_posts.rs` - Transform extractions
- [ ] Update crawling handlers to use library → mapping pipeline

### Phase 3: Consolidate Summarization

**Problem:** Two caching mechanisms for page summaries

| Current | Storage | Cache Key | Usage |
|---------|---------|-----------|-------|
| `PageSummary` | page_summaries table | content_hash | Website display |
| `PageExtraction` | page_extractions table | model+version | Post extraction |

**Solution:** Merge into single caching layer

```rust
// Use extraction library's summary cache
// packages/extraction/src/stores/postgres.rs already has:
// - summary_cache table
// - Content hash-based invalidation
// - Model/version tracking

// Remove PageSummary model, use library's cache
```

**Files to change:**
- [ ] Remove `domains/crawling/models/page_summary.rs` (use library cache)
- [ ] Update `domains/website/data/website.rs` to read from library cache
- [ ] Migrate existing page_summaries data to extraction library tables

### Phase 4: Add Evidence Tracking to All Extractions

**Current gap:** Agentic extraction doesn't track sources/conflicts

**Add to EnrichedPost:**
```rust
pub struct EnrichedPost {
    // ... existing fields ...

    // NEW: Evidence tracking (matches extraction library)
    pub sources: Vec<Source>,           // Where info came from
    pub confidence: ConfidenceLevel,    // Verified/SingleSource/Inferred
    pub gaps: Vec<String>,              // What couldn't be found
    pub conflicts: Vec<Conflict>,       // Contradictory info
}
```

**Files to update:**
- [ ] `domains/posts/effects/agentic_extraction.rs` - Add source tracking
- [ ] `domains/posts/effects/extraction_tools.rs` - Tools record sources
- [ ] `domains/extraction/data.rs` - GraphQL types already have this

### Phase 5: Unify Storage Model

**Problem:** `PageExtraction.content` is untyped JSON

**Solution:** Add schema validation + use typed enum

```rust
// page_extraction.rs
impl PageExtraction {
    pub fn create_typed<T: Serialize>(
        pool: &PgPool,
        page_snapshot_id: PageSnapshotId,
        extraction_type: ExtractionType,  // Enum, not string
        content: &T,                       // Typed, not Value
        model: Option<String>,
    ) -> Result<Self> {
        let content_value = serde_json::to_value(content)?;
        // Validate schema based on extraction_type
        Self::validate_schema(&extraction_type, &content_value)?;
        // ... insert
    }

    fn validate_schema(extraction_type: &ExtractionType, content: &Value) -> Result<()> {
        match extraction_type {
            ExtractionType::Posts => {
                // Validate against EnrichedPost schema
            }
            ExtractionType::Summary => {
                // Validate against SummaryContent schema
            }
            // ...
        }
    }
}
```

### Phase 6: Clarify Domain Ownership

| Domain | Owns | Does NOT Own |
|--------|------|--------------|
| `extraction` | AI extraction engine, query interface, index storage | Business rules, post creation |
| `crawling` | Website discovery, page fetching, crawl orchestration | AI extraction (delegates to extraction) |
| `posts` | Post domain models, approval workflow, deduplication | AI extraction (delegates to extraction) |

**Files to move/refactor:**
- [ ] Move `agentic_extraction.rs` tool-calling logic to `packages/extraction/src/tools/`
- [ ] Keep `domains/posts/actions/` for post-specific business logic
- [ ] Keep `domains/crawling/actions/` for crawl orchestration only

## Acceptance Criteria

### Functional Requirements
- [ ] Single ContactInfo type used across all domains
- [ ] Single LocationInfo type used across all domains
- [ ] ExtractionType enum replaces all string constants
- [ ] All extractions track sources and confidence
- [ ] `submitUrl` and `crawlWebsite` produce comparable results
- [ ] Library summarization replaces PageSummary model

### Non-Functional Requirements
- [ ] No duplicate type definitions
- [ ] Clear domain boundaries (extraction vs posts vs crawling)
- [ ] Evidence tracking on all AI outputs
- [ ] Schema validation for PageExtraction content

### Quality Gates
- [ ] All existing tests pass
- [ ] New tests for unified types
- [ ] No string-based extraction types in codebase

## Implementation Phases

### Phase 1: Shared Types (Foundation)
**Effort:** Small
**Risk:** Low

Tasks:
- [ ] Create `common/extraction_types.rs` with unified types
- [ ] Update imports across domains to use shared types
- [ ] Create ExtractionType enum
- [ ] Update PageExtraction to use enum

### Phase 2: Evidence Tracking
**Effort:** Medium
**Risk:** Low

Tasks:
- [ ] Add Source/Confidence/Gaps to EnrichedPost
- [ ] Update agentic extraction to populate evidence fields
- [ ] Update GraphQL types to expose evidence

### Phase 3: Consolidate Summarization
**Effort:** Medium
**Risk:** Medium (data migration)

Tasks:
- [ ] Migrate page_summaries to extraction library tables
- [ ] Remove PageSummary model
- [ ] Update website display to use library cache

### Phase 4: Unify Extraction Pipeline
**Effort:** Large
**Risk:** Medium

Tasks:
- [ ] Move tool-calling to extraction library
- [ ] Create post mapping layer in posts domain
- [ ] Update crawling handlers to use unified pipeline

### Phase 5: Cleanup
**Effort:** Small
**Risk:** Low

Tasks:
- [ ] Remove duplicate type definitions
- [ ] Add schema validation to PageExtraction
- [ ] Update documentation

## Success Metrics

- **Type duplication:** 0 (currently 3+ ContactInfo definitions)
- **String constants:** 0 (currently 5+ extraction type strings)
- **Evidence coverage:** 100% of extractions have source tracking
- **Clear ownership:** Each extraction concept has exactly one owner

## Dependencies & Risks

### Dependencies
- Extraction library must support tool-calling (Phase 4)
- Database migration for page_summaries (Phase 3)

### Risks
| Risk | Mitigation |
|------|------------|
| Breaking existing crawl workflows | Parallel run both systems during transition |
| Data migration failures | Create rollback script for page_summaries |
| Performance regression | Benchmark extraction pipeline before/after |

## Gaps Identified (SpecFlow Analysis)

### Type Completeness Gaps

**Gap 1: Missing LocationInfo fields**
- Current `LocationInfo` has `state: Option<String>` and `zip: Option<String>`
- Proposed type omits these
- **Resolution:** Add `state` and `zip` to unified `LocationInfo`

**Gap 2: Missing ContactInfo.other field**
- Current `ContactInfo` has `other: Vec<String>` for TTY, fax, etc.
- **Resolution:** Add `other: Vec<String>` to unified `ContactInfo`

**Gap 3: DayHours struct not defined**
- `ScheduleInfo.by_day` references `DayHours` but it's not defined
- **Resolution:** Include `DayHours` struct definition

**Gap 4: CallToAction not in unified types**
- `EnrichedPost` has `CallToAction` but no unified equivalent
- **Resolution:** Either add to unified types or merge into `ContactInfo`

### Migration Gaps

**Gap 5: No migration SQL for page_summaries**
- Plan says "migrate" but no script
- **Resolution:** Create migration script before Phase 3

**Gap 6: Existing PageExtraction content validation**
- Plan adds validation but doesn't address existing invalid records
- **Resolution:** Grandfather existing records; validate only new writes

**Gap 7: Parallel run semantics undefined**
- Which system is source of truth during transition?
- **Resolution:** New system overwrites for same page_snapshot_id

### Evidence Tracking Gaps

**Gap 8: Tool → Source mapping undefined**
- How do tool calls become `Source` objects?
- **Resolution:** Each tool finding creates `Source` with `role: Supporting`

**Gap 9: confidence → GroundingGrade mapping**
- Current: `confidence: f32` (0-1)
- Target: `GroundingGrade` enum
- **Resolution:** Define thresholds:
  - 0.8+ = Verified
  - 0.5-0.8 = SingleSource
  - <0.5 = Inferred

### Critical Questions to Answer Before Implementation

1. **Source of truth during transition?** New system results overwrite old.
2. **Existing untyped PageExtraction records?** Grandfathered; validate new only.
3. **EnrichedPost → unified type mapping?** Create explicit mapping function.
4. **Tool → Source population?** Tool calls create Supporting sources.

## Updated Type Definitions

Based on gaps, the complete unified types should be:

```rust
// common/extraction_types.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactInfo {
    pub phone: Option<String>,
    pub email: Option<String>,
    pub website: Option<String>,
    pub intake_form_url: Option<String>,
    pub contact_name: Option<String>,
    pub other: Vec<String>,  // TTY, fax, etc.
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationInfo {
    pub address: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,       // ADDED
    pub zip: Option<String>,         // ADDED
    pub service_area: Option<String>,
    pub is_virtual: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleInfo {
    pub hours: Option<String>,
    pub dates: Option<String>,
    pub frequency: Option<String>,
    pub duration: Option<String>,
    pub by_day: Option<Vec<DayHours>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DayHours {                // ADDED
    pub day: String,                 // "Monday", "Tuesday", etc.
    pub open: Option<String>,        // "9:00 AM"
    pub close: Option<String>,       // "5:00 PM"
    pub closed: bool,                // True if closed that day
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EligibilityInfo {
    pub who_qualifies: Option<String>,
    pub requirements: Vec<String>,
    pub restrictions: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallToAction {            // ADDED
    pub action: String,
    pub url: Option<String>,
    pub instructions: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfidenceLevel {
    High,    // >= 0.8
    Medium,  // 0.5 - 0.8
    Low,     // < 0.5
}

impl From<f32> for ConfidenceLevel {
    fn from(confidence: f32) -> Self {
        if confidence >= 0.8 { Self::High }
        else if confidence >= 0.5 { Self::Medium }
        else { Self::Low }
    }
}
```

## References

### Internal References
- Extraction library: `packages/extraction/src/pipeline/`
- Agentic extraction: `packages/server/src/domains/posts/effects/agentic_extraction.rs`
- Page extraction model: `packages/server/src/domains/crawling/models/page_extraction.rs`
- Architecture docs: `docs/architecture/SEESAW_ARCHITECTURE.md`

### Related Files
| File | Current Role | Target Role |
|------|--------------|-------------|
| `agentic_extraction.rs` | Full extraction logic | Thin wrapper calling library |
| `extraction_tools.rs` | Tool definitions | Move to extraction library |
| `page_extraction.rs` | Untyped storage | Typed storage with validation |
| `page_summary.rs` | Separate cache | REMOVE (use library cache) |
| `common/extraction_types.rs` | Does not exist | Single source of truth |
