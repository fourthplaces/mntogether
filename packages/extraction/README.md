# Extraction Library

A domain-agnostic, query-driven extraction library for crawling websites and extracting structured information using LLMs.

## Philosophy

**"Let LLMs be LLMs"**

| Principle | Description |
|-----------|-------------|
| **Query-driven** | Natural language questions, not rigid schemas |
| **Domain-agnostic** | Signal types are user-defined strings |
| **Evidence-grounded** | Every claim cites sources; inference is flagged |
| **Mechanism vs Policy** | Library provides mechanics, caller controls behavior |

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                       INDEX (Main API)                       │
│  - extract()           Full pipeline with auto-strategy      │
│  - search()            Semantic search primitive             │
│  - read()              Fetch specific pages                  │
│  - extract_from()      Extract from selected pages           │
│  - plan_investigation() / execute_step()  Gap resolution     │
└───────────────────────────────────────────────────────────────┘
        │
        ├─────────────┬─────────────┬─────────────┐
        ▼             ▼             ▼             ▼
   ┌─────────┐  ┌──────────┐  ┌───────────┐  ┌───────────┐
   │ INGEST  │  │ STRATEGY │  │ DETECTIVE │  │  STORAGE  │
   │         │  │          │  │           │  │           │
   │ Crawl   │  │Collection│  │ Gap Plan  │  │ PageCache │
   │ Fetch   │  │Singular  │  │ Gap Exec  │  │ Summaries │
   │ Store   │  │Narrative │  │           │  │ Embeddings│
   └─────────┘  └──────────┘  └───────────┘  └───────────┘
```

### Data Flow

```
INGEST → STORE → SUMMARIZE → EMBED → EXTRACT

1. Ingestor discovers/fetches pages → RawPage
2. Pages stored in PageCache → CachedPage (with content hash)
3. AI summarizes each page → Summary (with RecallSignals)
4. AI embeds summaries → Vec<f32> stored in EmbeddingStore
5. Query triggers: Recall → Partition → Extract → Extraction
```

## Quick Start

```rust
use extraction::{Index, MemoryStore, QueryFilter};
use extraction::ai::OpenAI;
use extraction::ingestors::{HttpIngestor, ValidatedIngestor, DiscoverConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup
    let store = MemoryStore::new();
    let ai = OpenAI::from_env()?;
    let index = Index::new(store, ai);

    // Ingest a website
    let ingestor = ValidatedIngestor::new(HttpIngestor::new());
    let config = DiscoverConfig::new("https://redcross.org").with_limit(50);
    let result = index.ingest(&config, &ingestor).await?;
    println!("Ingested {} pages", result.pages_summarized);

    // Extract from all indexed sites
    let results = index.extract("volunteer opportunities", None).await?;

    // Or filter to one site
    let filter = QueryFilter::for_site("redcross.org");
    let results = index.extract("volunteer opportunities", Some(filter)).await?;

    for extraction in results {
        println!("Content: {}", extraction.content);
        println!("Grounding: {:?}", extraction.grounding);
        println!("Gaps: {}", extraction.gaps.len());
    }

    Ok(())
}
```

## API Reference

### High-Level API (Auto Strategy)

```rust
// Full extraction - auto-selects Collection/Singular/Narrative strategy
let extractions = index.extract("find all volunteer opportunities", None).await?;

// With site filtering
let filter = QueryFilter::for_site("redcross.org");
let extractions = index.extract("find volunteer roles", Some(filter)).await?;

// Streaming extraction (yields results as each bucket completes)
let mut stream = index.extract_stream("find all events", None);
while let Some(result) = stream.next().await {
    println!("Bucket: {}", result?.content);
}

// With cancellation token
let cancel = CancellationToken::new();
let extractions = index.extract_with_cancel("query", None, cancel).await?;
```

### Agent-Native Primitives

For agents that want fine-grained control:

```rust
// Search the index (semantic + keyword hybrid)
let results = index.search("volunteer coordinator", 20, None).await?;

// Read specific pages by URL
let pages = index.read(&["https://example.com/about", "https://example.com/contact"]).await?;

// Extract from pages YOU select (skip recall phase)
let extraction = index.extract_from("contact information", &pages).await?;

// Keyword-heavy search for specific entities
let results = index.search_for_gap("john.doe@example.com", 10, None).await?;
```

### Ingestion API

```rust
// Ingest with discovery (crawl a site)
let ingestor = ValidatedIngestor::new(HttpIngestor::new());
let config = DiscoverConfig::new("https://example.com")
    .with_limit(100)
    .with_max_depth(3)
    .include("*/blog/*")
    .exclude("*/admin/*");
let result = index.ingest(&config, &ingestor).await?;

// Ingest specific URLs (for gap-filling)
let urls = vec!["https://example.com/team".to_string()];
let result = index.ingest_urls(&urls, &ingestor).await?;

// With custom config
let ingest_config = IngestorConfig {
    concurrency: 10,
    batch_size: 5,
    skip_cached: true,
    force_resummarize: false,
};
let result = index.ingest_with_config(&config, &ingest_config, &ingestor).await?;
```

### Detective Engine (Gap Resolution)

```rust
let mut extraction = index.extract("board members", None).await?.remove(0);

// Library provides intelligence (plan)
// Caller provides will (loop, budget, retries)
let mut iterations = 0;
while extraction.has_gaps() && iterations < 3 {
    let plan = index.plan_investigation(&extraction);

    for step in &plan.steps {
        let result = index.execute_step(&step, None).await?;
        let pages = index.pages_from_step_result(&result).await?;

        if !pages.is_empty() {
            let supplement = index.extract_from("board members", &pages).await?;
            extraction.merge(supplement);
        }
    }
    iterations += 1;
}

println!("Final grounding: {:?}", extraction.grounding);
```

## Extraction Strategies

The library automatically selects a strategy based on query type:

| Strategy | Query Pattern | Behavior |
|----------|--------------|----------|
| **Collection** | "find all X", "list X" | Recall → Partition into buckets → Extract each |
| **Singular** | "what is the X?" | Recall top N → Extract single answer |
| **Narrative** | "describe X" | Recall top N → Aggregate narrative |

## Output Structure

### Extraction

```rust
pub struct Extraction {
    /// The extracted content as markdown
    pub content: String,

    /// Pages that contributed to this extraction
    pub sources: Vec<Source>,

    /// Machine-readable queries for missing info
    pub gaps: Vec<MissingField>,

    /// How well-grounded is this extraction?
    pub grounding: GroundingGrade,

    /// Contradictions detected across sources
    pub conflicts: Vec<Conflict>,

    /// Overall status
    pub status: ExtractionStatus,
}
```

### Grounding Grades

| Grade | Meaning | Recommended Action |
|-------|---------|-------------------|
| `Verified` | Multiple sources agree | High confidence |
| `SingleSource` | One source | May want verification |
| `Conflicted` | Sources disagree | Check `conflicts` field |
| `Inferred` | LLM inferred | Often hallucination |

### Extraction Status

| Status | Meaning |
|--------|---------|
| `Found` | Information was found |
| `Partial` | Some info found, gaps remain |
| `Missing` | No information found |
| `Contradictory` | Sources disagree |

### Gaps

```rust
pub struct MissingField {
    pub field: String,           // e.g., "contact email"
    pub query: GapQuery,         // Ready to pass to search()
    pub reason: MissingReason,   // NotMentioned, Ambiguous, OutOfScope
}
```

## Core Traits

### AI Trait

Implement for any LLM provider:

```rust
#[async_trait]
pub trait AI: Send + Sync {
    // Core operations
    async fn summarize(&self, content: &str, url: &str) -> Result<SummaryResponse>;
    async fn classify_query(&self, query: &str) -> Result<ExtractionStrategy>;
    async fn recall_and_partition(&self, query: &str, summaries: &[Summary]) -> Result<Vec<Partition>>;

    // Extraction strategies
    async fn extract(&self, query: &str, pages: &[CachedPage], hints: Option<&[String]>) -> Result<Extraction>;
    async fn extract_single(&self, query: &str, pages: &[CachedPage]) -> Result<Extraction>;
    async fn extract_narrative(&self, query: &str, pages: &[CachedPage]) -> Result<Extraction>;

    // Embeddings
    async fn embed(&self, text: &str) -> Result<Vec<f32>>;
    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>>;
}
```

### PageStore Trait

Composite storage interface:

```rust
pub trait PageStore: PageCache + SummaryCache + EmbeddingStore {}

// PageCache - Raw page content
pub trait PageCache: Send + Sync {
    async fn get_page(&self, url: &str) -> Result<Option<CachedPage>>;
    async fn store_page(&self, page: &CachedPage) -> Result<()>;
    async fn get_pages(&self, urls: &[&str]) -> Result<Vec<CachedPage>>;
    async fn get_pages_for_site(&self, site_url: &str) -> Result<Vec<CachedPage>>;
}

// SummaryCache - Recall-optimized summaries
pub trait SummaryCache: Send + Sync {
    async fn get_summary(&self, url: &str, content_hash: &str) -> Result<Option<Summary>>;
    async fn store_summary(&self, summary: &Summary) -> Result<()>;
    async fn get_summaries_for_site(&self, site_url: &str) -> Result<Vec<Summary>>;
}

// EmbeddingStore - Vector search
pub trait EmbeddingStore: Send + Sync {
    async fn store_embedding(&self, url: &str, embedding: &[f32]) -> Result<()>;
    async fn search_similar(&self, embedding: &[f32], limit: usize, filter: Option<&QueryFilter>) -> Result<Vec<PageRef>>;
}
```

### Ingestor Trait

Pluggable content fetching:

```rust
pub trait Ingestor: Send + Sync {
    async fn discover(&self, config: &DiscoverConfig) -> CrawlResult<Vec<RawPage>>;
    async fn fetch_specific(&self, urls: &[String]) -> CrawlResult<Vec<RawPage>>;
    async fn fetch_one(&self, url: &str) -> CrawlResult<RawPage>;
    fn name(&self) -> &str;
}
```

**Available Implementations:**
- `HttpIngestor` - Basic HTTP crawling with link following
- `FirecrawlIngestor` - Firecrawl API (JS rendering, anti-bot)
- `ValidatedIngestor<I>` - Wraps any ingestor with SSRF protection
- `MockIngestor` - For testing

## Configuration

### ExtractionConfig

```rust
let config = ExtractionConfig {
    max_summaries_for_partition: 50,  // Max summaries to LLM for partitioning
    strict_mode: true,                 // Discard "Inferred" claims
    output_language: None,             // Translate output (e.g., Some("Spanish"))
    hints: vec![],                     // Extraction hints (e.g., ["title", "date"])
    detect_conflicts: true,            // Enable conflict detection
    hybrid_recall: true,               // Semantic + BM25 search
    semantic_weight: 0.6,              // Weight in hybrid (0.0-1.0)
};

let index = Index::with_config(store, ai, config);
```

### DiscoverConfig

```rust
let config = DiscoverConfig::new("https://example.com")
    .with_limit(100)              // Max pages
    .with_max_depth(3)            // Crawl depth
    .include("*/blog/*")          // Glob patterns to include
    .exclude("*/admin/*")         // Patterns to exclude
    .with_option("scrape_formats", "markdown");  // Source-specific options
```

### QueryFilter

```rust
// Single site
let filter = QueryFilter::for_site("redcross.org");

// Multiple sites
let filter = QueryFilter::for_sites(["redcross.org", "volunteer.org"]);

// Exclude sites
let filter = QueryFilter::excluding(["spam.org"]);

// Date range
let filter = QueryFilter::new()
    .with_min_date(Utc::now() - Duration::days(30));
```

## Storage Backends

| Backend | Feature Flag | Use Case |
|---------|--------------|----------|
| `MemoryStore` | (default) | Testing |
| `SqliteStore` | `sqlite` | Embedded |
| `PostgresStore` | `postgres` | Production (10M+ pages) |

### PostgreSQL Features

- **HNSW indexes** for fast vector search
- **Hybrid search** with Reciprocal Rank Fusion
- **Content hash** for cache invalidation
- **Prompt hash** for summary regeneration

```rust
let store = PostgresStore::new("postgres://localhost/extraction").await?;
```

## AI Implementations

### OpenAI (Reference)

```rust
use extraction::ai::OpenAI;

let ai = OpenAI::new("sk-...")
    .with_model("gpt-4o")
    .with_embedding_model("text-embedding-3-small");
```

### Bring Your Own

```rust
#[async_trait]
impl AI for MyAI {
    async fn summarize(&self, content: &str, url: &str) -> Result<SummaryResponse> {
        // Your implementation
    }
    async fn extract(&self, query: &str, pages: &[CachedPage], hints: Option<&[String]>) -> Result<Extraction> {
        // Your implementation
    }
    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        // Your implementation
    }
    // ...
}
```

## Error Handling

```rust
pub enum ExtractionError {
    Crawl(CrawlError),
    AI(Box<dyn Error>),
    PageNotFound { url: String },
    Storage(Box<dyn Error>),
    Cancelled,
    InvalidQuery { reason: String },
}

pub enum CrawlError {
    Security(SecurityError),       // SSRF protection
    Http(Box<dyn Error>),
    RateLimitExceeded,
    InvalidUrl { url: String },
    RobotsDisallowed { url: String },
    Timeout { url: String },
}
```

## Installation

```toml
[dependencies]
extraction = { path = "packages/extraction" }

# For PostgreSQL support
extraction = { path = "packages/extraction", features = ["postgres"] }

# For OpenAI reference implementation
extraction = { path = "packages/extraction", features = ["openai"] }
```

## Testing

```bash
# Unit tests
cargo test -p extraction --lib

# Integration tests
cargo test -p extraction --test detective_integration

# With PostgreSQL
cargo test -p extraction --features postgres
```

## License

MIT
