# Plan: Tavily-Powered Discovery (Hybrid Crawl)

## Overview

Replace traditional link-following crawl with Tavily site-scoped search for discovering relevant pages. This targets the actual content (volunteer, services, programs) rather than blindly crawling.

## Current Flow

```
Submit URL → Crawl homepage → Follow links → Scrape pages → Summarize → Synthesize → LLM Sync
                    ↓
            Problem: Crawls irrelevant pages (blog, staff, news)
            Problem: May miss pages not linked from homepage
            Problem: JS rendering issues
```

## Proposed Flow

```
Submit domain → Tavily Discovery → Get relevant pages + content → Summarize → Synthesize → LLM Sync
                      ↓
               Multiple targeted queries:
               - site:example.org volunteer
               - site:example.org donate
               - site:example.org services programs
               - site:example.org food pantry help
```

---

## Architecture

### Phase 1: Tavily Client

```rust
// src/kernel/tavily.rs

pub struct TavilyClient {
    api_key: String,
    http: reqwest::Client,
}

#[derive(Debug, Deserialize)]
pub struct TavilySearchResult {
    pub url: String,
    pub title: String,
    pub content: String,        // Extracted text content
    pub raw_content: Option<String>, // Full page content (if requested)
    pub score: f64,             // Relevance score
}

#[derive(Debug, Deserialize)]
pub struct TavilyResponse {
    pub results: Vec<TavilySearchResult>,
}

impl TavilyClient {
    pub async fn search(&self, query: &str, max_results: usize) -> Result<Vec<TavilySearchResult>> {
        // POST https://api.tavily.com/search
        // {
        //   "api_key": "...",
        //   "query": "site:example.org volunteer",
        //   "max_results": 10,
        //   "include_raw_content": true,  // Get full page content
        //   "search_depth": "advanced"
        // }
    }
}
```

### Phase 2: Discovery Queries

```rust
// src/domains/crawling/effects/discovery.rs

/// Standard queries to find community resource content
const DISCOVERY_QUERIES: &[&str] = &[
    "volunteer opportunities",
    "donate donation giving",
    "services programs",
    "food pantry meals",
    "housing shelter",
    "help resources assistance",
    "events calendar",
    "contact hours location",
];

pub struct DiscoveredPage {
    pub url: String,
    pub title: String,
    pub content: String,
    pub relevance_score: f64,
}

/// Discover relevant pages on a domain using Tavily search
pub async fn discover_pages(
    domain: &str,
    tavily: &TavilyClient,
) -> Result<Vec<DiscoveredPage>> {
    let mut all_results = Vec::new();
    let mut seen_urls = HashSet::new();

    for query_terms in DISCOVERY_QUERIES {
        let query = format!("site:{} {}", domain, query_terms);
        let results = tavily.search(&query, 5).await?;

        for result in results {
            if seen_urls.insert(result.url.clone()) {
                all_results.push(DiscoveredPage {
                    url: result.url,
                    title: result.title,
                    content: result.raw_content.unwrap_or(result.content),
                    relevance_score: result.score,
                });
            }
        }
    }

    // Sort by relevance, take top N
    all_results.sort_by(|a, b| b.relevance_score.partial_cmp(&a.relevance_score).unwrap());
    all_results.truncate(20); // Max pages per website

    Ok(all_results)
}
```

### Phase 3: Integration with Existing Pipeline

```rust
// New command variant
pub enum CrawlCommand {
    // Existing
    CrawlWebsite { website_id, max_pages, ... },

    // New: Tavily-based discovery
    DiscoverWebsite { website_id, job_id },
}

// Handler
async fn handle_discover_website(website_id: WebsiteId, job_id: JobId, ctx: &EffectContext) {
    let website = Website::find_by_id(website_id, pool).await?;

    // 1. Run Tavily discovery
    let pages = discover_pages(&website.domain, tavily).await?;

    // 2. Store as page snapshots (reuse existing model)
    for page in &pages {
        let snapshot = PageSnapshot::create(
            page.url.clone(),
            page.content.clone(),  // Use Tavily's extracted content
            "".to_string(),        // No raw HTML needed
            "tavily".to_string(),  // fetched_via = "tavily"
            pool,
        ).await?;

        WebsiteSnapshot::create(website_id, snapshot.id, &page.url, pool).await?;
    }

    // 3. Emit event to trigger summarization (existing pipeline)
    Ok(CrawlEvent::WebsiteCrawled {
        website_id,
        job_id,
        pages_found: pages.len(),
    })
}
```

---

## UI Changes

### Option A: Replace "Full Crawl" button
```
[Discover Pages]  ← Uses Tavily
```

### Option B: Add as alternative (recommended for testing)
```
[Full Crawl]  [Discover via Search]
```

### Option C: Make it the default for new websites
- New websites use Tavily discovery
- Existing websites keep crawl option
- Can switch between methods

---

## Query Strategy

### Base queries (always run)
```
site:{domain} volunteer
site:{domain} donate donation
site:{domain} services programs
site:{domain} help resources
```

### Category-specific queries (based on organization type)
```yaml
food_bank:
  - site:{domain} food pantry
  - site:{domain} meals distribution

shelter:
  - site:{domain} housing shelter
  - site:{domain} emergency beds

legal_aid:
  - site:{domain} legal help attorney
  - site:{domain} immigration assistance
```

### Smart query generation (future)
Use LLM to generate relevant queries based on:
- Organization name
- Initial homepage content
- Previous successful extractions

---

## Benefits

| Aspect | Traditional Crawl | Tavily Discovery |
|--------|------------------|------------------|
| Relevance | Crawls everything | Only relevant pages |
| Coverage | Limited by links | Finds unlinked pages |
| JS rendering | Often fails | Handled by Tavily |
| Speed | Slow (sequential) | Fast (parallel API) |
| Cost | Server resources | API cost (~$0.01/search) |
| Freshness | Real-time | Depends on index |

---

## Implementation Steps

### Step 1: Add Tavily Client
- [ ] Add `tavily` to dependencies or use `reqwest` directly
- [ ] Create `src/kernel/tavily.rs` with client
- [ ] Add `TAVILY_API_KEY` to environment config
- [ ] Add to `ServerDeps`

### Step 2: Discovery Effect
- [ ] Create `src/domains/crawling/effects/discovery.rs`
- [ ] Implement `discover_pages()` function
- [ ] Define discovery query strategy

### Step 3: New Command/Event
- [ ] Add `DiscoverWebsite` command variant
- [ ] Add handler that uses discovery + feeds into existing pipeline
- [ ] Reuse `WebsiteCrawled` event to trigger summarization

### Step 4: GraphQL Mutation
- [ ] Add `discoverWebsite(websiteId: ID!)` mutation
- [ ] Or modify `crawlWebsite` to accept `method: "crawl" | "discover"`

### Step 5: UI Button
- [ ] Add "Discover via Search" button to WebsiteDetail.tsx
- [ ] Show which method was used in crawl status

### Step 6: Testing
- [ ] Test with known websites (dhhmn.org, etc.)
- [ ] Compare results: crawl vs discovery
- [ ] Measure coverage and relevance

---

## Cost Estimation

Tavily pricing: ~$0.01 per search

Per website discovery:
- 8 base queries × $0.01 = $0.08
- Plus category queries: ~$0.04
- **Total: ~$0.12 per website**

vs. Traditional crawl:
- Server compute time
- Headless browser resources
- Rate limiting delays

---

## Open Questions

1. **Fallback strategy**: What if Tavily returns no results for a domain?
   - Fall back to traditional crawl?
   - Try different query variations?

2. **Content freshness**: How to handle stale search index?
   - Accept it (most content is stable)
   - Hybrid: Use Tavily for URL discovery, then scrape for fresh content

3. **Rate limiting**: How many queries per minute?
   - Tavily allows 100 requests/minute on paid plans
   - Batch websites or queue

4. **When to re-discover**:
   - On manual trigger only?
   - Scheduled like crawl?
