---
status: pending
priority: p2
issue_id: "009"
tags: [code-review, performance, caching, cost-optimization]
dependencies: []
---

# OpenAI Embedding API Has No Cache - Wasting $200-500/Month

## Problem Statement

The embedding generation service calls OpenAI's API on every request without caching, causing duplicate API calls for identical content and wasting significant money at scale.

## Findings

**Location**: `/packages/server/src/common/utils/embeddings.rs`

**Current Implementation**:
```rust
impl BaseEmbeddingService for EmbeddingService {
    async fn generate(&self, text: &str) -> Result<Vec<f32>> {
        let response = self.client
            .post("https://api.openai.com/v1/embeddings")  // ⚠️ NO CACHE
            .send()
            .await?;
        // ...
    }
}
```

**Cost Impact**:
- OpenAI embedding API: $0.02 per 1M tokens
- Average need description: ~100 tokens
- 1,000 needs/day: $0.60/day wasted on duplicates
- At scale: **$200-500/month** wasted on re-generating unchanged embeddings

**From Performance Oracle Agent**: "Re-generating embeddings for unchanged content. Expected Savings: 90% reduction in API calls, $200-500/month at scale, 200-500ms latency reduction per cached hit."

**Problem**: When volunteers submit similar needs or organizations scrape the same content multiple times, embeddings are regenerated unnecessarily.

## Proposed Solutions

### Option 1: In-Memory Cache with Moka (Recommended)
**Pros**: Fast, no dependencies, LRU eviction
**Cons**: Cache lost on restart
**Effort**: Small (2 hours)
**Risk**: Low

```rust
use moka::future::Cache;
use std::sync::Arc;

pub struct EmbeddingService {
    client: Client,
    api_key: String,
    cache: Arc<Cache<String, Vec<f32>>>,
}

impl EmbeddingService {
    pub fn new(api_key: String) -> Self {
        let cache = Cache::builder()
            .max_capacity(10_000)  // 10K embeddings = ~60MB
            .time_to_live(Duration::from_secs(86400))  // 24 hours
            .build();

        Self {
            client: Client::new(),
            api_key,
            cache: Arc::new(cache),
        }
    }
}

impl BaseEmbeddingService for EmbeddingService {
    async fn generate(&self, text: &str) -> Result<Vec<f32>> {
        // Check cache first
        if let Some(cached) = self.cache.get(text).await {
            return Ok(cached);
        }

        // Generate and cache
        let embedding = self.generate_from_api(text).await?;
        self.cache.insert(text.to_string(), embedding.clone()).await;
        Ok(embedding)
    }
}
```

### Option 2: Redis-Backed Cache
**Pros**: Persistent, shared across instances
**Cons**: Requires Redis, network latency
**Effort**: Medium (3 hours)
**Risk**: Low

```rust
async fn generate(&self, text: &str) -> Result<Vec<f32>> {
    let cache_key = format!("embedding:{}", hash(text));

    // Check Redis
    if let Ok(cached) = self.redis.get::<_, Vec<u8>>(&cache_key).await {
        return Ok(deserialize(&cached)?);
    }

    // Generate and cache in Redis
    let embedding = self.generate_from_api(text).await?;
    self.redis.set_ex(cache_key, serialize(&embedding)?, 86400).await?;
    Ok(embedding)
}
```

### Option 3: Database-Backed Cache
**Pros**: Already have Postgres, persistent
**Cons**: Slower than memory, query overhead
**Effort**: Medium (3 hours)
**Risk**: Low

```sql
CREATE TABLE embedding_cache (
    content_hash VARCHAR(64) PRIMARY KEY,
    embedding vector(1536),
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_embedding_cache_created ON embedding_cache(created_at);
```

## Recommended Action

**Option 1** (Moka in-memory cache) for immediate deployment. 90% of savings with minimal complexity. Upgrade to Redis if running multiple servers.

## Technical Details

**Cache Key Strategy**:
Use SHA-256 hash of normalized text:
```rust
use sha2::{Sha256, Digest};

fn cache_key(text: &str) -> String {
    let normalized = text.trim().to_lowercase();
    let mut hasher = Sha256::new();
    hasher.update(normalized.as_bytes());
    format!("{:x}", hasher.finalize())
}
```

**Cache Size Calculation**:
- 1 embedding: 1536 floats × 4 bytes = 6 KB
- 10,000 embeddings: 60 MB
- 100,000 embeddings: 600 MB

**Affected Operations**:
- Need approval (generates embedding for matching)
- Member registration (generates embedding for skills)
- Content hash changes (triggers re-embedding)

## Acceptance Criteria

- [ ] Moka cache dependency added to Cargo.toml
- [ ] EmbeddingService updated with cache layer
- [ ] Cache key uses content hash (not raw text)
- [ ] Cache TTL set to 24 hours
- [ ] Cache size limited to 10K entries
- [ ] Cache hit/miss metrics added
- [ ] Tests verify cache behavior
- [ ] Monitor API cost reduction (expect 70-90% decrease)

## Work Log

*Empty - work not started*

## Resources

- **PR/Issue**: N/A - Found in code review
- **Related Code**:
  - `/packages/server/src/common/utils/embeddings.rs` (embedding service)
  - `/packages/server/src/domains/matching/effects/mod.rs:62` (generates embeddings)
- **Documentation**:
  - [Moka Cache](https://docs.rs/moka/)
  - [OpenAI Pricing](https://openai.com/pricing)
- **Cost Analysis**: At 1000 needs/day with 30% duplicates, wasting $0.18/day = $5.40/month currently. At 10K needs/day scale, would waste $54/month.
