//! In-memory storage implementation for testing and development.

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::RwLock;

use crate::error::Result;
use crate::traits::store::{cosine_similarity, EmbeddingStore, KeywordSearch, PageCache, SummaryCache};
use crate::types::{
    config::QueryFilter,
    page::{CachedPage, PageRef},
    summary::Summary,
};

/// In-memory storage for pages, summaries, and embeddings.
///
/// Useful for testing and development. Not suitable for production
/// as data is lost on restart.
pub struct MemoryStore {
    pages: RwLock<HashMap<String, CachedPage>>,
    summaries: RwLock<HashMap<String, Summary>>,
    embeddings: RwLock<HashMap<String, Vec<f32>>>,
}

impl Default for MemoryStore {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryStore {
    /// Create a new empty memory store.
    pub fn new() -> Self {
        Self {
            pages: RwLock::new(HashMap::new()),
            summaries: RwLock::new(HashMap::new()),
            embeddings: RwLock::new(HashMap::new()),
        }
    }

    /// Clear all stored data.
    pub fn clear(&self) {
        self.pages.write().unwrap().clear();
        self.summaries.write().unwrap().clear();
        self.embeddings.write().unwrap().clear();
    }

    /// Get the number of stored pages.
    pub fn page_count(&self) -> usize {
        self.pages.read().unwrap().len()
    }

    /// Get the number of stored summaries.
    pub fn summary_count(&self) -> usize {
        self.summaries.read().unwrap().len()
    }

    /// Get the number of stored embeddings.
    pub fn embedding_count(&self) -> usize {
        self.embeddings.read().unwrap().len()
    }
}

#[async_trait]
impl PageCache for MemoryStore {
    async fn get_page(&self, url: &str) -> Result<Option<CachedPage>> {
        Ok(self.pages.read().unwrap().get(url).cloned())
    }

    async fn store_page(&self, page: &CachedPage) -> Result<()> {
        self.pages
            .write()
            .unwrap()
            .insert(page.url.clone(), page.clone());
        Ok(())
    }

    async fn get_pages_for_site(&self, site_url: &str) -> Result<Vec<CachedPage>> {
        Ok(self
            .pages
            .read()
            .unwrap()
            .values()
            .filter(|p| p.site_url == site_url)
            .cloned()
            .collect())
    }

    async fn delete_page(&self, url: &str) -> Result<()> {
        self.pages.write().unwrap().remove(url);
        Ok(())
    }

    async fn count_pages(&self, site_url: &str) -> Result<usize> {
        Ok(self
            .pages
            .read()
            .unwrap()
            .values()
            .filter(|p| p.site_url == site_url)
            .count())
    }
}

#[async_trait]
impl SummaryCache for MemoryStore {
    async fn get_summary(&self, url: &str, content_hash: &str) -> Result<Option<Summary>> {
        Ok(self
            .summaries
            .read()
            .unwrap()
            .get(url)
            .filter(|s| s.content_hash == content_hash)
            .cloned())
    }

    async fn store_summary(&self, summary: &Summary) -> Result<()> {
        self.summaries
            .write()
            .unwrap()
            .insert(summary.url.clone(), summary.clone());
        Ok(())
    }

    async fn get_summaries_for_site(&self, site_url: &str) -> Result<Vec<Summary>> {
        Ok(self
            .summaries
            .read()
            .unwrap()
            .values()
            .filter(|s| s.site_url == site_url)
            .cloned()
            .collect())
    }

    async fn get_summaries(&self, filter: Option<&QueryFilter>) -> Result<Vec<Summary>> {
        let summaries = self.summaries.read().unwrap();

        Ok(summaries
            .values()
            .filter(|s| {
                if let Some(f) = filter {
                    f.matches_site(&s.site_url) && f.matches_date(s.created_at)
                } else {
                    true
                }
            })
            .cloned()
            .collect())
    }

    async fn delete_summary(&self, url: &str) -> Result<()> {
        self.summaries.write().unwrap().remove(url);
        Ok(())
    }

    async fn invalidate_stale_summaries(&self, current_prompt_hash: &str) -> Result<usize> {
        let mut summaries = self.summaries.write().unwrap();
        let stale_urls: Vec<_> = summaries
            .iter()
            .filter(|(_, s)| s.prompt_hash != current_prompt_hash)
            .map(|(url, _)| url.clone())
            .collect();

        let count = stale_urls.len();
        for url in stale_urls {
            summaries.remove(&url);
        }

        Ok(count)
    }
}

#[async_trait]
impl EmbeddingStore for MemoryStore {
    async fn store_embedding(&self, url: &str, embedding: &[f32]) -> Result<()> {
        self.embeddings
            .write()
            .unwrap()
            .insert(url.to_string(), embedding.to_vec());
        Ok(())
    }

    async fn get_embedding(&self, url: &str) -> Result<Option<Vec<f32>>> {
        Ok(self.embeddings.read().unwrap().get(url).cloned())
    }

    async fn search_similar(
        &self,
        embedding: &[f32],
        limit: usize,
        filter: Option<&QueryFilter>,
    ) -> Result<Vec<PageRef>> {
        let embeddings = self.embeddings.read().unwrap();
        let summaries = self.summaries.read().unwrap();

        let mut scored: Vec<_> = embeddings
            .iter()
            .filter_map(|(url, emb)| {
                // Get summary for site_url
                let summary = summaries.get(url)?;

                // Apply filter
                if let Some(f) = filter {
                    if !f.matches_site(&summary.site_url) || !f.matches_date(summary.created_at) {
                        return None;
                    }
                }

                let score = cosine_similarity(embedding, emb);
                Some(PageRef {
                    url: url.clone(),
                    title: None,
                    site_url: summary.site_url.clone(),
                    score,
                })
            })
            .collect();

        scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(limit);

        Ok(scored)
    }

    async fn delete_embedding(&self, url: &str) -> Result<()> {
        self.embeddings.write().unwrap().remove(url);
        Ok(())
    }
}

#[async_trait]
impl KeywordSearch for MemoryStore {
    async fn keyword_search(
        &self,
        query: &str,
        limit: usize,
        filter: Option<&QueryFilter>,
    ) -> Result<Vec<PageRef>> {
        let pages = self.pages.read().unwrap();
        let query_lower = query.to_lowercase();
        let query_terms: Vec<&str> = query_lower.split_whitespace().collect();

        let mut scored: Vec<_> = pages
            .values()
            .filter(|p| {
                // Apply site filter
                if let Some(f) = filter {
                    if !f.matches_site(&p.site_url) {
                        return false;
                    }
                }
                true
            })
            .filter_map(|page| {
                // Simple term frequency scoring
                let content_lower = page.content.to_lowercase();
                let mut score = 0.0;

                for term in &query_terms {
                    // Count occurrences
                    let count = content_lower.matches(term).count();
                    if count > 0 {
                        // TF-IDF-like scoring (simplified)
                        score += (1.0 + (count as f32).ln()) / (1.0 + (page.content.len() as f32).ln());
                    }
                }

                if score > 0.0 {
                    Some(PageRef {
                        url: page.url.clone(),
                        title: page.title.clone(),
                        site_url: page.site_url.clone(),
                        score,
                    })
                } else {
                    None
                }
            })
            .collect();

        scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(limit);

        Ok(scored)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_page_crud() {
        let store = MemoryStore::new();
        let page = CachedPage::new("https://example.com", "https://example.com", "Hello world");

        // Store
        store.store_page(&page).await.unwrap();
        assert_eq!(store.page_count(), 1);

        // Get
        let retrieved = store.get_page("https://example.com").await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().content, "Hello world");

        // Delete
        store.delete_page("https://example.com").await.unwrap();
        assert_eq!(store.page_count(), 0);
    }

    #[tokio::test]
    async fn test_summary_with_content_hash() {
        let store = MemoryStore::new();
        let summary = Summary::new(
            "https://example.com",
            "https://example.com",
            "Summary text",
            "hash123",
            "prompt123",
        );

        store.store_summary(&summary).await.unwrap();

        // Get with matching hash
        let retrieved = store
            .get_summary("https://example.com", "hash123")
            .await
            .unwrap();
        assert!(retrieved.is_some());

        // Get with different hash (content changed)
        let retrieved = store
            .get_summary("https://example.com", "different_hash")
            .await
            .unwrap();
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_embedding_search() {
        let store = MemoryStore::new();

        // Store a summary (needed for site_url lookup)
        let summary = Summary::new(
            "https://example.com/page1",
            "https://example.com",
            "Test",
            "hash",
            "prompt",
        );
        store.store_summary(&summary).await.unwrap();

        // Store embedding
        let embedding = vec![1.0, 0.0, 0.0];
        store
            .store_embedding("https://example.com/page1", &embedding)
            .await
            .unwrap();

        // Search with similar embedding
        let query = vec![0.9, 0.1, 0.0];
        let results = store.search_similar(&query, 10, None).await.unwrap();

        assert_eq!(results.len(), 1);
        assert!(results[0].score > 0.9);
    }

    #[tokio::test]
    async fn test_invalidate_stale_summaries() {
        let store = MemoryStore::new();

        // Store summaries with different prompt hashes
        let s1 = Summary::new("url1", "site", "text", "content", "old_prompt");
        let s2 = Summary::new("url2", "site", "text", "content", "new_prompt");
        let s3 = Summary::new("url3", "site", "text", "content", "old_prompt");

        store.store_summary(&s1).await.unwrap();
        store.store_summary(&s2).await.unwrap();
        store.store_summary(&s3).await.unwrap();

        assert_eq!(store.summary_count(), 3);

        // Invalidate old prompts
        let invalidated = store
            .invalidate_stale_summaries("new_prompt")
            .await
            .unwrap();

        assert_eq!(invalidated, 2);
        assert_eq!(store.summary_count(), 1);
    }

    #[tokio::test]
    async fn test_filter_by_site() {
        let store = MemoryStore::new();

        let s1 = Summary::new("url1", "https://a.com", "text", "hash", "prompt");
        let s2 = Summary::new("url2", "https://b.com", "text", "hash", "prompt");
        let s3 = Summary::new("url3", "https://a.com", "text", "hash", "prompt");

        store.store_summary(&s1).await.unwrap();
        store.store_summary(&s2).await.unwrap();
        store.store_summary(&s3).await.unwrap();

        let filter = QueryFilter::for_site("a.com");
        let results = store.get_summaries(Some(&filter)).await.unwrap();

        assert_eq!(results.len(), 2);
    }
}
