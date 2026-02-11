---
title: "feat: Post Relevance Scoring System"
type: feat
date: 2026-02-11
---

# Post Relevance Scoring System

## Overview

Add a standalone LLM-based scoring pass that evaluates every post on a 1-10 composite relevance scale. Scores are stored on the post record and surfaced in the admin UI so human reviewers can triage efficiently: high-confidence posts get quick approval, low-confidence posts are flagged as likely noise.

The scorer is independent from extraction — it works on both newly extracted posts (as Pass 4 in the pipeline) and existing posts (via batch scoring), enabling retroactive cleanup of posts extracted under the old broad prompt.

## Problem Statement / Motivation

The extraction prompt was just rewritten to focus on immigration-crisis-related events. But even with a tighter prompt, some borderline posts will get through. And hundreds of existing posts were extracted under the old broad prompt — many are noise (worship services, generic programs).

Human reviewers currently see all proposals with equal weight. They have no signal about which posts are likely relevant and which are likely noise, making review slow and tedious.

## Proposed Solution

A **standalone scoring activity** that:
1. Takes post content (title + summary + description) and org name as input
2. Asks the LLM to evaluate three factors: immigration relevance, actionability, completeness
3. Returns a composite 1-10 score + human-readable breakdown
4. Stores the result on the post record

Integrated at two points:
- **New posts:** Scored after extraction (Pass 4), before proposals are surfaced
- **Existing posts:** Batch-scored via admin-triggered Restate workflow

## Technical Approach

### Architecture

```
New Post Pipeline:
  Pass 1 (Extract) → Pass 2 (Dedupe) → Pass 3 (Investigate)
  → [NEW] Pass 4 (Score) → LLM Sync → Proposals UI (with scores)

Existing Post Batch Scoring:
  Admin triggers → Load unscored active posts → Score each
  → Store scores → Admin filters by score in posts list
```

### Scoring Activity

New file: `packages/server/src/domains/posts/activities/scoring.rs`

```rust
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct RelevanceScoreResponse {
    /// Immigration relevance score (1-10)
    pub immigration_relevance: i32,
    /// Immigration relevance reasoning
    pub immigration_relevance_reasoning: String,
    /// Actionability score (1-10)
    pub actionability: i32,
    /// Actionability reasoning
    pub actionability_reasoning: String,
    /// Completeness score (1-10)
    pub completeness: i32,
    /// Completeness reasoning
    pub completeness_reasoning: String,
}

pub struct RelevanceScore {
    /// Weighted composite (1-10)
    pub score: i32,
    /// Human-readable breakdown
    pub breakdown: String,
}

/// Score a single post's relevance.
/// LLM returns sub-scores; code computes weighted composite.
pub async fn score_post_relevance(
    title: &str,
    summary: Option<&str>,
    description: &str,
    org_name: &str,
    ai: &OpenAi,
) -> Result<RelevanceScore>
```

**Key design decision:** The LLM returns three sub-scores (each 1-10) plus reasoning per factor. Code computes the weighted composite: `relevance*0.5 + actionability*0.3 + completeness*0.2`, rounded to nearest integer. This means weights can be adjusted without re-scoring.

**Scoring prompt** evaluates:

| Factor | Weight | What it measures |
|--------|--------|-----------------|
| Immigration relevance | 50% | Connected to immigrant communities / ICE / the crisis |
| Actionability | 30% | Specific event, drive, or action someone can show up to |
| Completeness | 20% | Has date, location, contact info, clear next steps |

**Breakdown format** (stored as TEXT):
```
Relevance: 9/10 — ICE rapid response training for community members.
Actionability: 4/10 — No specific date or signup link mentioned.
Completeness: 3/10 — Missing contact info and location.
Composite: 6/10
```

### Schema Changes

New migration: `packages/server/migrations/000164_add_relevance_scoring_to_posts.sql`

```sql
ALTER TABLE posts ADD COLUMN relevance_score INTEGER;
ALTER TABLE posts ADD COLUMN relevance_breakdown TEXT;
ALTER TABLE posts ADD COLUMN scored_at TIMESTAMP WITH TIME ZONE;

CREATE INDEX idx_posts_relevance_score ON posts(relevance_score)
    WHERE relevance_score IS NOT NULL;

COMMENT ON COLUMN posts.relevance_score IS 'Composite relevance score 1-10 (immigration relevance 50%, actionability 30%, completeness 20%)';
```

Three columns:
- `relevance_score INTEGER` — the composite 1-10 score (nullable until scored)
- `relevance_breakdown TEXT` — human-readable per-factor breakdown
- `scored_at TIMESTAMPTZ` — when the score was generated (for staleness detection)

Index on `relevance_score` for efficient filtering in admin queries.

### Post Model Updates

File: `packages/server/src/domains/posts/models/post.rs`

Add fields to `Post` struct:
```rust
pub relevance_score: Option<i32>,
pub relevance_breakdown: Option<String>,
pub scored_at: Option<DateTime<Utc>>,
```

Add model method:
```rust
impl Post {
    pub async fn update_relevance_score(
        id: PostId,
        score: i32,
        breakdown: &str,
        pool: &PgPool,
    ) -> Result<()> {
        sqlx::query_as::<_, Self>(
            "UPDATE posts SET relevance_score = $1, relevance_breakdown = $2,
             scored_at = NOW(), updated_at = NOW() WHERE id = $3"
        )
        .bind(score)
        .bind(breakdown)
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn find_unscored_active(pool: &PgPool) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM posts
             WHERE status = 'active'
             AND relevance_score IS NULL
             AND revision_of_post_id IS NULL
             AND translation_of_id IS NULL
             AND duplicate_of_id IS NULL
             AND deleted_at IS NULL
             ORDER BY created_at DESC"
        )
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }
}
```

### Pipeline Integration (Pass 4)

Scoring happens **after** `create_extracted_post` persists the post, as an UPDATE. This is less invasive than threading scores through `ExtractedPost` → `create_extracted_post`.

In `llm_sync.rs`, after each INSERT operation creates a draft post:
1. Call `score_post_relevance(title, summary, description, org_name, ai)`
2. Call `Post::update_relevance_score(post_id, score, breakdown, pool)`

**Failure handling:** Best-effort. If scoring fails, the post proceeds with `relevance_score = NULL`. Consistent with Pass 3 investigation failure handling. The admin UI shows "Unscored" for NULL scores.

**Both pipeline paths** need integration:
- `RegeneratePostsWorkflow` (per-website)
- `ExtractOrgPostsWorkflow` (per-org)

### Batch Scoring for Existing Posts

New Restate service endpoint on `PostsService`:

```rust
async fn batch_score_posts(&self, ctx: Context<'_>) -> Result<BatchScoreResult, HandlerError>
```

Flow:
1. Load unscored active posts via `Post::find_unscored_active()`
2. For each post, resolve org name (via post_sources → sources → organizations)
3. Call `score_post_relevance()` sequentially (avoid rate limiting)
4. Store score via `Post::update_relevance_score()`
5. Return summary: total scored, score distribution

**Scope:** Only active, non-revision, non-translation, non-duplicate, non-deleted posts with `relevance_score IS NULL`.

**Org name resolution:** If post has no organization, use source domain or "Unknown Organization". The scorer can still evaluate relevance from content alone.

**Posts with NULL summary:** Pass empty string. The scorer works primarily from title + description.

### Admin UI Updates

#### Proposals Page (`packages/web/app/admin/(app)/proposals/page.tsx`)

- Add **score badge** to each proposal card, color-coded:
  - Green (8-10): high confidence
  - Yellow (5-7): review needed
  - Red (1-4): likely noise
  - Gray: unscored
- Add **score filter dropdown**: "All", "High (8-10)", "Review (5-7)", "Noise (1-4)", "Unscored"
- Show breakdown on click/expand (not hover — mobile-friendly)

#### Posts Page (`packages/web/app/admin/(app)/posts/[id]/page.tsx`)

- Show score badge in post header
- Show full breakdown below score

#### TypeScript Types (`packages/web/lib/restate/types.ts`)

Add to post-related types:
```typescript
relevance_score: number | null;
relevance_breakdown: string | null;
scored_at: string | null;
```

### Score Staleness

When post content changes (revision approved, admin edit), clear the score:
- Set `relevance_score = NULL`, `relevance_breakdown = NULL`, `scored_at = NULL`
- The post appears as "Unscored" in admin UI
- Next batch scoring pass or manual re-score picks it up

This is handled in the model layer — `approve_revision()` and any content-update methods should clear scoring fields.

## Edge Cases Addressed

| Edge Case | Handling |
|-----------|----------|
| Scoring LLM fails | Best-effort: post proceeds with NULL score |
| Post has no org | Use source domain or "Unknown Organization" |
| Post has NULL summary | Pass empty string to scorer |
| Revision posts | Excluded from batch scoring; scored when created as draft |
| Translation posts | Excluded from batch scoring |
| Duplicate posts | Excluded from batch scoring |
| Content changes after scoring | Score cleared (set to NULL) |
| Proposals for UPDATE ops | Show revision post's score |
| Proposals for DELETE ops | Show existing post's score |

## Acceptance Criteria

### Functional Requirements
- [x] `score_post_relevance()` activity returns composite score + breakdown
- [x] Composite computed in code from sub-scores (weights adjustable without re-scoring)
- [x] New posts scored as Pass 4 before proposals are surfaced
- [x] Batch scoring endpoint scores all unscored active posts
- [x] Scores stored on post record with `scored_at` timestamp
- [x] Scores cleared when post content changes
- [ ] Admin proposals page shows color-coded score badges
- [ ] Admin proposals page supports filtering by score threshold
- [x] Admin post detail page shows score and breakdown

### Non-Functional Requirements
- [x] Scoring is best-effort — failures don't block the pipeline
- [x] Batch scoring handles hundreds of posts without rate limit issues
- [x] Score filter queries use the index efficiently

## Implementation Phases

### Phase 1: Scoring Activity + Migration
1. Create migration `000164_add_relevance_scoring_to_posts.sql`
2. Add fields to `Post` model + query methods
3. Create `scoring.rs` activity with prompt and `score_post_relevance()` function
4. Register in `mod.rs`

### Phase 2: Pipeline Integration
5. Integrate scoring into `llm_sync.rs` INSERT path (score after `create_extracted_post`)
6. Integrate into both `RegeneratePostsWorkflow` and `ExtractOrgPostsWorkflow` paths
7. Add score-clearing logic to `approve_revision()` and content-update methods

### Phase 3: Batch Scoring
8. Add `batch_score_posts` endpoint to `PostsService`
9. Implement org name resolution for existing posts
10. Add progress logging for batch operations

### Phase 4: Admin UI
11. Add score fields to TypeScript types
12. Add score badge component (color-coded)
13. Add score filter to proposals page
14. Add score display to post detail page

## Files Changed

| File | Change |
|------|--------|
| `packages/server/migrations/000164_add_relevance_scoring_to_posts.sql` | NEW — schema migration |
| `packages/server/src/domains/posts/models/post.rs` | Add score fields + query methods |
| `packages/server/src/domains/posts/activities/scoring.rs` | NEW — scoring activity |
| `packages/server/src/domains/posts/activities/mod.rs` | Register + re-export scoring |
| `packages/server/src/domains/posts/activities/llm_sync.rs` | Call scorer after INSERT |
| `packages/server/src/domains/posts/activities/revision_actions.rs` | Clear score on revision approve |
| `packages/server/src/domains/posts/restate/services/posts.rs` | Add batch_score endpoint |
| `packages/server/src/domains/website/restate/workflows/regenerate_posts.rs` | Ensure scoring in pipeline |
| `packages/server/src/domains/organization/restate/workflows/extract_org_posts.rs` | Ensure scoring in pipeline |
| `packages/web/lib/restate/types.ts` | Add score fields to types |
| `packages/web/app/admin/(app)/proposals/page.tsx` | Score badges + filter |
| `packages/web/app/admin/(app)/posts/[id]/page.tsx` | Score display + breakdown |

## References

- Brainstorm: `docs/brainstorms/2026-02-11-focused-post-extraction-brainstorm.md`
- Extraction pipeline: `packages/server/src/domains/crawling/activities/post_extraction.rs`
- LLM sync: `packages/server/src/domains/posts/activities/llm_sync.rs`
- Post model: `packages/server/src/domains/posts/models/post.rs`
- Proposals UI: `packages/web/app/admin/(app)/proposals/page.tsx`
- AI client extract: `packages/ai-client/src/openai/mod.rs:81`
