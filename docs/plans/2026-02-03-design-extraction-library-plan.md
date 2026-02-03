# Plan: Domain-Agnostic Site Extraction Library

## Enhancement Summary

**Deepened on:** 2026-02-03
**Research agents used:** architecture-strategist, performance-oracle, security-sentinel, code-simplicity-reviewer, pattern-recognition-specialist, best-practices-researcher (Rust async, RAG patterns), agent-native-reviewer, Context7 (tokio, sqlx)

### Key Improvements from Research

1. **Simplified Pipeline** - Collapse 4-phase to 2-phase (recall+partition → extract+verify) for ~40% code reduction
2. **Security Hardening** - SSRF prevention, URL allowlist validation, credential isolation
3. **Performance Optimization** - Batch LLM calls, use tower rate limiting, 1024-dim embeddings
4. **Error Handling** - Use `thiserror` for typed errors, proper cancellation with `CancellationToken`
5. **Agent-Native Design** - Expose primitives (search, read, extract_from), add streaming support

### Critical Findings

| Area | Finding | Action |
|------|---------|--------|
| Security | SSRF vulnerability in crawlers | Add URL validation + allowlist |
| Performance | N+1 LLM calls in partition phase | Batch summaries into single call |
| Simplicity | Claims/Evidence adds complexity | Keep internal, derive confidence from source count |
| Rust Patterns | Native async traits available (Rust 2024) | Use native for internal, async-trait for dyn |

### Gemini Review Refinements (Round 1)

| Refinement | Problem | Solution |
|------------|---------|----------|
| "Assumed" Evidence | Claims without source evidence are hallucinations | Add `strict_mode` - discard Assumed claims, only keep Direct/Inferred |
| Summary Staleness | Prompt changes invalidate cached summaries | Add `prompt_hash` to Summary for cache invalidation on prompt updates |
| Large Site Pagination | 100+ pages overwhelms LLM context | Implement "Ranked Recall" - top N summaries by embedding similarity first |

### Gemini Review Refinements (Round 2 - Soundness)

| Gap | Problem | Solution |
|-----|---------|----------|
| Query Type Handling | Partitioning isn't right for all queries | **Strategy Orchestrator** - classify intent before extraction |
| Conflicting Information | Pages contradict each other | **Conflicts field** - expose tension, don't resolve |
| Discovery Strategy | Breadth-first crawl misses deep pages | **Informed Crawling** - query-driven, not blind |
| Summary SPOF | Summary loses info, downstream fails | **Hybrid Recall** - BM25 on raw text as safety net |
| Iterative Refinement | No "try harder" pattern | **Refine primitive** - agent-driven, not auto-retry |
| Confidence Ceremony | Arbitrary 0.0-1.0 math | **Grounding Grade enum** - Verified/SingleSource/Conflicted/Inferred |

---

## Overview

Design and build a general-purpose, query-driven extraction library that can crawl any website and extract structured information based on natural language queries. The library is completely domain-agnostic - it knows nothing about "posts", "jobs", "products", etc. Applications provide the semantics.

## Design Philosophy

**"Let LLMs be LLMs"**

- Query-driven, not schema-driven
- Plain text output (markdown), not rigid JSON
- Evidence-grounded for accuracy
- Maximum flexibility, minimum interference
- Library handles mechanics, app handles semantics

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    extraction-lib (new crate)                   │
│                    Domain-Agnostic Engine                       │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  CRAWL ──► INDEX ──► QUERY ──► EXTRACT ──► RETURN              │
│                                                                 │
│  - Crawl pages (Firecrawl, direct, etc.)                       │
│  - Build recall-optimized index with summaries + embeddings     │
│  - Answer natural language queries                              │
│  - Return evidence-grounded extractions                         │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼ Vec<Extraction>
┌─────────────────────────────────────────────────────────────────┐
│                    Application Layer                            │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Extraction.content ──► App parses ──► Domain types            │
│                                                                 │
│  App provides: queries, validation rules, domain mapping        │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Research Insights: Architecture

**Trait Design (from architecture-strategist):**
- Split `PageStore` into focused traits: `PageCache`, `SummaryCache`, `EmbeddingStore`
- Use `tower::Service` pattern for composable middleware (rate limiting, retries)
- Add `CancellationToken` support for long-running operations

**Error Handling (from Rust async research):**
```rust
// Use thiserror for library errors (not anyhow)
#[derive(Debug, thiserror::Error)]
pub enum ExtractionError {
    #[error("crawl failed: {0}")]
    Crawl(#[from] CrawlError),

    #[error("AI service unavailable: {0}")]
    AI(#[source] Box<dyn std::error::Error + Send + Sync>),

    #[error("page not found: {url}")]
    PageNotFound { url: String },

    #[error("operation cancelled")]
    Cancelled,
}
```

**Cancellation Support:**
```rust
pub async fn extract(
    &self,
    query: &str,
    cancel: CancellationToken,  // From tokio_util
) -> Result<Vec<Extraction>, ExtractionError> {
    tokio::select! {
        result = self.do_extract(query) => result,
        _ = cancel.cancelled() => Err(ExtractionError::Cancelled),
    }
}
```

## Core Pipeline

### Phase 1: Crawl & Summarize

```
Pages ──► Summarize each ──► Store in PageStore (cached)
```

Summaries are **recall-optimized**, not just readable:
- Preserve calls-to-action, offers, asks, roles
- Store embeddings for semantic search
- Extract structured signals (CTAs, verbs, entities)

**Research Insights: Summarization**

**Performance (from performance-oracle):**
- Batch summarization: Send 5-10 pages per LLM call with clear separators
- Use streaming for large batches to get early results
- Cache embeddings separately from summaries (different invalidation cadence)

**RAG Best Practices (from research):**
- Use 512-1024 token chunks for summaries (sweet spot for retrieval)
- Hybrid search: BM25 keyword + semantic vectors with reranking
- Consider Cohere embed-v4 (1024 dims) or text-embedding-3-small (1536 dims)

```rust
// Batch summarization pattern
async fn summarize_batch(pages: &[CachedPage], ai: &impl AI) -> Vec<Summary> {
    let combined = pages.iter()
        .map(|p| format!("=== PAGE: {} ===\n{}", p.url, p.content))
        .collect::<Vec<_>>()
        .join("\n\n---\n\n");

    let response = ai.complete_json(&format!(
        "Summarize each page. Return JSON array:\n{}\n\n{}",
        BATCH_SUMMARY_SCHEMA, combined
    )).await?;

    // Parse array response
}
```

### Phase 2: Build Site Index

```
All summaries ──► SiteIndex (queryable knowledge base)
```

The index is a **live knowledge base** that any part of the pipeline can query:
- "Which pages mention volunteering?"
- "Where is the contact information?"
- "What pages discuss pricing?"

**Research Insights: Indexing**

**Hybrid Search (from RAG research):**
```rust
pub struct SiteIndex<S: PageStore> {
    store: S,
    // BM25 for keyword matching
    bm25: tantivy::Index,
    // Vector store for semantic search
    embeddings: Vec<(String, Vec<f32>)>,  // url -> embedding
}

impl<S: PageStore, A: AI> Index<S, A> {
    /// Hybrid search: combine BM25 + semantic, then rerank
    pub async fn search(&self, query: &str, limit: usize) -> Vec<PageRef> {
        let keyword_results = self.bm25_search(query, limit * 2);
        let semantic_results = self.semantic_search(query, limit * 2).await;

        // Reciprocal Rank Fusion (RRF) for combining
        let combined = reciprocal_rank_fusion(&keyword_results, &semantic_results);
        combined.into_iter().take(limit).collect()
    }
}
```

**Embedding Dimensions:**
- 1024 dims recommended (Cohere embed-v4, OpenAI small)
- Good balance of quality vs storage/compute
- pgvector handles 1024 dims efficiently

### Phase 3: Query → Extract

```
index.extract("volunteer opportunities")
        │
        ▼
┌───────────────────────────────────────┐
│ RECALL (over-inclusive)               │
│ - Semantic expansion of query         │
│ - Synonym matching                    │
│ - Signal-based search (CTAs, asks)    │
│ - Union, not intersection             │
└───────────────────────────────────────┘
        │
        ▼
┌───────────────────────────────────────┐
│ PARTITION (query-scoped)              │
│ - "What constitutes ONE item?"        │
│ - Group pages by distinct items       │
│ - LLM explains grouping rationale     │
└───────────────────────────────────────┘
        │
        ▼
┌───────────────────────────────────────┐
│ EXTRACT (evidence-grounded)           │
│ - Fetch full content for each bucket  │
│ - LLM extracts with citations         │
│ - Internal: claims + evidence         │
└───────────────────────────────────────┘
        │
        ▼
┌───────────────────────────────────────┐
│ VERIFY (internal)                     │
│ - Check claims against evidence       │
│ - Flag unsupported statements         │
│ - Calculate confidence score          │
└───────────────────────────────────────┘
        │
        ▼
Vec<Extraction>
```

**Research Insights: Pipeline Simplification**

**Simplicity Review Finding:** The 4-phase pipeline can be collapsed to 2 phases:

```
SIMPLIFIED PIPELINE (recommended):
┌─────────────────────────────────┐     ┌─────────────────────────────────┐
│ RECALL + PARTITION              │     │ EXTRACT + VERIFY                │
│ (Single LLM call)               │ ──► │ (Single LLM call per bucket)    │
│                                 │     │                                 │
│ - Semantic search               │     │ - Fetch full pages              │
│ - Signal matching               │     │ - Extract with citations        │
│ - Group into buckets            │     │ - Calculate confidence          │
└─────────────────────────────────┘     └─────────────────────────────────┘
```

**Benefits:**
- Fewer LLM round-trips (2 vs 4)
- Simpler internal state machine
- ~40% less code to maintain
- Same accuracy (evidence grounding still happens in extract phase)

**Implementation:**
```rust
// Phase 1: Recall + Partition in one call
let buckets = ai.recall_and_partition(query, &summaries).await?;

// Phase 2: Extract + Verify per bucket (parallelized)
let extractions = futures::future::join_all(
    buckets.iter().map(|bucket| {
        let pages = self.store.get_pages(&bucket.urls).await?;
        ai.extract_with_verification(query, &pages).await
    })
).await;
```

**Gemini Refinement: Ranked Recall for Large Sites**

For sites with 100+ pages, sending all summaries to the LLM would:
- Exceed token limits
- Dilute the LLM's attention on relevant content
- Increase latency and cost

**Solution: Two-stage recall**

```rust
const MAX_SUMMARIES_FOR_PARTITION: usize = 50;

async fn recall_and_partition(&self, query: &str) -> Result<Vec<Partition>> {
    let all_summaries = self.store.get_summaries_for_site(&self.site_url).await?;

    // Stage 1: Ranked recall via embeddings (fast, cheap)
    let ranked_summaries = if all_summaries.len() > MAX_SUMMARIES_FOR_PARTITION {
        let query_embedding = self.ai.embed(query).await?;
        let mut scored: Vec<_> = all_summaries.into_iter()
            .map(|s| {
                let score = cosine_similarity(&query_embedding, &s.embedding);
                (score, s)
            })
            .collect();
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
        scored.into_iter()
            .take(MAX_SUMMARIES_FOR_PARTITION)
            .map(|(_, s)| s)
            .collect()
    } else {
        all_summaries
    };

    // Stage 2: LLM partitioning on top N summaries
    self.ai.partition(query, &ranked_summaries).await
}
```

**Benefits:**
- Embedding search is fast and cheap (~50ms vs ~5s for LLM)
- Keeps LLM context focused on most relevant pages
- Scales to sites with thousands of pages

## Strategy Orchestrator (Gemini Round 2)

**The Problem:** Partitioning is a "map-reduce" strategy. It's great for high-cardinality data (lists of things) but overkill for point-lookups and potentially lossy for aggregations.

**The Fix:** Classify query intent before choosing extraction strategy.

```rust
pub enum ExtractionStrategy {
    /// "Find all X" - partition into buckets, extract each
    /// Example: "Find volunteer opportunities"
    Collection,

    /// "Find specific info" - single answer from relevant pages
    /// Example: "What is their phone number?"
    Singular,

    /// "Summarize/describe" - aggregate across all relevant pages
    /// Example: "What does this organization do?"
    Narrative,
}
```

**Strategy Selection (lightweight LLM call or heuristics):**

```rust
impl<S: PageStore, A: AI> Index<S, A> {
    async fn classify_query(&self, query: &str) -> ExtractionStrategy {
        // Heuristics first (cheap)
        if query.starts_with("find all") || query.contains("list of") {
            return ExtractionStrategy::Collection;
        }
        if query.starts_with("what is the") || query.contains("phone") || query.contains("email") {
            return ExtractionStrategy::Singular;
        }
        if query.contains("summarize") || query.contains("describe") || query.contains("what does") {
            return ExtractionStrategy::Narrative;
        }

        // Fall back to LLM classification
        self.ai.classify_query_intent(query).await
    }

    pub async fn extract(&self, query: &str) -> Vec<Extraction> {
        let strategy = self.classify_query(query).await;

        match strategy {
            ExtractionStrategy::Collection => {
                // Recall → Partition → Extract per bucket
                let buckets = self.recall_and_partition(query).await?;
                self.extract_buckets(query, buckets).await
            }
            ExtractionStrategy::Singular => {
                // Recall → Extract single answer (no partitioning)
                let pages = self.recall(query).await?;
                vec![self.extract_single(query, &pages).await?]
            }
            ExtractionStrategy::Narrative => {
                // Recall → Summarize context → Generate narrative
                let pages = self.recall(query).await?;
                vec![self.extract_narrative(query, &pages).await?]
            }
        }
    }
}
```

**Pipeline by Strategy:**

| Strategy | Pipeline | Output |
|----------|----------|--------|
| Collection | Recall → Partition → Extract each | `Vec<Extraction>` (one per item) |
| Singular | Recall → Extract | `Vec<Extraction>` (single element) |
| Narrative | Recall → Summarize → Extract | `Vec<Extraction>` (single element) |

## Conflict Detection (Gemini Round 2)

**The Problem:** Websites contradict themselves. Header says "Open 24/7", footer says "Closed Sundays".

**The Take:** The library shouldn't resolve conflicts. Its job is to **expose the tension**.

```rust
pub struct Extraction {
    pub content: String,
    pub sources: Vec<Source>,
    pub gaps: Vec<String>,
    pub grounding: GroundingGrade,  // Replaces confidence: f32
    pub conflicts: Vec<Conflict>,   // NEW: exposed contradictions
}

pub struct Conflict {
    pub topic: String,
    pub claims: Vec<ConflictingClaim>,
}

pub struct ConflictingClaim {
    pub statement: String,
    pub source_url: String,
}
```

**Example output:**
```json
{
  "content": "Volunteer hours are available...",
  "conflicts": [
    {
      "topic": "Schedule",
      "claims": [
        { "statement": "Tuesdays at 5pm", "source_url": "/volunteer" },
        { "statement": "Wednesdays at 6pm", "source_url": "/calendar" }
      ]
    }
  ]
}
```

**Application decides:** "Trust /calendar over /volunteer" or "Flag for human review".

## Grounding Grade (Gemini Round 2)

**Replaces arbitrary confidence float with meaningful enum:**

```rust
pub enum GroundingGrade {
    /// Multiple independent sources agree
    Verified,

    /// Only one page mentioned it
    SingleSource,

    /// Sources disagree (see conflicts field)
    Conflicted,

    /// Not explicitly stated, LLM inferred (WARNING)
    Inferred,
}
```

**Derived during verification phase:**
```rust
fn calculate_grounding(sources: &[Source], conflicts: &[Conflict], has_inference: bool) -> GroundingGrade {
    if !conflicts.is_empty() {
        return GroundingGrade::Conflicted;
    }
    if has_inference {
        return GroundingGrade::Inferred;
    }
    if sources.len() >= 2 {
        return GroundingGrade::Verified;
    }
    GroundingGrade::SingleSource
}
```

## Informed Crawling (Gemini Round 2)

**The Problem:** Breadth-first crawl misses deep-linked pages that might have the answer.

**The Fix:** Query-driven crawling.

```rust
pub struct InformedCrawler<C: Crawler, S: SearchService> {
    http_crawler: C,
    search: S,
}

impl<C: Crawler, S: SearchService> InformedCrawler<C, S> {
    /// Crawl strategy informed by the extraction query
    pub async fn crawl_for_query(
        &self,
        site_url: &str,
        query: &str,
        config: &CrawlConfig,
    ) -> Result<Vec<CrawledPage>> {
        // 1. Search-based discovery (jump to relevant pages)
        let search_results = self.search
            .search(&format!("site:{} {}", site_url, query), Some(20), None, None)
            .await?;

        let search_urls: Vec<_> = search_results.iter().map(|r| r.url.clone()).collect();

        // 2. Standard crawl from root
        let crawled = self.http_crawler.crawl(config).await?;

        // 3. Fetch search-discovered pages not in crawl
        let crawled_urls: HashSet<_> = crawled.iter().map(|p| &p.url).collect();
        let missing: Vec<_> = search_urls.iter()
            .filter(|url| !crawled_urls.contains(url))
            .collect();

        let additional = self.http_crawler.fetch_pages(&missing).await?;

        // 4. Combine
        Ok([crawled, additional].concat())
    }
}
```

**Deep crawl on gaps:** If extraction returns gaps, agent can trigger targeted crawl on specific sub-paths.

## Hybrid Recall (Gemini Round 2)

**The Problem:** Summary is a single point of failure. If summary loses info, downstream fails.

**The Fix:** BM25 on raw text as safety net alongside semantic search on summaries.

```rust
impl<S: PageStore, A: AI> Index<S, A> {
    /// Hybrid recall: semantic on summaries + keyword on raw text
    pub async fn recall(&self, query: &str) -> Result<Vec<PageRef>> {
        // 1. Semantic search on summaries (broad, conceptual)
        let semantic_results = self.semantic_search(query, 30).await?;

        // 2. Keyword search on RAW TEXT (safety net for specific terms)
        let keyword_results = self.keyword_search_raw(query, 30).await?;

        // 3. Boost keyword results for specific terms
        // If query has proper nouns, IDs, or rare terms, weight keyword higher
        let has_specific_terms = self.detect_specific_terms(query);

        let combined = if has_specific_terms {
            // Weight keyword results higher
            reciprocal_rank_fusion(&semantic_results, &keyword_results, 0.4, 0.6)
        } else {
            // Weight semantic higher for conceptual queries
            reciprocal_rank_fusion(&semantic_results, &keyword_results, 0.6, 0.4)
        };

        Ok(combined.into_iter().take(50).collect())
    }

    /// Keyword search directly on raw page content (bypasses summary)
    async fn keyword_search_raw(&self, query: &str, limit: usize) -> Result<Vec<PageRef>> {
        // BM25 search on stored raw content
        self.bm25_index.search(query, limit)
    }
}
```

**Key insight:** Summary-based semantic search finds conceptually related pages. Keyword search on raw text catches specific terms the summary might have dropped.

## Refine Primitive (Gemini Round 2)

**The Problem:** Auto-retry on gaps can be an infinite loop burning tokens.

**The Fix:** Explicit agent-driven refinement pattern.

```rust
impl<S: PageStore, A: AI> Index<S, A> {
    // Existing primitives
    pub async fn search(&self, query: &str, limit: usize) -> Vec<PageRef>;
    pub async fn read(&self, urls: &[&str]) -> Vec<CachedPage>;
    pub async fn extract_from(&self, query: &str, pages: &[CachedPage]) -> Extraction;

    // NEW: Targeted search for filling gaps
    pub async fn search_for_gap(&self, gap: &str) -> Vec<PageRef> {
        // Keyword-heavy search for specific missing info
        self.keyword_search_raw(gap, 10).await
    }
}
```

**Agent refinement pattern (using machine-readable gaps):**
```rust
// 1. Initial extraction
let result = index.extract("volunteer opportunities").await?;

// 2. Check for gaps
if !result.gaps.is_empty() {
    for gap in &result.gaps {
        // gap.query is machine-readable, e.g.:
        // "the contact email for the volunteer coordinator"
        // NOT just "email" - no reformulation needed!

        // 3. Pipe gap.query directly to search
        let gap_pages = index.search_for_gap(&gap.query).await;

        if !gap_pages.is_empty() {
            // 4. Read and extract from new pages
            let pages = index.read(&gap_pages.urls()).await;

            // 5. Use gap.query as extraction query (no reformulation)
            let refined = index.extract_from(&gap.query, &pages).await;

            // 6. Merge refined info (app logic)
            result.merge(refined);
        }
    }
}
```

**The library provides primitives. The agent decides when to refine.**

## Key Design Decisions

### 1. No Dedupe Pipeline

Instead of: Extract per page → Merge → Dedupe

We do: Summarize all → LLM sees everything → Partition → Extract

The LLM naturally deduplicates when it sees the whole site context during partitioning.

### 2. Claims/Evidence are Internal

The LLM is prompted to produce claims with evidence (for accuracy), but this scaffolding is internal. The app receives a clean `Extraction` with content, sources, and confidence.

```rust
// Internal (for LLM reasoning)
struct InternalExtraction {
    claims: Vec<Claim>,      // Forces grounding
    evidence: Vec<Evidence>, // Proves citations
}

// External (what app sees)
struct Extraction {
    content: String,         // Markdown
    sources: Vec<Source>,    // Pages used
    gaps: Vec<String>,       // What wasn't found
    confidence: f32,         // Derived from claims
}
```

### 3. DB-Backed Caching via PageStore Trait

Library defines trait, app implements storage:

```rust
#[async_trait]
pub trait PageStore: Send + Sync {
    // Page content
    async fn get_page(&self, url: &str) -> Result<Option<CachedPage>>;
    async fn store_page(&self, page: &CachedPage) -> Result<()>;

    // Summaries
    async fn get_summary(&self, url: &str, content_hash: &str) -> Result<Option<Summary>>;
    async fn store_summary(&self, url: &str, content_hash: &str, summary: &Summary) -> Result<()>;

    // Embeddings for semantic search
    async fn store_embedding(&self, url: &str, embedding: &[f32]) -> Result<()>;
    async fn search_similar(&self, embedding: &[f32], limit: usize) -> Result<Vec<PageRef>>;

    // Bulk operations
    async fn get_all_summaries(&self, site_url: &str) -> Result<Vec<Summary>>;
    async fn get_pages(&self, urls: &[&str]) -> Result<Vec<CachedPage>>;
}
```

Library provides default implementations (SQLite, Postgres, in-memory).

**Research Insights: Storage**

**Trait Splitting (from architecture-strategist):**
Consider splitting into focused traits for flexibility:
```rust
// Separate concerns for different caching strategies
pub trait PageCache: Send + Sync {
    async fn get_page(&self, url: &str) -> Result<Option<CachedPage>>;
    async fn store_page(&self, page: &CachedPage) -> Result<()>;
}

pub trait SummaryCache: Send + Sync {
    async fn get_summary(&self, url: &str, hash: &str) -> Result<Option<Summary>>;
    async fn store_summary(&self, summary: &Summary) -> Result<()>;
}

pub trait EmbeddingStore: Send + Sync {
    async fn store(&self, url: &str, embedding: &[f32]) -> Result<()>;
    async fn search(&self, embedding: &[f32], limit: usize) -> Result<Vec<PageRef>>;
}

// Composite for convenience
pub trait PageStore: PageCache + SummaryCache + EmbeddingStore {}
```

**SQLx Pattern (from Context7 research):**
```rust
// Use query_as function, not macro (per CLAUDE.md)
impl PageStore for PostgresStore {
    async fn get_page(&self, url: &str) -> Result<Option<CachedPage>> {
        sqlx::query_as::<_, CachedPage>(
            "SELECT * FROM pages WHERE url = $1"
        )
        .bind(url)
        .fetch_optional(&self.pool)
        .await
        .map_err(Into::into)
    }
}
```

### 4. Translation is Implicit

No separate translation step. Just specify output language:

```rust
let site = scraper.index("https://example.org")
    .output_language("en")  // Summaries/extractions in English
    .await?;
```

LLMs handle multilingual content naturally.

## Public API

### Flat Index Model (Gemini Simplification)

**Key insight:** The index is the authority, the source is just a filter.

```
OLD (over-complicated):
┌─────────────┐  ┌─────────────┐  ┌─────────────┐
│ SiteIndex A │  │ SiteIndex B │  │ SiteIndex C │
└─────────────┘  └─────────────┘  └─────────────┘
       │                │                │
       └────── App orchestrates ─────────┘

NEW (flat index):
┌─────────────────────────────────────────────────┐
│                     Index                        │
│              (all pages, all sites)              │
│                                                  │
│  page.site_url = metadata for filtering          │
└─────────────────────────────────────────────────┘
```

**Benefits:**
- Cross-site deduplication happens naturally in Partition phase
- Simplified lifecycle (no multiple index objects)
- Standard DB indexing on `site_url` for fast filtered queries

### Core Types

```rust
/// The main entry point - a flat index over all ingested pages
pub struct Index<S: PageStore, A: AI> {
    store: S,
    ai: A,
}

/// Filter for scoping queries
pub struct QueryFilter {
    /// Only include pages from these sites (empty = all sites)
    pub include_sites: Vec<String>,
    /// Exclude pages from these sites
    pub exclude_sites: Vec<String>,
    /// Only pages fetched after this date
    pub min_date: Option<DateTime<Utc>>,
}

pub struct Extraction {
    /// The extracted content as markdown
    pub content: String,

    /// Pages that contributed to this extraction
    pub sources: Vec<Source>,

    /// Machine-readable queries for missing info (Gemini "Last Mile" tip)
    /// Example: "the contact email for the volunteer coordinator"
    /// Pipe directly to search_for_gap() without reformulation
    pub gaps: Vec<GapQuery>,

    /// How well-grounded is this extraction? (replaces arbitrary confidence float)
    pub grounding: GroundingGrade,

    /// Contradictions detected across sources
    pub conflicts: Vec<Conflict>,
}

pub enum GroundingGrade {
    Verified,      // Multiple sources agree
    SingleSource,  // Only one page mentioned it
    Conflicted,    // Sources disagree (see conflicts)
    Inferred,      // LLM guessed, not explicit (WARNING)
}

pub struct Conflict {
    pub topic: String,
    pub claims: Vec<ConflictingClaim>,
}

pub struct ConflictingClaim {
    pub statement: String,
    pub source_url: String,
}

/// Machine-readable gap for agent-driven refinement
pub struct GapQuery {
    /// Human-readable field name (e.g., "contact email")
    pub field: String,
    /// Search query - pipe directly to search_for_gap()
    /// (e.g., "the contact email for the volunteer coordinator")
    pub query: String,
}

pub struct Source {
    pub url: String,
    pub title: Option<String>,
    pub fetched_at: DateTime<Utc>,
    pub role: SourceRole,
    pub metadata: HashMap<String, String>,  // App pass-through
}

pub enum SourceRole {
    Primary,
    Supporting,
    Corroborating,
}
```

### Usage

```rust
// Initialize with storage backend
let store = PostgresStore::new(&db_pool).await?;
let index = Index::new(store, ai);

// Ingest sites (background job, crawl + summarize)
index.ingest("https://redcross.org").await?;
index.ingest("https://habitat.org").await?;
index.ingest("https://foodbank.org").await?;

// Query ALL sites (no filter)
let all_opportunities = index.extract("volunteer opportunities", None).await?;

// Query ONE site
let filter = QueryFilter {
    include_sites: vec!["redcross.org".into()],
    ..Default::default()
};
let redcross_only = index.extract("volunteer opportunities", Some(filter)).await?;

// Query with date filter
let recent_filter = QueryFilter {
    min_date: Some(Utc::now() - Duration::days(30)),
    ..Default::default()
};
let recent = index.extract("events", Some(recent_filter)).await?;

// Optional: hints to focus extraction
let results = index.extract("job listings", None)
    .with_hints(&["title", "salary", "location"])
    .await?;

// Query index directly (for enrichment / gap-filling)
let contact_pages = index.search("contact information", None).await?;

// Refresh stale pages for a specific site
index.refresh("https://redcross.org").await?;
```

### Domain-Agnostic Examples

```rust
// Job board
let jobs = index.extract("engineering positions").await?;

// E-commerce
let products = index.extract("items under $50").await?;

// Academic
let papers = index.extract("machine learning papers from 2024").await?;

// Real estate
let listings = index.extract("3 bedroom houses downtown").await?;

// Nonprofit (your app)
let posts = index.extract("volunteer opportunities, services, events").await?;
```

The library doesn't know what you're extracting. It just does it.

### Agent-Native Primitives

**From agent-native-reviewer:** Expose low-level primitives for agent integration:

```rust
impl<S: PageStore, A: AI> Index<S, A> {
    // PRIMITIVE: Search the index
    pub async fn search(&self, query: &str, limit: usize) -> Vec<PageRef>;

    // PRIMITIVE: Read specific pages
    pub async fn read(&self, urls: &[&str]) -> Vec<CachedPage>;

    // PRIMITIVE: Extract from specific pages (skip recall)
    pub async fn extract_from(
        &self,
        query: &str,
        pages: &[CachedPage],
    ) -> Vec<Extraction>;

    // HIGH-LEVEL: Full pipeline (uses primitives internally)
    pub async fn extract(&self, query: &str) -> Vec<Extraction>;
}
```

**Benefits:**
- Agents can compose their own workflows
- Streaming: Return results as buckets complete
- Feedback loops: Agent can refine search, try different pages

**Streaming Support:**
```rust
// Return stream for incremental results
pub fn extract_stream(
    &self,
    query: &str,
) -> impl Stream<Item = Result<Extraction, ExtractionError>> {
    async_stream::stream! {
        let buckets = self.recall_and_partition(query).await?;
        for bucket in buckets {
            let extraction = self.extract_bucket(&bucket).await?;
            yield Ok(extraction);
        }
    }
}
```

## Separation of Concerns

| Concern | Library | Application |
|---------|---------|-------------|
| Crawling & caching | ✓ | |
| Summarization | ✓ | |
| Recall expansion | ✓ | |
| Partitioning | ✓ | |
| Evidence grounding | ✓ | |
| Confidence calculation | ✓ | |
| What query to run | | ✓ |
| What fields matter | | ✓ |
| Domain types (Post, Job) | | ✓ |
| Validation rules | | ✓ |
| Gap-filling strategy | | ✓ |
| Storage schema | | ✓ (via PageStore) |

## Accuracy Principles

Based on OpenAI feedback:

1. **Recall must be over-inclusive** - False negatives worse than false positives
2. **Partitioning must be query-scoped** - What's "one item" depends on the query
3. **Extraction must be evidence-grounded** - LLM cites sources internally
4. **Confidence is derived, not declared** - Calculated from evidence quality

**Research Insights: Confidence Calculation**

**Gemini Round 2:** Replace arbitrary confidence float with meaningful `GroundingGrade` enum:

```rust
impl Extraction {
    /// Derive grounding from source analysis and conflict detection
    pub fn calculate_grounding(
        sources: &[Source],
        conflicts: &[Conflict],
        has_inference: bool,
    ) -> GroundingGrade {
        if !conflicts.is_empty() {
            return GroundingGrade::Conflicted;
        }
        if has_inference {
            return GroundingGrade::Inferred;  // WARNING: LLM guessed
        }
        if sources.len() >= 2 {
            return GroundingGrade::Verified;  // Multiple sources agree
        }
        GroundingGrade::SingleSource
    }
}
```

**Why this is better than a float:**
- `Verified` means something (multiple sources agree)
- `Conflicted` tells app to check the conflicts field
- `Inferred` is an explicit warning, not a low number
- No fake precision (0.73 vs 0.71 is meaningless)

## Security Considerations

**Critical findings from security-sentinel:**

### SSRF Prevention (CRITICAL)

Crawlers are vulnerable to Server-Side Request Forgery. A malicious site could redirect to internal services.

```rust
pub struct UrlValidator {
    allowed_schemes: HashSet<String>,
    blocked_hosts: HashSet<String>,
    blocked_cidrs: Vec<IpNet>,
}

impl UrlValidator {
    pub fn new() -> Self {
        Self {
            allowed_schemes: ["http", "https"].into_iter().map(String::from).collect(),
            blocked_hosts: ["localhost", "127.0.0.1", "::1", "metadata.google.internal"]
                .into_iter().map(String::from).collect(),
            blocked_cidrs: vec![
                "10.0.0.0/8".parse().unwrap(),
                "172.16.0.0/12".parse().unwrap(),
                "192.168.0.0/16".parse().unwrap(),
                "169.254.0.0/16".parse().unwrap(),  // Link-local / cloud metadata
            ],
        }
    }

    pub fn validate(&self, url: &str) -> Result<(), SecurityError> {
        let parsed = Url::parse(url)?;

        // Check scheme
        if !self.allowed_schemes.contains(parsed.scheme()) {
            return Err(SecurityError::DisallowedScheme(parsed.scheme().to_string()));
        }

        // Check host
        let host = parsed.host_str().ok_or(SecurityError::NoHost)?;
        if self.blocked_hosts.contains(host) {
            return Err(SecurityError::BlockedHost(host.to_string()));
        }

        // Resolve and check IP
        let ips: Vec<IpAddr> = tokio::net::lookup_host((host, 80)).await?
            .map(|s| s.ip()).collect();
        for ip in ips {
            for cidr in &self.blocked_cidrs {
                if cidr.contains(&ip) {
                    return Err(SecurityError::BlockedCidr(ip.to_string()));
                }
            }
        }

        Ok(())
    }
}
```

### Credential Isolation

Never log or expose API keys:

```rust
// Use secrecy crate for sensitive values
use secrecy::{Secret, ExposeSecret};

pub struct AIConfig {
    pub api_key: Secret<String>,
    pub model: String,
}

impl AIConfig {
    pub fn key(&self) -> &str {
        self.api_key.expose_secret()
    }
}

// Debug won't leak the key
impl std::fmt::Debug for AIConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AIConfig")
            .field("api_key", &"[REDACTED]")
            .field("model", &self.model)
            .finish()
    }
}
```

### Rate Limiting

Protect external services from abuse:

```rust
use governor::{Quota, RateLimiter};

pub struct RateLimitedCrawler<C: Crawler> {
    inner: C,
    limiter: RateLimiter<NotKeyed, InMemoryState, DefaultClock>,
}

impl<C: Crawler> RateLimitedCrawler<C> {
    pub fn new(crawler: C, requests_per_second: u32) -> Self {
        let quota = Quota::per_second(NonZeroU32::new(requests_per_second).unwrap());
        Self {
            inner: crawler,
            limiter: RateLimiter::direct(quota),
        }
    }
}

#[async_trait]
impl<C: Crawler> Crawler for RateLimitedCrawler<C> {
    async fn crawl(&self, config: &CrawlConfig) -> Result<Vec<CrawledPage>> {
        self.limiter.until_ready().await;
        self.inner.crawl(config).await
    }
}
```

## Data Structures

### CachedPage

```rust
pub struct CachedPage {
    pub url: String,
    pub content: String,
    pub content_hash: String,
    pub fetched_at: DateTime<Utc>,
    pub http_headers: HashMap<String, String>,
    pub metadata: HashMap<String, String>,
}
```

### Summary (Recall-Optimized)

```rust
pub struct Summary {
    pub url: String,
    pub text: String,
    pub signals: RecallSignals,
    pub language: Option<String>,
    pub created_at: DateTime<Utc>,
    /// Hash of the summarization prompt - invalidate cache when prompt changes
    pub prompt_hash: String,
}

pub struct RecallSignals {
    pub calls_to_action: Vec<String>,
    pub offers: Vec<String>,
    pub asks: Vec<String>,
    pub entities: Vec<String>,
}
```

## File Structure

```
packages/extraction/
├── Cargo.toml
├── src/
│   ├── lib.rs                    # Public API exports
│   ├── error.rs                  # Typed errors (ExtractionError, CrawlError, etc.)
│   │
│   ├── traits/                   # Core abstractions
│   │   ├── mod.rs
│   │   ├── ai.rs                 # AI trait (LLM abstraction)
│   │   ├── store.rs              # PageCache, SummaryCache, EmbeddingStore traits
│   │   └── crawler.rs            # Crawler trait + UrlValidator
│   │
│   ├── types/                    # Domain-agnostic data types
│   │   ├── mod.rs
│   │   ├── extraction.rs         # Extraction, Source, SourceRole
│   │   ├── page.rs               # CachedPage, PageRef
│   │   ├── summary.rs            # Summary, RecallSignals
│   │   └── config.rs             # ExtractionConfig, CrawlConfig
│   │
│   ├── index/                    # Site indexing
│   │   ├── mod.rs
│   │   ├── site_index.rs         # SiteIndex struct and methods
│   │   ├── summarizer.rs         # Page summarization with recall signals
│   │   └── search.rs             # Hybrid search (BM25 + semantic)
│   │
│   ├── pipeline/                 # Strategy-aware extraction pipeline
│   │   ├── mod.rs
│   │   ├── strategy.rs           # Query classification (Collection/Singular/Narrative)
│   │   ├── recall.rs             # Hybrid recall (semantic + BM25)
│   │   ├── partition.rs          # Bucket grouping for Collection queries
│   │   ├── extract.rs            # Evidence-grounded extraction
│   │   ├── conflicts.rs          # Contradiction detection across sources
│   │   └── grounding.rs          # GroundingGrade calculation
│   │
│   ├── stores/                   # Storage implementations
│   │   ├── mod.rs
│   │   ├── memory.rs             # In-memory store for testing
│   │   └── sqlite.rs             # SQLite store for local use
│   │
│   ├── crawlers/                 # Crawler implementations
│   │   ├── mod.rs
│   │   ├── validator.rs          # UrlValidator (SSRF protection)
│   │   ├── http.rs               # Direct HTTP crawler
│   │   ├── tavily.rs             # Tavily-powered discovery
│   │   ├── informed.rs           # Query-driven InformedCrawler (Gemini R2)
│   │   └── rate_limited.rs       # RateLimitedCrawler<C> wrapper
│   │
│   ├── security/                 # Security utilities
│   │   ├── mod.rs
│   │   └── credentials.rs        # Secret<String> wrappers
│   │
│   └── scraper.rs                # Main Scraper struct
│
└── tests/
    ├── common/
    │   └── mod.rs                # Test utilities, mock implementations
    ├── extraction_tests.rs       # End-to-end extraction tests
    ├── pipeline_tests.rs         # Pipeline phase tests
    ├── security_tests.rs         # SSRF and validation tests
    └── store_tests.rs            # Storage implementation tests
```

## Detailed Type Definitions

### Core Traits

**Research Insights: Trait Design**

Use native async traits for internal code (Rust 2024), `async-trait` only for dyn dispatch:

```rust
// traits/ai.rs

// For library internals - native async trait
pub trait AI: Send + Sync {
    /// Summarize page content with recall-optimized signals
    fn summarize(&self, content: &str, url: &str) -> impl Future<Output = Result<SummaryResponse>> + Send;

    /// Expand query for recall (synonyms, related concepts)
    fn expand_query(&self, query: &str) -> impl Future<Output = Result<Vec<String>>> + Send;

    /// Recall and partition in single call (simplified pipeline)
    fn recall_and_partition(
        &self,
        query: &str,
        summaries: &[Summary]
    ) -> impl Future<Output = Result<Vec<Partition>>> + Send;

    /// Extract from page content with evidence grounding
    fn extract(
        &self,
        query: &str,
        pages: &[CachedPage],
        hints: Option<&[String]>,
    ) -> impl Future<Output = Result<Extraction>> + Send;

    /// Generate embedding for text
    fn embed(&self, text: &str) -> impl Future<Output = Result<Vec<f32>>> + Send;
}

// For dynamic dispatch (when needed) - use async-trait
#[async_trait]
pub trait DynAI: Send + Sync {
    async fn summarize(&self, content: &str, url: &str) -> Result<SummaryResponse>;
    async fn extract(&self, query: &str, pages: &[CachedPage]) -> Result<Extraction>;
}
```

**Alternative using `async-trait` throughout (simpler, works with older MSRV):**
```rust
// traits/ai.rs
#[async_trait]
pub trait AI: Send + Sync {
    /// Summarize page content with recall-optimized signals
    async fn summarize(&self, content: &str, url: &str) -> Result<SummaryResponse>;

    /// Expand query for recall (synonyms, related concepts)
    async fn expand_query(&self, query: &str) -> Result<Vec<String>>;

    /// Partition candidates into distinct items
    async fn partition(
        &self,
        query: &str,
        summaries: &[Summary]
    ) -> Result<Vec<Partition>>;

    /// Extract from page content with evidence grounding
    async fn extract(
        &self,
        query: &str,
        pages: &[CachedPage],
        hints: Option<&[String]>,
    ) -> Result<InternalExtraction>;

    /// Generate embedding for text
    async fn embed(&self, text: &str) -> Result<Vec<f32>>;
}

// traits/store.rs
#[async_trait]
pub trait PageStore: Send + Sync {
    // Page content
    async fn get_page(&self, url: &str) -> Result<Option<CachedPage>>;
    async fn store_page(&self, page: &CachedPage) -> Result<()>;
    async fn get_pages_for_site(&self, site_url: &str) -> Result<Vec<CachedPage>>;

    // Summaries
    async fn get_summary(&self, url: &str, content_hash: &str) -> Result<Option<Summary>>;
    async fn store_summary(&self, summary: &Summary) -> Result<()>;
    async fn get_summaries_for_site(&self, site_url: &str) -> Result<Vec<Summary>>;

    // Embeddings
    async fn store_embedding(&self, url: &str, embedding: &[f32]) -> Result<()>;
    async fn search_similar(&self, embedding: &[f32], limit: usize) -> Result<Vec<PageRef>>;

    // Metadata
    async fn get_site_metadata(&self, site_url: &str) -> Result<Option<SiteMetadata>>;
    async fn store_site_metadata(&self, metadata: &SiteMetadata) -> Result<()>;
}

// traits/crawler.rs
#[async_trait]
pub trait Crawler: Send + Sync {
    /// Discover and fetch pages from a site
    async fn crawl(&self, config: &CrawlConfig) -> Result<Vec<CrawledPage>>;
}

pub struct CrawlConfig {
    pub url: String,
    pub max_pages: usize,
    pub max_depth: usize,
    pub rate_limit_ms: u64,
    pub respect_robots: bool,
}

/// Configuration for extraction pipeline
pub struct ExtractionConfig {
    /// Maximum summaries to send to LLM for partitioning (Gemini refinement)
    pub max_summaries_for_partition: usize,  // Default: 50

    /// Discard claims with "Assumed" grounding (Gemini refinement)
    /// When true: only Direct and Inferred claims kept
    /// When false: Assumed claims included but penalize confidence
    pub strict_mode: bool,  // Default: true (recommended for accuracy)

    /// Output language for summaries and extractions
    pub output_language: Option<String>,

    /// Hints for extraction (e.g., ["title", "date", "location"])
    pub hints: Vec<String>,
}

impl Default for ExtractionConfig {
    fn default() -> Self {
        Self {
            max_summaries_for_partition: 50,
            strict_mode: true,
            output_language: None,
            hints: vec![],
        }
    }
}
```

### Internal Types (LLM Reasoning Scaffolding)

```rust
// Internal only - not exposed in public API
struct InternalExtraction {
    content: String,
    sources: Vec<Source>,
    claims: Vec<Claim>,
    gaps: Vec<Gap>,
}

struct Claim {
    statement: String,
    evidence: Vec<Evidence>,
    grounding: Grounding,
}

struct Evidence {
    quote: String,
    source_url: String,
}

enum Grounding {
    Direct,    // Exact quote supports claim
    Inferred,  // Reasonable inference from source
    Assumed,   // No direct evidence (WARNING: often hallucination)
}

// Convert internal extraction to public API (Gemini Round 2)
impl InternalExtraction {
    fn to_extraction(&self, strict_mode: bool) -> Extraction {
        // In strict mode, discard "Assumed" claims (often hallucinations)
        let filtered_claims: Vec<_> = if strict_mode {
            self.claims.iter()
                .filter(|c| !matches!(c.grounding, Grounding::Assumed))
                .collect()
        } else {
            self.claims.iter().collect()
        };

        // Detect conflicts (same topic, different values from different sources)
        let conflicts = self.detect_conflicts(&filtered_claims);

        // Check if any claims are inferred (not Assumed, but not Direct either)
        let has_inference = filtered_claims.iter()
            .any(|c| matches!(c.grounding, Grounding::Inferred));

        // Calculate grounding grade (replaces arbitrary confidence float)
        let grounding = Extraction::calculate_grounding(
            &self.sources,
            &conflicts,
            has_inference,
        );

        Extraction {
            content: self.content.clone(),
            sources: self.sources.clone(),
            gaps: self.gaps.iter().map(|g| g.field.clone()).collect(),
            grounding,
            conflicts,
        }
    }

    fn detect_conflicts(&self, claims: &[&Claim]) -> Vec<Conflict> {
        // Group claims by topic, flag when sources disagree
        // Implementation: compare claims with same topic from different sources
        todo!("Conflict detection logic")
    }
}

/// Gaps are machine-readable queries (Gemini "Last Mile" tip)
/// Instead of: gaps: ["email"]
/// Use:        gaps: ["the contact email for the volunteer coordinator"]
/// This allows direct piping to search_for_gap() without reformulation
struct Gap {
    /// Human-readable field name
    field: String,
    /// Machine-readable search query - pipe directly to search_for_gap()
    query: String,
    /// URLs already searched (avoid re-searching)
    searched: Vec<String>,
}

struct Partition {
    title: String,
    pages: Vec<String>,  // URLs
    rationale: String,   // Why these pages grouped
}
```

### Prompts

```rust
// prompts/mod.rs
pub mod prompts {
    pub const SUMMARIZE: &str = r#"
Summarize this webpage for information retrieval.

Your summary must capture:
1. What the page offers (services, programs, opportunities)
2. What the page asks for (volunteers, donations, applications)
3. Calls to action (sign up, apply, contact, donate)
4. Key entities (organization names, locations, dates, contacts)

Output JSON:
{
    "summary": "2-3 sentence overview",
    "signals": {
        "offers": ["list of things offered"],
        "asks": ["list of things requested"],
        "calls_to_action": ["list of CTAs"],
        "entities": ["key proper nouns"]
    },
    "language": "detected language code"
}
"#;

    pub const PARTITION: &str = r#"
Given a query and page summaries, identify distinct items to extract.

Query: {query}

For this query, determine:
1. What constitutes ONE distinct item?
2. Which pages contribute to each item?
3. Why are these pages grouped together?

Output JSON array:
[
    {
        "title": "Brief item title",
        "pages": ["url1", "url2"],
        "rationale": "Why these pages are grouped"
    }
]
"#;

    pub const EXTRACT: &str = r#"
Extract information about: {query}

From these pages:
{pages}

Rules:
1. For EVERY claim, quote the source text
2. Note which page each quote comes from
3. Mark claims as Direct (exact quote), Inferred, or Assumed
4. Explicitly note what information is MISSING

Output markdown content followed by structured metadata.
"#;
}
```

## Implementation Phases

### Phase 1: Core Library Structure
**Goal:** Compilable crate with traits and types

- [ ] Create `packages/extraction/Cargo.toml`
  ```toml
  [package]
  name = "extraction"
  version = "0.1.0"
  edition = "2021"

  [dependencies]
  # Error handling (thiserror for libraries, not anyhow)
  thiserror = "2.0"

  # Async
  async-trait = "0.1"
  tokio = { version = "1", features = ["full"] }
  tokio-util = "0.7"  # For CancellationToken
  futures = "0.3"
  async-stream = "0.3"  # For streaming results

  # Serialization
  serde = { version = "1.0", features = ["derive"] }
  serde_json = "1.0"
  chrono = { version = "0.4", features = ["serde"] }

  # Utilities
  sha2 = "0.10"
  tracing = "0.1"
  uuid = { version = "1", features = ["v7", "serde"] }
  url = "2.5"

  # Security
  secrecy = "0.10"  # For API key handling
  ipnet = "2.9"  # For CIDR blocking

  # Rate limiting
  governor = "0.8"

  # Optional: hybrid search
  tantivy = { version = "0.22", optional = true }

  [dev-dependencies]
  tokio-test = "0.4"
  mockall = "0.13"  # For mock generation

  [features]
  default = []
  hybrid-search = ["tantivy"]
  ```
- [ ] Define typed errors in `error.rs` (use thiserror)
- [ ] Define `AI` trait in `traits/ai.rs`
- [ ] Define split storage traits: `PageCache`, `SummaryCache`, `EmbeddingStore`
- [ ] Define `Crawler` trait in `traits/crawler.rs` with `UrlValidator`
- [ ] Define core types: `Extraction`, `Source`, `CachedPage`, `Summary`
- [ ] Implement `MemoryStore` for testing
- [ ] Create mock `AI` implementation for tests

### Phase 2: Summarization & Indexing
**Goal:** Build site index from crawled pages

- [ ] Implement `summarizer.rs` - recall-optimized summarization
- [ ] Implement `SiteIndex::build()` - orchestrates crawl + summarize
- [ ] Implement content hashing for cache invalidation
- [ ] Implement prompt hashing for summary staleness detection (Gemini refinement)
- [ ] Implement embedding generation and storage
- [ ] Add tests for summarization with mock AI

### Phase 3: Extraction Pipeline (Strategy-Aware)
**Goal:** Strategy orchestrator + recall + extract with conflict detection

- [ ] Implement `strategy.rs` - query classification (Collection/Singular/Narrative)
- [ ] Implement `recall.rs` - hybrid recall (semantic on summaries + BM25 on raw)
- [ ] Implement `partition.rs` - bucket grouping for Collection strategy
- [ ] Implement `extract.rs` - evidence-grounded extraction with conflict detection
- [ ] Implement `conflicts.rs` - detect contradictions across sources
- [ ] Implement `grounding.rs` - GroundingGrade calculation
- [ ] Implement `SiteIndex::extract()` - strategy-aware orchestration
- [ ] Implement `SiteIndex::extract_stream()` - streaming results
- [ ] Add agent-native primitives: `search()`, `read()`, `extract_from()`, `search_for_gap()`
- [ ] Add cancellation support via `CancellationToken`
- [ ] Add comprehensive pipeline tests

### Phase 4: Crawler Implementations
**Goal:** Pluggable crawl strategies with security + query-driven discovery

- [ ] Implement `UrlValidator` with SSRF protection (CRITICAL)
- [ ] Implement `HttpCrawler` - direct HTTP with link following
- [ ] Implement `TavilyCrawler` - search-based discovery
- [ ] Implement `InformedCrawler` - query-driven crawling (Gemini R2)
- [ ] Implement `RateLimitedCrawler<C>` wrapper using governor
- [ ] Add robots.txt support
- [ ] Make crawler configurable via `CrawlConfig`
- [ ] Add deep-crawl-on-gaps pattern for refinement

### Phase 5: Storage Implementations
**Goal:** Production-ready storage backends

- [ ] Implement `SqliteStore` for local/CLI use
- [ ] Create migrations for SQLite schema
- [ ] Add staleness checking and cache invalidation
- [ ] Add tests for store implementations

### Phase 6: Integration with Server
**Goal:** Replace existing extraction code

- [ ] Create `AppPageStore` wrapping existing `page_snapshots` table
- [ ] Create `AppAI` wrapping existing `BaseAI` trait
- [ ] Wire extraction library into crawling domain
- [ ] Migrate post extraction to use new pipeline
- [ ] Remove old `agentic_extraction.rs` code
- [ ] Update tests to use new extraction

## LLM Prompt Strategy

### Summarization Prompt
Forces extraction of recall-optimized signals:
- **Offers**: Things the page provides (services, programs)
- **Asks**: Things the page requests (volunteers, donations)
- **CTAs**: Calls to action (sign up, apply, contact)
- **Entities**: Key proper nouns for entity matching

### Partition Prompt
Forces explicit reasoning about grouping:
- What constitutes ONE item for this query?
- Why are these pages grouped together?
- What distinguishes this item from others?

### Extract Prompt
Forces evidence grounding:
- Quote source text for every claim
- Cite which page each quote comes from
- Mark confidence level (Direct/Inferred/Assumed)
- Explicitly list what's missing

## Open Questions

1. **Firecrawl Integration** - Add as third crawler option? Requires API key.
2. ~~**Rate Limiting** - Library enforces or caller configures?~~ **RESOLVED:** Library provides `RateLimitedCrawler` wrapper, caller configures rate.
3. **JavaScript Rendering** - HttpCrawler limitation. Use Firecrawl for JS-heavy sites?
4. **Sitemap Discovery** - Auto-discover from robots.txt/sitemap.xml?
5. ~~**Embedding Dimensions** - Fix at 1536 or make configurable?~~ **RESOLVED:** Use 1024 dims (Cohere embed-v4 compatible), configurable via trait.
6. ~~**Streaming** - Should `extract()` stream results as buckets complete?~~ **RESOLVED:** Yes, provide `extract_stream()` for agent integration.

**New Questions from Research:**

7. **Hybrid Search** - Include BM25 + semantic, or just semantic? (Recommendation: hybrid with RRF)
8. **Cancellation** - Expose `CancellationToken` on all async methods? (Recommendation: yes, via optional parameter)
9. **Trait Granularity** - Single `PageStore` or split into `PageCache + SummaryCache + EmbeddingStore`? (Recommendation: split for flexibility)

**Questions Resolved by Gemini Review:**

10. ~~**Large Site Pagination** - What happens when site has 100+ pages?~~ **RESOLVED:** Implement "Ranked Recall" - embedding search for top 50 summaries before LLM partitioning
11. ~~**"Assumed" Evidence** - How to handle claims without source evidence?~~ **RESOLVED:** Add `strict_mode` (default: true) - discard Assumed claims, only keep Direct/Inferred
12. ~~**Summary Cache Invalidation** - How to detect stale summaries when prompt changes?~~ **RESOLVED:** Add `prompt_hash` to Summary struct, re-summarize when hash differs

## Success Criteria

- [ ] Library compiles independently (`cargo build -p extraction`)
- [ ] No dependencies on server crate
- [ ] Can extract from any website with natural language query
- [ ] Extractions include source attribution with roles
- [ ] Confidence scores derived from source count/quality
- [ ] Caching prevents redundant crawls/summaries (content hash)
- [ ] Tests pass with mock AI and in-memory store
- [ ] Server integration works with existing tables
- [ ] Old extraction code removed after migration
- [ ] **Security:** SSRF protection validates all URLs before crawling
- [ ] **Security:** API keys never appear in logs or errors
- [ ] **Performance:** Batch LLM calls (summarization, recall+partition)
- [ ] **Agent-Native:** Primitives exposed (search, read, extract_from)
- [ ] **Agent-Native:** Streaming via `extract_stream()`
- [ ] **Gemini R1:** Ranked Recall limits summaries to 50 for LLM partitioning
- [ ] **Gemini R1:** Strict mode discards "Assumed" claims by default
- [ ] **Gemini R1:** Summary cache invalidates when prompt_hash changes
- [ ] **Gemini R2:** Strategy Orchestrator classifies query type (Collection/Singular/Narrative)
- [ ] **Gemini R2:** Conflict detection exposes contradictions across sources
- [ ] **Gemini R2:** GroundingGrade enum replaces arbitrary confidence float
- [ ] **Gemini R2:** Informed Crawling uses query to discover deep-linked pages
- [ ] **Gemini R2:** Hybrid Recall uses BM25 on raw text as safety net
- [ ] **Gemini R2:** Refine primitive enables agent-driven gap-filling

## Migration Path

```
Current State:
┌─────────────────────────────────────────┐
│ posts/effects/agentic_extraction.rs     │ 1620 lines
│ posts/effects/llm_sync.rs               │  867 lines
│ crawling/actions/*.rs                   │ 1200 lines
└─────────────────────────────────────────┘

Target State:
┌─────────────────────────────────────────┐
│ packages/extraction/ (new library)      │
│   - Domain-agnostic                     │
│   - Reusable across projects            │
│   - Clean trait boundaries              │
└─────────────────────────────────────────┘
           │
           ▼ implements traits
┌─────────────────────────────────────────┐
│ packages/server/                        │
│   - AppPageStore (wraps page_snapshots) │
│   - AppAI (wraps BaseAI)                │
│   - Domain mapping (Extraction → Post)  │
└─────────────────────────────────────────┘
```

## Example Usage After Implementation

```rust
// In server code
use extraction::{Scraper, MemoryStore, HttpCrawler};

// Create scraper with app's storage and AI
let store = AppPageStore::new(&db_pool);
let ai = AppAI::new(deps.ai.clone(), deps.embedding_service.clone());
let crawler = HttpCrawler::new();

let scraper = Scraper::new(store, ai, crawler);

// Index a site
let site = scraper.index("https://nonprofit.org").await?;

// Extract with natural language
let extractions = index.extract("volunteer opportunities, services, events").await?;

// Map to domain types (app's responsibility)
for extraction in extractions {
    let post = Post::from_extraction(&extraction, &ai).await?;
    post.save(&db_pool).await?;
}
```

## Testing Strategy

**From pattern-recognition-specialist and Rust best practices:**

### Unit Tests (per module)

```rust
// Use mockall for trait mocking
#[cfg(test)]
mod tests {
    use super::*;
    use mockall::predicate::*;

    mock! {
        pub AI {}
        #[async_trait]
        impl AI for AI {
            async fn summarize(&self, content: &str, url: &str) -> Result<SummaryResponse>;
            async fn extract(&self, query: &str, pages: &[CachedPage]) -> Result<Extraction>;
        }
    }

    #[tokio::test]
    async fn test_recall_partition_groups_related_pages() {
        let mut mock_ai = MockAI::new();
        mock_ai.expect_recall_and_partition()
            .returning(|_, _| Ok(vec![
                Partition { title: "Item 1".into(), urls: vec!["page1".into()] },
            ]));

        let result = recall_and_partition("query", &summaries, &mock_ai).await?;
        assert_eq!(result.len(), 1);
    }
}
```

### Integration Tests (with real components)

```rust
#[tokio::test]
async fn test_full_extraction_pipeline() {
    let store = MemoryStore::new();
    let ai = TestAI::new();  // Deterministic responses
    let crawler = MockCrawler::with_pages(vec![...]);

    let scraper = Scraper::new(store, ai, crawler);
    let site = scraper.index("https://test.example").await?;
    let extractions = index.extract("find products").await?;

    assert!(!extractions.is_empty());
    assert!(extractions[0].confidence > 0.5);
}
```

### Security Tests (critical)

```rust
#[tokio::test]
async fn test_ssrf_protection_blocks_internal_urls() {
    let validator = UrlValidator::new();

    // Should block
    assert!(validator.validate("http://localhost/").is_err());
    assert!(validator.validate("http://127.0.0.1/").is_err());
    assert!(validator.validate("http://169.254.169.254/").is_err());  // AWS metadata
    assert!(validator.validate("http://10.0.0.1/internal").is_err());

    // Should allow
    assert!(validator.validate("https://example.com/").is_ok());
}

#[tokio::test]
async fn test_credentials_not_in_debug_output() {
    let config = AIConfig {
        api_key: Secret::new("sk-secret-key".into()),
        model: "gpt-4".into(),
    };

    let debug = format!("{:?}", config);
    assert!(!debug.contains("sk-secret"));
    assert!(debug.contains("[REDACTED]"));
}
```

### Property-Based Tests (optional, for robustness)

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn grounding_grade_is_valid(sources in 0usize..100, conflicts in 0usize..10) {
        let grounding = Extraction::calculate_grounding(
            &(0..sources).map(|_| mock_source()).collect::<Vec<_>>(),
            &(0..conflicts).map(|_| mock_conflict()).collect::<Vec<_>>(),
            false,
        );

        // Grounding must be one of the valid variants
        prop_assert!(matches!(
            grounding,
            GroundingGrade::Verified |
            GroundingGrade::SingleSource |
            GroundingGrade::Conflicted |
            GroundingGrade::Inferred
        ));

        // If conflicts exist, must be Conflicted
        if conflicts > 0 {
            prop_assert!(matches!(grounding, GroundingGrade::Conflicted));
        }
    }
}
```
