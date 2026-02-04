//! Storage traits for pages, summaries, and embeddings.
//!
//! The storage layer is split into focused traits for flexibility:
//! - `PageCache`: Raw page content
//! - `SummaryCache`: Recall-optimized summaries
//! - `EmbeddingStore`: Vector embeddings for semantic search
//! - `PageStore`: Composite trait combining all three

use async_trait::async_trait;

use crate::error::Result;
use crate::types::{
    config::QueryFilter,
    page::{CachedPage, PageRef},
    summary::Summary,
};

/// Cache for raw page content.
#[async_trait]
pub trait PageCache: Send + Sync {
    /// Get a cached page by URL.
    async fn get_page(&self, url: &str) -> Result<Option<CachedPage>>;

    /// Store a page.
    async fn store_page(&self, page: &CachedPage) -> Result<()>;

    /// Get multiple pages by URL.
    async fn get_pages(&self, urls: &[&str]) -> Result<Vec<CachedPage>> {
        let mut pages = Vec::with_capacity(urls.len());
        for url in urls {
            if let Some(page) = self.get_page(url).await? {
                pages.push(page);
            }
        }
        Ok(pages)
    }

    /// Get all pages for a site.
    async fn get_pages_for_site(&self, site_url: &str) -> Result<Vec<CachedPage>>;

    /// Delete a page by URL.
    async fn delete_page(&self, url: &str) -> Result<()>;

    /// Count pages for a site.
    async fn count_pages(&self, site_url: &str) -> Result<usize>;
}

/// Cache for recall-optimized summaries.
#[async_trait]
pub trait SummaryCache: Send + Sync {
    /// Get a summary by URL and content hash.
    ///
    /// Returns None if:
    /// - No summary exists
    /// - Summary exists but content hash doesn't match (content changed)
    async fn get_summary(&self, url: &str, content_hash: &str) -> Result<Option<Summary>>;

    /// Store a summary.
    async fn store_summary(&self, summary: &Summary) -> Result<()>;

    /// Get all summaries for a site.
    async fn get_summaries_for_site(&self, site_url: &str) -> Result<Vec<Summary>>;

    /// Get summaries matching a filter.
    async fn get_summaries(&self, filter: Option<&QueryFilter>) -> Result<Vec<Summary>>;

    /// Delete summary by URL.
    async fn delete_summary(&self, url: &str) -> Result<()>;

    /// Invalidate summaries with old prompt hash.
    ///
    /// Returns the number of summaries invalidated.
    async fn invalidate_stale_summaries(&self, current_prompt_hash: &str) -> Result<usize>;
}

/// Store for vector embeddings (semantic search).
#[async_trait]
pub trait EmbeddingStore: Send + Sync {
    /// Store an embedding for a URL.
    async fn store_embedding(&self, url: &str, embedding: &[f32]) -> Result<()>;

    /// Get embedding for a URL.
    async fn get_embedding(&self, url: &str) -> Result<Option<Vec<f32>>>;

    /// Search for similar pages by embedding.
    ///
    /// Returns page refs sorted by similarity (highest first).
    async fn search_similar(
        &self,
        embedding: &[f32],
        limit: usize,
        filter: Option<&QueryFilter>,
    ) -> Result<Vec<PageRef>>;

    /// Search similar with score threshold.
    async fn search_similar_threshold(
        &self,
        embedding: &[f32],
        min_score: f32,
        limit: usize,
        filter: Option<&QueryFilter>,
    ) -> Result<Vec<PageRef>> {
        let results = self.search_similar(embedding, limit * 2, filter).await?;
        Ok(results
            .into_iter()
            .filter(|r| r.score >= min_score)
            .take(limit)
            .collect())
    }

    /// Delete embedding by URL.
    async fn delete_embedding(&self, url: &str) -> Result<()>;
}

/// Composite storage trait combining all caches.
///
/// This is the main trait used by the Index.
pub trait PageStore: PageCache + SummaryCache + EmbeddingStore {}

// Blanket implementation: anything implementing all three traits is a PageStore
impl<T: PageCache + SummaryCache + EmbeddingStore> PageStore for T {}

/// Site metadata stored alongside pages.
#[derive(Debug, Clone)]
pub struct SiteMetadata {
    /// Canonical site URL
    pub url: String,

    /// Site title
    pub title: Option<String>,

    /// When the site was first indexed
    pub first_indexed: chrono::DateTime<chrono::Utc>,

    /// When the site was last crawled
    pub last_crawled: chrono::DateTime<chrono::Utc>,

    /// Number of pages indexed
    pub page_count: usize,

    /// Number of summaries generated
    pub summary_count: usize,
}

/// Keyword search capabilities (for hybrid recall).
#[async_trait]
pub trait KeywordSearch: Send + Sync {
    /// Search raw page content with BM25.
    ///
    /// Returns page refs sorted by relevance.
    async fn keyword_search(
        &self,
        query: &str,
        limit: usize,
        filter: Option<&QueryFilter>,
    ) -> Result<Vec<PageRef>>;
}

/// Combined semantic + keyword search.
#[async_trait]
pub trait HybridSearch: EmbeddingStore + KeywordSearch {
    /// Hybrid search combining semantic and keyword results.
    ///
    /// Uses Reciprocal Rank Fusion (RRF) to combine results.
    async fn hybrid_search(
        &self,
        query: &str,
        query_embedding: &[f32],
        limit: usize,
        semantic_weight: f32,
        filter: Option<&QueryFilter>,
    ) -> Result<Vec<PageRef>> {
        let semantic_results = self
            .search_similar(query_embedding, limit * 2, filter)
            .await?;
        let keyword_results = self.keyword_search(query, limit * 2, filter).await?;

        Ok(reciprocal_rank_fusion(
            &semantic_results,
            &keyword_results,
            semantic_weight,
            1.0 - semantic_weight,
        )
        .into_iter()
        .take(limit)
        .collect())
    }
}

// Blanket implementation
impl<T: EmbeddingStore + KeywordSearch> HybridSearch for T {}

/// Reciprocal Rank Fusion for combining search results.
///
/// RRF score = sum(1 / (k + rank)) for each list
/// k is typically 60 (constant to prevent high ranks from dominating)
pub fn reciprocal_rank_fusion(
    results_a: &[PageRef],
    results_b: &[PageRef],
    weight_a: f32,
    weight_b: f32,
) -> Vec<PageRef> {
    use std::collections::HashMap;

    const K: f32 = 60.0;
    let mut scores: HashMap<String, (f32, Option<String>, String)> = HashMap::new();

    // Score from results A
    for (rank, page) in results_a.iter().enumerate() {
        let rrf_score = weight_a / (K + rank as f32 + 1.0);
        scores
            .entry(page.url.clone())
            .and_modify(|(s, _, _)| *s += rrf_score)
            .or_insert((rrf_score, page.title.clone(), page.site_url.clone()));
    }

    // Score from results B
    for (rank, page) in results_b.iter().enumerate() {
        let rrf_score = weight_b / (K + rank as f32 + 1.0);
        scores
            .entry(page.url.clone())
            .and_modify(|(s, _, _)| *s += rrf_score)
            .or_insert((rrf_score, page.title.clone(), page.site_url.clone()));
    }

    // Sort by combined score
    let mut combined: Vec<_> = scores
        .into_iter()
        .map(|(url, (score, title, site_url))| PageRef {
            url,
            title,
            site_url,
            score,
        })
        .collect();

    combined.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    combined
}

/// Cosine similarity between two vectors.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }

    dot / (norm_a * norm_b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 0.001);

        let c = vec![0.0, 1.0, 0.0];
        assert!(cosine_similarity(&a, &c).abs() < 0.001);

        let d = vec![-1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &d) + 1.0).abs() < 0.001);
    }

    #[test]
    fn test_rrf_basic() {
        let a = vec![
            PageRef::new("url1", "site").with_score(0.9),
            PageRef::new("url2", "site").with_score(0.8),
        ];
        let b = vec![
            PageRef::new("url2", "site").with_score(0.95),
            PageRef::new("url3", "site").with_score(0.85),
        ];

        let combined = reciprocal_rank_fusion(&a, &b, 0.5, 0.5);

        // url2 should be first (appears in both)
        assert_eq!(combined[0].url, "url2");
        assert_eq!(combined.len(), 3);
    }
}
