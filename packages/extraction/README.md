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

## Quick Start

```rust
use extraction::{Index, MemoryStore, QueryFilter};
use extraction::ai::OpenAI;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup
    let store = MemoryStore::new();
    let ai = OpenAI::from_env()?;
    let index = Index::new(store, ai);

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

## Features

### Extraction Strategies

The library automatically selects a strategy based on query type:

| Strategy | Query Type | Example |
|----------|------------|---------|
| **Collection** | "Find all X" | "List volunteer opportunities" |
| **Singular** | Point lookup | "What is their phone number?" |
| **Narrative** | Aggregation | "Describe what this org does" |

### Grounding Grades

Every extraction includes a grounding quality indicator:

| Grade | Meaning | Recommended Action |
|-------|---------|-------------------|
| `Verified` | Multiple sources agree | High confidence |
| `SingleSource` | One source | May want verification |
| `Conflicted` | Sources disagree | Check `conflicts` field |
| `Inferred` | LLM inferred | ⚠️ Often hallucination |

### Detective Engine

Auto-resolve gaps in extractions with the investigation API:

```rust
let mut extraction = index.extract("board members", None).await?.remove(0);

// Library provides intelligence (plan)
// Caller provides will (loop, budget, retries)
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

### Domain-Agnostic Signals

Signals are user-defined strings, not hardcoded enums:

```rust
// E-commerce
ExtractedSignal::new("product", "iPhone 15 Pro")
    .with_subtype("electronics")
    .with_confidence(0.95);

// Real estate
ExtractedSignal::new("listing", "3BR apartment")
    .with_subtype("rental");

// Job board
ExtractedSignal::new("requirement", "5+ years Python");
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
- **Normalized signals** table (not JSONB)
- **Investigation tracking** for debugging

```rust
let store = PostgresStore::new("postgres://localhost/extraction").await?;
store.run_detective_migrations().await?;
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

Implement the `AI` trait for any LLM:

```rust
#[async_trait]
impl AI for MyAI {
    async fn summarize(&self, content: &str, url: &str) -> Result<SummaryResponse>;
    async fn extract(&self, query: &str, pages: &[CachedPage], hints: Option<&[String]>) -> Result<Extraction>;
    async fn embed(&self, text: &str) -> Result<Vec<f32>>;
    // ...
}
```

## Crawlers

### HTTP Crawler

```rust
let crawler = HttpCrawler::new()
    .rate_limited(10, Duration::from_secs(1));

let pages = crawler.crawl(&CrawlConfig::new("https://example.org")).await?;
```

### Informed Crawler

Query-driven discovery for finding deep pages:

```rust
let informed = InformedCrawler::new(http_crawler, search_service);
let pages = informed.crawl_for_query(&config, "board members").await?;
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

## Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                              Index                                   │
│  (Main entry point - flat index over all ingested pages)            │
├─────────────────────────────────────────────────────────────────────┤
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────────────────┐  │
│  │   Search    │    │   Extract   │    │   Detective Engine      │  │
│  │  (semantic  │    │  (Collection│    │  plan_investigation()   │  │
│  │   + hybrid) │    │   Singular  │    │  execute_step()         │  │
│  │             │    │   Narrative)│    │  merge()                │  │
│  └─────────────┘    └─────────────┘    └─────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────┘
                              │
          ┌───────────────────┼───────────────────┐
          ▼                   ▼                   ▼
   ┌─────────────┐    ┌─────────────┐    ┌─────────────┐
   │ MemoryStore │    │SqliteStore  │    │PostgresStore│
   └─────────────┘    └─────────────┘    └─────────────┘
```

## Testing

```bash
# Unit tests
cargo test --lib

# Integration tests
cargo test --test detective_integration

# With PostgreSQL
cargo test --features postgres
```

## License

MIT
