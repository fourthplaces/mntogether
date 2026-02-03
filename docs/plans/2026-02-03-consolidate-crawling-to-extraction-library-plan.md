# Plan: Consolidate Crawling Domain to Extraction Library

## Progress

| Phase | Status | Notes |
|-------|--------|-------|
| Phase 1: Add Ingestor Trait | ✅ Complete | Created `traits/ingestor.rs` with `Ingestor`, `RawPage`, `DiscoverConfig`, `ValidatedIngestor` |
| Phase 2: Add Ingestors | ✅ Complete | Created `ingestors/` module with `HttpIngestor`, `FirecrawlIngestor`, `MockIngestor` |
| Phase 3: Update Index | ✅ Complete | Added `Index.ingest()`, `ingest_with_config()`, `ingest_urls()` methods + `ingest_with_ingestor()`, `ingest_urls_with_ingestor()` functions |
| Phase 4: ExtractionService | ✅ Complete | Added `ingest()`, `ingest_with_config()`, `ingest_urls()`, `ingest_url()` methods; re-exported ingestor types in kernel |
| Phase 5: Simplify Server | ✅ Complete | Created `ingest_website.rs` action using ExtractionService; added `WebsiteIngested` event |
| Phase 6: Migrations | ✅ Complete | Added `last_synced_at` column to website_snapshots (migration 000096) |
| Phase 7: Cleanup | ✅ Complete | Dropped unused tables: schemas, detections, extractions, field_provenance, relationships (migration 000097) |

## Overview

Replace the server's crawling infrastructure with the extraction library's **pluggable Ingestor pattern**. The server becomes a thin orchestration layer while the extraction library owns all ingestion, storage, summarization, and embedding logic.

**Philosophy:**
- Server owns business logic (auth, workflows, domain models)
- Extraction library owns infrastructure (ingestion, storage, AI)
- **Index** = Retrieval + Reasoning (the brain)
- **Ingestor** = Pluggable data fetching (the field agents)

## The Ingestor Pattern

Separating ingestion from the core Index keeps the library domain-agnostic. One user might want a simple HTTP client, while another needs a headless browser for React-heavy sites.

### Core Trait

```rust
/// Pluggable data ingestion - the "field agents" that feed the Index.
#[async_trait]
pub trait Ingestor: Send + Sync {
    /// Discover and fetch pages from a root URL.
    /// Used for initial site crawling.
    async fn discover(&self, url: &str, limit: usize) -> Result<Vec<RawPage>>;

    /// Targeted fetch for specific URLs.
    /// Used by the Detective to follow GapQueries.
    async fn fetch_specific(&self, urls: &[String]) -> Result<Vec<RawPage>>;
}

/// Raw page content before processing.
pub struct RawPage {
    pub url: String,
    pub content: String,
    pub title: Option<String>,
    pub content_type: Option<String>,
    pub fetched_at: DateTime<Utc>,
}
```

### Implementation Strategy

| Ingestor | Use Case | Tech |
|----------|----------|------|
| `HttpIngestor` | Basic HTML sites, sitemaps | `reqwest` |
| `FirecrawlIngestor` | JS-heavy sites, managed crawling | Firecrawl API |
| `TavilyIngestor` | Search-based discovery | Tavily API |
| `PlaywrightIngestor` | Complex JS, infinite scroll | Playwright |
| `DocumentIngestor` | Local files, S3 buckets | Filesystem/S3 |

### Informed Ingestion

The magic happens when the **Index informs the Ingestor**:

```rust
impl<S: PageStore, A: AI> Index<S, A> {
    pub async fn ingest<I: Ingestor>(
        &self,
        url: &str,
        limit: usize,
        ingestor: &I,
    ) -> Result<IngestResult> {
        // 1. Ingestor discovers and fetches raw content
        let raw_pages = ingestor.discover(url, limit).await?;

        // 2. Index processes them (Summarize → Embed → Store)
        let mut page_urls = Vec::new();
        for page in raw_pages {
            let cached = self.process_and_store(page).await?;
            page_urls.push(cached.url);
        }

        Ok(IngestResult { page_urls, pages_processed: page_urls.len() })
    }

    /// Detective can request specific pages to fill gaps
    pub async fn fetch_for_gap<I: Ingestor>(
        &self,
        gap: &GapQuery,
        ingestor: &I,
    ) -> Result<Vec<CachedPage>> {
        let urls = gap.suggested_urls();
        let raw_pages = ingestor.fetch_specific(&urls).await?;
        // Process and return
    }
}
```

## Current State

### Server Tables (to be replaced)

| Table | Purpose | Replacement |
|-------|---------|-------------|
| `page_snapshots` | Crawled page content | `extraction_pages` |
| `page_summaries` | Cached AI summaries | `extraction_summaries` |
| `page_extractions` | AI-extracted content | `extraction_summaries` |
| `detections` | AI detection records | (delete - unused) |
| `extractions` | Schema-based extractions | (delete - unused) |
| `schemas` | External schema registry | (delete - unused) |
| `field_provenance` | Field tracing | (delete - unused) |
| `relationships` | Graph edges | (delete - unused) |

### Server Tables (to keep)

| Table | Purpose | Changes |
|-------|---------|---------|
| `websites` | Core domain model | None |
| `website_snapshots` | Junction: website ↔ pages | Redesign to reference URL |
| `posts` | Extracted announcements | None |
| `resources` | Extracted services | None (or delete if unused) |

### Extraction Library Tables (already exist)

| Table | Purpose |
|-------|---------|
| `extraction_pages` | Cached crawled pages |
| `extraction_summaries` | AI summaries with signals |
| `extraction_embeddings` | Vector embeddings |
| `extraction_signals` | Normalized signals |
| `extraction_jobs` | Job tracking |
| `extraction_gaps` | Gap tracking |
| `extraction_investigation_logs` | Audit logs |

## Architecture After Migration

```
┌─────────────────────────────────────────────────────────────────────┐
│                    SERVER (Thin Orchestration)                       │
│  - Authorization (Actor/Capability)                                  │
│  - Website domain (approval, status tracking)                        │
│  - Posts domain (business entity with workflow)                      │
│  - Event cascade (Seesaw effects)                                    │
│  - website_snapshots junction table                                  │
└─────────────────────────────────────────────────────────────────────┘
                                    │
                    Uses ExtractionService.ingest()
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────┐
│                    EXTRACTION LIBRARY (Infrastructure)               │
│                                                                      │
│  ┌─────────────────────────────────────────────────────────────┐    │
│  │                     Ingestor Trait                           │    │
│  │  ┌──────────────┐ ┌──────────────┐ ┌──────────────────────┐ │    │
│  │  │ Firecrawl    │ │ HTTP         │ │ Tavily / Playwright  │ │    │
│  │  │ Ingestor     │ │ Ingestor     │ │ / Document Ingestor  │ │    │
│  │  └──────┬───────┘ └──────┬───────┘ └──────────┬───────────┘ │    │
│  │         │                │                    │              │    │
│  │         └────────────────┼────────────────────┘              │    │
│  │                          │                                   │    │
│  │                          ▼                                   │    │
│  │              discover() / fetch_specific()                   │    │
│  └─────────────────────────────────────────────────────────────┘    │
│                                    │                                 │
│                                    ▼                                 │
│  ┌─────────────────────────────────────────────────────────────┐    │
│  │                        Index                                 │    │
│  │  - ingest(url, limit, ingestor)   → Summarize → Embed → Store│    │
│  │  - extract(query, filter)         → Recall → Partition → LLM │    │
│  │  - fetch_for_gap(gap, ingestor)   → Detective gap-filling    │    │
│  └─────────────────────────────────────────────────────────────┘    │
│                                    │                                 │
│                                    ▼                                 │
│  ┌─────────────────┐  ┌─────────────────┐  ┌──────────────────┐     │
│  │ AI Trait        │  │ PageStore Trait │  │ Tables           │     │
│  │ - OpenAI        │  │ - PostgresStore │  │ - extraction_*   │     │
│  └─────────────────┘  └─────────────────┘  └──────────────────┘     │
└─────────────────────────────────────────────────────────────────────┘
```

## Implementation Phases

### Phase 1: Add Ingestor Trait to Extraction Library

**File:** `packages/extraction/src/traits/ingestor.rs` (NEW)

```rust
//! Ingestor trait for pluggable data ingestion.
//!
//! Ingestors are "field agents" that fetch raw content for the Index to process.
//! This separation keeps the library domain-agnostic - swap HTTP for Playwright,
//! Firecrawl for local files, without touching core logic.

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::error::Result;

/// Raw page content before processing by the Index.
#[derive(Debug, Clone)]
pub struct RawPage {
    pub url: String,
    pub content: String,
    pub title: Option<String>,
    pub content_type: Option<String>,
    pub fetched_at: DateTime<Utc>,
    pub metadata: std::collections::HashMap<String, String>,
}

/// Pluggable data ingestion trait.
///
/// Implementations fetch raw content from various sources:
/// - `HttpIngestor` - Basic HTTP requests
/// - `FirecrawlIngestor` - Managed JS rendering
/// - `TavilyIngestor` - Search-based discovery
/// - `PlaywrightIngestor` - Browser automation
/// - `DocumentIngestor` - Local files, S3
#[async_trait]
pub trait Ingestor: Send + Sync {
    /// Discover and fetch pages starting from a root URL.
    ///
    /// Used for initial site crawling. The ingestor decides how to
    /// discover pages (sitemap, link following, search, etc.)
    async fn discover(&self, url: &str, limit: usize) -> Result<Vec<RawPage>>;

    /// Fetch specific URLs directly.
    ///
    /// Used by the Detective to follow GapQuery suggestions.
    /// Unlike discover(), this doesn't explore - just fetches.
    async fn fetch_specific(&self, urls: &[String]) -> Result<Vec<RawPage>>;
}

/// SSRF-safe wrapper around any Ingestor.
pub struct ValidatedIngestor<I: Ingestor> {
    inner: I,
    validator: crate::traits::crawler::UrlValidator,
}

impl<I: Ingestor> ValidatedIngestor<I> {
    pub fn new(ingestor: I) -> Self {
        Self {
            inner: ingestor,
            validator: crate::traits::crawler::UrlValidator::new(),
        }
    }
}

#[async_trait]
impl<I: Ingestor> Ingestor for ValidatedIngestor<I> {
    async fn discover(&self, url: &str, limit: usize) -> Result<Vec<RawPage>> {
        self.validator.validate_with_dns(url).await
            .map_err(|e| crate::error::ExtractionError::Security(e.to_string()))?;

        let pages = self.inner.discover(url, limit).await?;

        // Filter out any pages that redirect to blocked URLs
        Ok(pages.into_iter()
            .filter(|p| self.validator.validate(&p.url).is_ok())
            .collect())
    }

    async fn fetch_specific(&self, urls: &[String]) -> Result<Vec<RawPage>> {
        // Validate all URLs first
        for url in urls {
            self.validator.validate_with_dns(url).await
                .map_err(|e| crate::error::ExtractionError::Security(e.to_string()))?;
        }

        self.inner.fetch_specific(urls).await
    }
}
```

**File:** `packages/extraction/src/traits/mod.rs` - Add export:

```rust
pub mod ingestor;
pub use ingestor::{Ingestor, RawPage, ValidatedIngestor};
```

### Phase 2: Add FirecrawlIngestor

**File:** `packages/extraction/src/ingestors/firecrawl.rs` (NEW)

```rust
//! Firecrawl-based ingestor for production use.
//!
//! Handles JavaScript rendering, rate limiting, and proxy rotation
//! via the Firecrawl managed service.

use async_trait::async_trait;
use chrono::Utc;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::error::{ExtractionError, Result};
use crate::traits::ingestor::{Ingestor, RawPage};

/// Firecrawl ingestor for JS-heavy sites.
pub struct FirecrawlIngestor {
    api_key: String,
    client: Client,
    base_url: String,
}

impl FirecrawlIngestor {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            client: Client::new(),
            base_url: "https://api.firecrawl.dev/v1".to_string(),
        }
    }

    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }
}

#[async_trait]
impl Ingestor for FirecrawlIngestor {
    async fn discover(&self, url: &str, limit: usize) -> Result<Vec<RawPage>> {
        #[derive(Serialize)]
        struct CrawlRequest {
            url: String,
            limit: usize,
            #[serde(rename = "scrapeOptions")]
            scrape_options: ScrapeOptions,
        }

        #[derive(Serialize)]
        struct ScrapeOptions {
            formats: Vec<String>,
        }

        let request = CrawlRequest {
            url: url.to_string(),
            limit,
            scrape_options: ScrapeOptions {
                formats: vec!["markdown".to_string()],
            },
        };

        let response = self.client
            .post(format!("{}/crawl", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request)
            .send()
            .await
            .map_err(|e| ExtractionError::Crawl(e.to_string().into()))?;

        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            return Err(ExtractionError::Crawl(format!("Firecrawl error: {}", error).into()));
        }

        #[derive(Deserialize)]
        struct CrawlResponse {
            data: Vec<PageData>,
        }

        #[derive(Deserialize)]
        struct PageData {
            url: String,
            markdown: Option<String>,
            title: Option<String>,
        }

        let crawl_response: CrawlResponse = response.json().await
            .map_err(|e| ExtractionError::Crawl(e.to_string().into()))?;

        Ok(crawl_response.data.into_iter().map(|p| RawPage {
            url: p.url,
            content: p.markdown.unwrap_or_default(),
            title: p.title,
            content_type: Some("text/markdown".to_string()),
            fetched_at: Utc::now(),
            metadata: std::collections::HashMap::new(),
        }).collect())
    }

    async fn fetch_specific(&self, urls: &[String]) -> Result<Vec<RawPage>> {
        let mut pages = Vec::with_capacity(urls.len());

        for url in urls {
            #[derive(Serialize)]
            struct ScrapeRequest {
                url: String,
                formats: Vec<String>,
            }

            let request = ScrapeRequest {
                url: url.clone(),
                formats: vec!["markdown".to_string()],
            };

            let response = self.client
                .post(format!("{}/scrape", self.base_url))
                .header("Authorization", format!("Bearer {}", self.api_key))
                .json(&request)
                .send()
                .await;

            match response {
                Ok(resp) if resp.status().is_success() => {
                    #[derive(Deserialize)]
                    struct ScrapeResponse {
                        data: PageData,
                    }

                    #[derive(Deserialize)]
                    struct PageData {
                        markdown: Option<String>,
                        title: Option<String>,
                    }

                    if let Ok(scrape) = resp.json::<ScrapeResponse>().await {
                        pages.push(RawPage {
                            url: url.clone(),
                            content: scrape.data.markdown.unwrap_or_default(),
                            title: scrape.data.title,
                            content_type: Some("text/markdown".to_string()),
                            fetched_at: Utc::now(),
                            metadata: std::collections::HashMap::new(),
                        });
                    }
                }
                _ => {
                    tracing::warn!("Failed to fetch {}", url);
                }
            }
        }

        Ok(pages)
    }
}
```

**File:** `packages/extraction/src/ingestors/mod.rs` (NEW)

```rust
//! Ingestor implementations.

mod firecrawl;
mod http;

pub use firecrawl::FirecrawlIngestor;
pub use http::HttpIngestor;
```

**File:** `packages/extraction/src/ingestors/http.rs` (NEW)

```rust
//! Simple HTTP ingestor for basic HTML sites.

use async_trait::async_trait;
use chrono::Utc;
use reqwest::Client;

use crate::error::{ExtractionError, Result};
use crate::traits::ingestor::{Ingestor, RawPage};

/// Basic HTTP ingestor using reqwest.
pub struct HttpIngestor {
    client: Client,
}

impl Default for HttpIngestor {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpIngestor {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .user_agent("extraction-bot/1.0")
                .build()
                .unwrap_or_default(),
        }
    }
}

#[async_trait]
impl Ingestor for HttpIngestor {
    async fn discover(&self, url: &str, limit: usize) -> Result<Vec<RawPage>> {
        // Simple single-page fetch for now
        // TODO: Add sitemap parsing and link following
        let pages = self.fetch_specific(&[url.to_string()]).await?;
        Ok(pages.into_iter().take(limit).collect())
    }

    async fn fetch_specific(&self, urls: &[String]) -> Result<Vec<RawPage>> {
        let mut pages = Vec::with_capacity(urls.len());

        for url in urls {
            let response = self.client.get(url).send().await
                .map_err(|e| ExtractionError::Crawl(e.to_string().into()))?;

            if response.status().is_success() {
                let content_type = response.headers()
                    .get("content-type")
                    .and_then(|v| v.to_str().ok())
                    .map(String::from);

                let content = response.text().await
                    .map_err(|e| ExtractionError::Crawl(e.to_string().into()))?;

                pages.push(RawPage {
                    url: url.clone(),
                    content,
                    title: None, // Would need HTML parsing
                    content_type,
                    fetched_at: Utc::now(),
                    metadata: std::collections::HashMap::new(),
                });
            }
        }

        Ok(pages)
    }
}
```

### Phase 3: Update Index to Use Ingestor

**File:** `packages/extraction/src/pipeline/index.rs` - Add methods:

```rust
impl<S: PageStore + SummaryCache + EmbeddingStore, A: AI> Index<S, A> {
    /// Ingest pages from a URL using the provided ingestor.
    ///
    /// This is the main entry point for adding content to the index.
    /// The ingestor discovers pages, then Index processes them:
    /// 1. Convert RawPage → CachedPage
    /// 2. Summarize content
    /// 3. Generate embeddings
    /// 4. Store everything
    pub async fn ingest<I: Ingestor>(
        &self,
        url: &str,
        limit: usize,
        ingestor: &I,
    ) -> Result<IngestResult> {
        // 1. Discover and fetch raw pages
        let raw_pages = ingestor.discover(url, limit).await?;

        // 2. Process each page
        let mut page_urls = Vec::with_capacity(raw_pages.len());
        for raw in raw_pages {
            // Convert to CachedPage format
            let cached = CachedPage {
                url: raw.url.clone(),
                site_url: extract_site_url(&raw.url),
                content: raw.content,
                content_hash: hash_content(&raw.content),
                fetched_at: raw.fetched_at,
                title: raw.title,
                http_headers: std::collections::HashMap::new(),
                metadata: raw.metadata,
            };

            // Store page
            self.store.store_page(&cached).await?;

            // Summarize
            let summary_response = self.ai.summarize(&cached.content, &cached.url).await?;
            let summary = Summary {
                url: cached.url.clone(),
                site_url: cached.site_url.clone(),
                text: summary_response.summary,
                signals: summary_response.signals,
                language: summary_response.language,
                created_at: Utc::now(),
                prompt_hash: "v1".to_string(), // TODO: Version prompts
                content_hash: cached.content_hash.clone(),
                embedding: None,
            };
            self.store.store_summary(&summary).await?;

            // Embed
            let embedding = self.ai.embed(&summary.text).await?;
            self.store.store_embedding(&cached.url, &embedding).await?;

            page_urls.push(cached.url);
        }

        Ok(IngestResult {
            page_urls,
            pages_processed: page_urls.len(),
        })
    }

    /// Fetch specific pages for gap-filling (Detective mode).
    pub async fn fetch_for_gap<I: Ingestor>(
        &self,
        urls: &[String],
        ingestor: &I,
    ) -> Result<Vec<CachedPage>> {
        let raw_pages = ingestor.fetch_specific(urls).await?;

        let mut cached_pages = Vec::with_capacity(raw_pages.len());
        for raw in raw_pages {
            let cached = CachedPage {
                url: raw.url.clone(),
                site_url: extract_site_url(&raw.url),
                content: raw.content,
                content_hash: hash_content(&raw.content),
                fetched_at: raw.fetched_at,
                title: raw.title,
                http_headers: std::collections::HashMap::new(),
                metadata: raw.metadata,
            };

            self.store.store_page(&cached).await?;
            cached_pages.push(cached);
        }

        Ok(cached_pages)
    }
}

/// Result of an ingest operation.
#[derive(Debug, Clone)]
pub struct IngestResult {
    pub page_urls: Vec<String>,
    pub pages_processed: usize,
}

fn extract_site_url(url: &str) -> String {
    url::Url::parse(url)
        .map(|u| format!("{}://{}", u.scheme(), u.host_str().unwrap_or("")))
        .unwrap_or_else(|_| url.to_string())
}

fn hash_content(content: &str) -> String {
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}
```

### Phase 4: Update ExtractionService for Server Use

**File:** `packages/server/src/kernel/extraction_service.rs`

```rust
use extraction::traits::Ingestor;
use extraction::pipeline::IngestResult;

impl<A: AI + Clone> ExtractionService<A> {
    /// Ingest a website using provided ingestor.
    pub async fn ingest_site<I: Ingestor>(
        &self,
        url: &str,
        limit: usize,
        ingestor: &I,
    ) -> Result<IngestResult> {
        self.index
            .ingest(url, limit, ingestor)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))
    }

    /// Get pages for a site (for linking to website_snapshots).
    pub async fn get_site_pages(&self, site_url: &str) -> Result<Vec<String>> {
        self.index
            .store()
            .get_pages_for_site(site_url)
            .await
            .map(|pages| pages.into_iter().map(|p| p.url).collect())
            .map_err(|e| anyhow::anyhow!("{}", e))
    }

    /// Get summaries for extraction.
    pub async fn get_summaries(&self, site_url: &str) -> Result<Vec<Summary>> {
        self.index
            .store()
            .get_summaries_for_site(site_url)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))
    }
}
```

### Phase 5: Simplify Server Crawling Actions

**File:** `packages/server/src/domains/crawling/actions/mod.rs`

```rust
use extraction::ingestors::FirecrawlIngestor;
use extraction::traits::ValidatedIngestor;

/// Crawl a website using the extraction library's Ingestor pattern.
pub async fn crawl_website(
    website_id: WebsiteId,
    requested_by: MemberId,
    is_admin: bool,
    deps: &ServerDeps,
) -> Result<CrawlResult> {
    // 1. Auth check (server-specific)
    check_crawl_authorization(requested_by, is_admin, "CrawlWebsite", deps).await?;

    // 2. Load website config
    let website = Website::find_by_id(website_id, &deps.db_pool).await?;
    Website::start_crawl(&deps.db_pool, website_id).await?;

    // 3. Create ingestor (SSRF-safe wrapper)
    let firecrawl = FirecrawlIngestor::new(&deps.firecrawl_api_key);
    let ingestor = ValidatedIngestor::new(firecrawl);

    // 4. Use extraction library for ingestion
    let extraction = deps.extraction.as_ref()
        .ok_or_else(|| anyhow::anyhow!("Extraction service not configured"))?;

    let limit = website.max_pages.unwrap_or(50) as usize;
    let result = extraction.ingest_site(&website.url, limit, &ingestor).await?;

    // 5. Link pages to website (server-specific junction)
    for page_url in &result.page_urls {
        WebsiteSnapshot::link_page(&deps.db_pool, website_id, page_url).await?;
    }

    // 6. Update website status
    Website::complete_crawl(&deps.db_pool, website_id, result.pages_processed as i32).await?;

    Ok(CrawlResult {
        website_id,
        pages_crawled: result.pages_processed,
        pages: result.page_urls,
    })
}
```

### Phase 6: Redesign WebsiteSnapshot

**Migration:** `packages/server/migrations/000096_redesign_website_snapshots.sql`

```sql
-- Step 1: Add new columns
ALTER TABLE website_snapshots
    ADD COLUMN page_url_new TEXT,
    ADD COLUMN last_synced_at TIMESTAMPTZ;

-- Step 2: Migrate data (join through page_snapshots to get URL)
UPDATE website_snapshots ws
SET page_url_new = ps.url,
    last_synced_at = ps.crawled_at
FROM page_snapshots ps
WHERE ws.page_snapshot_id = ps.id;

-- Step 3: For rows without page_snapshot, use existing page_url
UPDATE website_snapshots
SET page_url_new = page_url
WHERE page_url_new IS NULL AND page_url IS NOT NULL;

-- Step 4: Drop old columns and rename
ALTER TABLE website_snapshots
    DROP COLUMN page_snapshot_id,
    DROP COLUMN page_url;

ALTER TABLE website_snapshots
    RENAME COLUMN page_url_new TO page_url;

-- Step 5: Add NOT NULL constraint
ALTER TABLE website_snapshots
    ALTER COLUMN page_url SET NOT NULL;

-- Step 6: Update unique constraint
DROP INDEX IF EXISTS idx_website_snapshots_unique;
CREATE UNIQUE INDEX idx_website_snapshots_unique
    ON website_snapshots(website_id, page_url);
```

### Phase 7: Delete Obsolete Files & Tables

**Files to delete:**
```
packages/server/src/domains/crawling/models/page_snapshot.rs
packages/server/src/domains/crawling/models/page_summary.rs
packages/server/src/domains/crawling/models/page_extraction.rs
packages/server/src/domains/crawling/effects/extraction/summarize.rs
packages/server/src/domains/crawling/actions/build_pages.rs
packages/server/src/domains/crawling/actions/crawl_website.rs (most of it)
```

**Migration:** `packages/server/migrations/000097_drop_old_crawling_tables.sql`

```sql
-- Drop tables replaced by extraction library
DROP TABLE IF EXISTS field_provenance CASCADE;
DROP TABLE IF EXISTS relationships CASCADE;
DROP TABLE IF EXISTS detections CASCADE;
DROP TABLE IF EXISTS extractions CASCADE;
DROP TABLE IF EXISTS schemas CASCADE;
DROP TABLE IF EXISTS page_extractions CASCADE;
DROP TABLE IF EXISTS page_summaries CASCADE;
DROP TABLE IF EXISTS page_snapshots CASCADE;
```

## Files Summary

### New Files (Extraction Library)

| File | Purpose |
|------|---------|
| `src/traits/ingestor.rs` | Ingestor trait + ValidatedIngestor |
| `src/ingestors/mod.rs` | Ingestor implementations module |
| `src/ingestors/firecrawl.rs` | FirecrawlIngestor |
| `src/ingestors/http.rs` | HttpIngestor (basic) |

### New Files (Server)

| File | Purpose |
|------|---------|
| `migrations/000096_redesign_website_snapshots.sql` | Schema migration |
| `migrations/000097_drop_old_crawling_tables.sql` | Cleanup |

### Modified Files

| File | Changes |
|------|---------|
| `extraction/src/lib.rs` | Export ingestors |
| `extraction/src/traits/mod.rs` | Export Ingestor trait |
| `extraction/src/pipeline/index.rs` | Add ingest() method |
| `server/src/kernel/extraction_service.rs` | Add ingest_site() |
| `server/src/domains/crawling/actions/mod.rs` | Use Ingestor pattern |

### Deleted Files

| File | Reason |
|------|--------|
| `page_snapshot.rs` | Replaced by extraction_pages |
| `page_summary.rs` | Replaced by extraction_summaries |
| `page_extraction.rs` | Replaced by extraction_summaries |
| `summarize.rs` | Index handles this |
| `build_pages.rs` | No longer needed |

## Benefits of Ingestor Pattern

1. **Domain-agnostic** - Library doesn't know about Firecrawl, Playwright, etc.
2. **Version isolation** - New scraping tools just implement the trait
3. **SSRF safety** - ValidatedIngestor wraps any implementation
4. **Document support** - DocumentIngestor for PDFs, S3 buckets
5. **Detective integration** - fetch_specific() for gap-filling
6. **Testability** - MockIngestor for unit tests

## Success Criteria

- [ ] Ingestor trait defined in extraction library
- [ ] FirecrawlIngestor implements Ingestor
- [ ] Index.ingest() uses Ingestor pattern
- [ ] Server crawling uses extraction library
- [ ] Old tables dropped
- [ ] All tests pass
