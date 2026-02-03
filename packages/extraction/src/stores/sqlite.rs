//! SQLite storage implementation.
//!
//! A file-based storage backend using SQLite. Good for:
//! - Local development
//! - Single-server deployments
//! - Testing with persistent data

use async_trait::async_trait;
use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use sqlx::FromRow;

use crate::error::{ExtractionError, Result};
use crate::traits::store::{EmbeddingStore, KeywordSearch, PageCache, SummaryCache};
use crate::types::{
    config::QueryFilter,
    page::{CachedPage, PageRef},
    summary::{RecallSignals, Summary},
};

/// SQLite-based page store.
pub struct SqliteStore {
    pool: SqlitePool,
}

impl SqliteStore {
    /// Create a new SQLite store with the given connection URL.
    ///
    /// # Example URLs
    /// - `:memory:` - In-memory database (ephemeral)
    /// - `file:./extraction.db` - File-based database
    /// - `file:./test.db?mode=rwc` - Create if not exists
    pub async fn new(database_url: &str) -> Result<Self> {
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await
            .map_err(|e| ExtractionError::Storage(e.to_string().into()))?;

        let store = Self { pool };
        store.run_migrations().await?;
        Ok(store)
    }

    /// Create an in-memory SQLite store (for testing).
    pub async fn in_memory() -> Result<Self> {
        Self::new("sqlite::memory:").await
    }

    /// Run database migrations.
    async fn run_migrations(&self) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS pages (
                url TEXT PRIMARY KEY,
                site_url TEXT NOT NULL,
                content TEXT NOT NULL,
                content_hash TEXT NOT NULL,
                fetched_at TEXT NOT NULL,
                title TEXT,
                http_headers TEXT NOT NULL DEFAULT '{}',
                metadata TEXT NOT NULL DEFAULT '{}'
            );

            CREATE INDEX IF NOT EXISTS idx_pages_site_url ON pages(site_url);
            CREATE INDEX IF NOT EXISTS idx_pages_content_hash ON pages(content_hash);
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| ExtractionError::Storage(e.to_string().into()))?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS summaries (
                url TEXT PRIMARY KEY,
                site_url TEXT NOT NULL,
                text TEXT NOT NULL,
                signals TEXT NOT NULL DEFAULT '{}',
                language TEXT,
                created_at TEXT NOT NULL,
                prompt_hash TEXT NOT NULL,
                content_hash TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_summaries_site_url ON summaries(site_url);
            CREATE INDEX IF NOT EXISTS idx_summaries_prompt_hash ON summaries(prompt_hash);
            CREATE INDEX IF NOT EXISTS idx_summaries_content_hash ON summaries(content_hash);
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| ExtractionError::Storage(e.to_string().into()))?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS embeddings (
                url TEXT PRIMARY KEY,
                site_url TEXT NOT NULL,
                embedding BLOB NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_embeddings_site_url ON embeddings(site_url);
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| ExtractionError::Storage(e.to_string().into()))?;

        // Create FTS5 table for keyword search
        sqlx::query(
            r#"
            CREATE VIRTUAL TABLE IF NOT EXISTS pages_fts USING fts5(
                url,
                content,
                title,
                content='pages',
                content_rowid='rowid'
            );
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| ExtractionError::Storage(e.to_string().into()))?;

        Ok(())
    }

    /// Get the underlying connection pool.
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}

// Row types for sqlx queries
#[derive(Debug, FromRow)]
struct PageRow {
    url: String,
    site_url: String,
    content: String,
    content_hash: String,
    fetched_at: String,
    title: Option<String>,
    http_headers: String,
    metadata: String,
}

impl PageRow {
    fn into_cached_page(self) -> Result<CachedPage> {
        let fetched_at = chrono::DateTime::parse_from_rfc3339(&self.fetched_at)
            .map_err(|e| ExtractionError::Storage(format!("Invalid date: {}", e).into()))?
            .with_timezone(&chrono::Utc);

        let http_headers: std::collections::HashMap<String, String> =
            serde_json::from_str(&self.http_headers)
                .map_err(|e| ExtractionError::Storage(format!("Invalid headers JSON: {}", e).into()))?;

        let metadata: std::collections::HashMap<String, String> =
            serde_json::from_str(&self.metadata)
                .map_err(|e| ExtractionError::Storage(format!("Invalid metadata JSON: {}", e).into()))?;

        Ok(CachedPage {
            url: self.url,
            site_url: self.site_url,
            content: self.content,
            content_hash: self.content_hash,
            fetched_at,
            title: self.title,
            http_headers,
            metadata,
        })
    }
}

#[derive(Debug, FromRow)]
struct SummaryRow {
    url: String,
    site_url: String,
    text: String,
    signals: String,
    language: Option<String>,
    created_at: String,
    prompt_hash: String,
    content_hash: String,
}

impl SummaryRow {
    fn into_summary(self) -> Result<Summary> {
        let created_at = chrono::DateTime::parse_from_rfc3339(&self.created_at)
            .map_err(|e| ExtractionError::Storage(format!("Invalid date: {}", e).into()))?
            .with_timezone(&chrono::Utc);

        let signals: RecallSignals = serde_json::from_str(&self.signals)
            .map_err(|e| ExtractionError::Storage(format!("Invalid signals JSON: {}", e).into()))?;

        Ok(Summary {
            url: self.url,
            site_url: self.site_url,
            text: self.text,
            signals,
            language: self.language,
            created_at,
            prompt_hash: self.prompt_hash,
            content_hash: self.content_hash,
            embedding: None,
        })
    }
}

#[async_trait]
impl PageCache for SqliteStore {
    async fn get_page(&self, url: &str) -> Result<Option<CachedPage>> {
        let row = sqlx::query_as::<_, PageRow>(
            "SELECT url, site_url, content, content_hash, fetched_at, title, http_headers, metadata FROM pages WHERE url = ?",
        )
        .bind(url)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ExtractionError::Storage(e.to_string().into()))?;

        match row {
            Some(r) => Ok(Some(r.into_cached_page()?)),
            None => Ok(None),
        }
    }

    async fn store_page(&self, page: &CachedPage) -> Result<()> {
        let http_headers = serde_json::to_string(&page.http_headers)
            .map_err(|e| ExtractionError::Storage(e.to_string().into()))?;
        let metadata = serde_json::to_string(&page.metadata)
            .map_err(|e| ExtractionError::Storage(e.to_string().into()))?;

        sqlx::query(
            r#"
            INSERT INTO pages (url, site_url, content, content_hash, fetched_at, title, http_headers, metadata)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(url) DO UPDATE SET
                site_url = excluded.site_url,
                content = excluded.content,
                content_hash = excluded.content_hash,
                fetched_at = excluded.fetched_at,
                title = excluded.title,
                http_headers = excluded.http_headers,
                metadata = excluded.metadata
            "#,
        )
        .bind(&page.url)
        .bind(&page.site_url)
        .bind(&page.content)
        .bind(&page.content_hash)
        .bind(page.fetched_at.to_rfc3339())
        .bind(&page.title)
        .bind(&http_headers)
        .bind(&metadata)
        .execute(&self.pool)
        .await
        .map_err(|e| ExtractionError::Storage(e.to_string().into()))?;

        // Update FTS index
        sqlx::query(
            r#"
            INSERT INTO pages_fts(url, content, title)
            VALUES (?, ?, ?)
            ON CONFLICT(url) DO UPDATE SET
                content = excluded.content,
                title = excluded.title
            "#,
        )
        .bind(&page.url)
        .bind(&page.content)
        .bind(&page.title)
        .execute(&self.pool)
        .await
        .ok(); // FTS update is best-effort

        Ok(())
    }

    async fn get_pages_for_site(&self, site_url: &str) -> Result<Vec<CachedPage>> {
        let rows = sqlx::query_as::<_, PageRow>(
            "SELECT url, site_url, content, content_hash, fetched_at, title, http_headers, metadata FROM pages WHERE site_url = ?",
        )
        .bind(site_url)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ExtractionError::Storage(e.to_string().into()))?;

        rows.into_iter().map(|r| r.into_cached_page()).collect()
    }

    async fn delete_page(&self, url: &str) -> Result<()> {
        sqlx::query("DELETE FROM pages WHERE url = ?")
            .bind(url)
            .execute(&self.pool)
            .await
            .map_err(|e| ExtractionError::Storage(e.to_string().into()))?;

        // Also delete from FTS
        sqlx::query("DELETE FROM pages_fts WHERE url = ?")
            .bind(url)
            .execute(&self.pool)
            .await
            .ok();

        Ok(())
    }

    async fn count_pages(&self, site_url: &str) -> Result<usize> {
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM pages WHERE site_url = ?")
            .bind(site_url)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| ExtractionError::Storage(e.to_string().into()))?;

        Ok(count.0 as usize)
    }
}

#[async_trait]
impl SummaryCache for SqliteStore {
    async fn get_summary(&self, url: &str, content_hash: &str) -> Result<Option<Summary>> {
        let row = sqlx::query_as::<_, SummaryRow>(
            "SELECT url, site_url, text, signals, language, created_at, prompt_hash, content_hash FROM summaries WHERE url = ? AND content_hash = ?",
        )
        .bind(url)
        .bind(content_hash)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ExtractionError::Storage(e.to_string().into()))?;

        match row {
            Some(r) => Ok(Some(r.into_summary()?)),
            None => Ok(None),
        }
    }

    async fn store_summary(&self, summary: &Summary) -> Result<()> {
        let signals = serde_json::to_string(&summary.signals)
            .map_err(|e| ExtractionError::Storage(e.to_string().into()))?;

        sqlx::query(
            r#"
            INSERT INTO summaries (url, site_url, text, signals, language, created_at, prompt_hash, content_hash)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(url) DO UPDATE SET
                site_url = excluded.site_url,
                text = excluded.text,
                signals = excluded.signals,
                language = excluded.language,
                created_at = excluded.created_at,
                prompt_hash = excluded.prompt_hash,
                content_hash = excluded.content_hash
            "#,
        )
        .bind(&summary.url)
        .bind(&summary.site_url)
        .bind(&summary.text)
        .bind(&signals)
        .bind(&summary.language)
        .bind(summary.created_at.to_rfc3339())
        .bind(&summary.prompt_hash)
        .bind(&summary.content_hash)
        .execute(&self.pool)
        .await
        .map_err(|e| ExtractionError::Storage(e.to_string().into()))?;

        Ok(())
    }

    async fn get_summaries_for_site(&self, site_url: &str) -> Result<Vec<Summary>> {
        let rows = sqlx::query_as::<_, SummaryRow>(
            "SELECT url, site_url, text, signals, language, created_at, prompt_hash, content_hash FROM summaries WHERE site_url = ?",
        )
        .bind(site_url)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ExtractionError::Storage(e.to_string().into()))?;

        rows.into_iter().map(|r| r.into_summary()).collect()
    }

    async fn get_summaries(&self, filter: Option<&QueryFilter>) -> Result<Vec<Summary>> {
        let rows = match filter {
            Some(f) if !f.include_sites.is_empty() => {
                // Filter by site URLs
                let placeholders = f.include_sites.iter().map(|_| "?").collect::<Vec<_>>().join(",");
                let query = format!(
                    "SELECT url, site_url, text, signals, language, created_at, prompt_hash, content_hash FROM summaries WHERE site_url IN ({})",
                    placeholders
                );

                let mut q = sqlx::query_as::<_, SummaryRow>(&query);
                for url in &f.include_sites {
                    q = q.bind(url);
                }
                q.fetch_all(&self.pool)
                    .await
                    .map_err(|e| ExtractionError::Storage(e.to_string().into()))?
            }
            _ => {
                sqlx::query_as::<_, SummaryRow>(
                    "SELECT url, site_url, text, signals, language, created_at, prompt_hash, content_hash FROM summaries",
                )
                .fetch_all(&self.pool)
                .await
                .map_err(|e| ExtractionError::Storage(e.to_string().into()))?
            }
        };

        rows.into_iter().map(|r| r.into_summary()).collect()
    }

    async fn delete_summary(&self, url: &str) -> Result<()> {
        sqlx::query("DELETE FROM summaries WHERE url = ?")
            .bind(url)
            .execute(&self.pool)
            .await
            .map_err(|e| ExtractionError::Storage(e.to_string().into()))?;

        Ok(())
    }

    async fn invalidate_stale_summaries(&self, current_prompt_hash: &str) -> Result<usize> {
        let result = sqlx::query("DELETE FROM summaries WHERE prompt_hash != ?")
            .bind(current_prompt_hash)
            .execute(&self.pool)
            .await
            .map_err(|e| ExtractionError::Storage(e.to_string().into()))?;

        Ok(result.rows_affected() as usize)
    }
}

#[async_trait]
impl EmbeddingStore for SqliteStore {
    async fn store_embedding(&self, url: &str, embedding: &[f32]) -> Result<()> {
        // Get site_url from pages table
        let site_url: Option<(String,)> =
            sqlx::query_as("SELECT site_url FROM pages WHERE url = ?")
                .bind(url)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| ExtractionError::Storage(e.to_string().into()))?;

        let site_url = site_url
            .map(|(s,)| s)
            .unwrap_or_else(|| url.to_string());

        // Convert f32 slice to bytes
        let embedding_bytes: Vec<u8> = embedding
            .iter()
            .flat_map(|f| f.to_le_bytes())
            .collect();

        sqlx::query(
            r#"
            INSERT INTO embeddings (url, site_url, embedding)
            VALUES (?, ?, ?)
            ON CONFLICT(url) DO UPDATE SET
                site_url = excluded.site_url,
                embedding = excluded.embedding
            "#,
        )
        .bind(url)
        .bind(&site_url)
        .bind(&embedding_bytes)
        .execute(&self.pool)
        .await
        .map_err(|e| ExtractionError::Storage(e.to_string().into()))?;

        Ok(())
    }

    async fn get_embedding(&self, url: &str) -> Result<Option<Vec<f32>>> {
        let row: Option<(Vec<u8>,)> =
            sqlx::query_as("SELECT embedding FROM embeddings WHERE url = ?")
                .bind(url)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| ExtractionError::Storage(e.to_string().into()))?;

        match row {
            Some((bytes,)) => {
                let embedding: Vec<f32> = bytes
                    .chunks_exact(4)
                    .map(|chunk| {
                        let arr: [u8; 4] = chunk.try_into().unwrap();
                        f32::from_le_bytes(arr)
                    })
                    .collect();
                Ok(Some(embedding))
            }
            None => Ok(None),
        }
    }

    async fn search_similar(
        &self,
        query_embedding: &[f32],
        limit: usize,
        filter: Option<&QueryFilter>,
    ) -> Result<Vec<PageRef>> {
        // SQLite doesn't have native vector similarity, so we compute in Rust
        let rows = match filter {
            Some(f) if !f.include_sites.is_empty() => {
                let placeholders = f.include_sites.iter().map(|_| "?").collect::<Vec<_>>().join(",");
                let query = format!(
                    "SELECT e.url, e.site_url, e.embedding, p.title FROM embeddings e LEFT JOIN pages p ON e.url = p.url WHERE e.site_url IN ({})",
                    placeholders
                );

                let mut q = sqlx::query_as::<_, (String, String, Vec<u8>, Option<String>)>(&query);
                for url in &f.include_sites {
                    q = q.bind(url);
                }
                q.fetch_all(&self.pool)
                    .await
                    .map_err(|e| ExtractionError::Storage(e.to_string().into()))?
            }
            _ => {
                sqlx::query_as::<_, (String, String, Vec<u8>, Option<String>)>(
                    "SELECT e.url, e.site_url, e.embedding, p.title FROM embeddings e LEFT JOIN pages p ON e.url = p.url",
                )
                .fetch_all(&self.pool)
                .await
                .map_err(|e| ExtractionError::Storage(e.to_string().into()))?
            }
        };

        // Calculate similarities
        let mut results: Vec<PageRef> = rows
            .into_iter()
            .map(|(url, site_url, emb_bytes, title)| {
                let embedding: Vec<f32> = emb_bytes
                    .chunks_exact(4)
                    .map(|chunk| {
                        let arr: [u8; 4] = chunk.try_into().unwrap();
                        f32::from_le_bytes(arr)
                    })
                    .collect();

                let score = crate::traits::store::cosine_similarity(query_embedding, &embedding);

                PageRef {
                    url,
                    title,
                    site_url,
                    score,
                }
            })
            .collect();

        // Sort by score descending
        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        results.truncate(limit);

        Ok(results)
    }

    async fn delete_embedding(&self, url: &str) -> Result<()> {
        sqlx::query("DELETE FROM embeddings WHERE url = ?")
            .bind(url)
            .execute(&self.pool)
            .await
            .map_err(|e| ExtractionError::Storage(e.to_string().into()))?;

        Ok(())
    }
}

#[async_trait]
impl KeywordSearch for SqliteStore {
    async fn keyword_search(
        &self,
        query: &str,
        limit: usize,
        filter: Option<&QueryFilter>,
    ) -> Result<Vec<PageRef>> {
        // Use FTS5 for keyword search
        let rows = match filter {
            Some(f) if !f.include_sites.is_empty() => {
                let placeholders = f.include_sites.iter().map(|_| "?").collect::<Vec<_>>().join(",");
                let query_str = format!(
                    r#"
                    SELECT p.url, p.site_url, p.title, bm25(pages_fts) as score
                    FROM pages_fts f
                    JOIN pages p ON f.url = p.url
                    WHERE pages_fts MATCH ? AND p.site_url IN ({})
                    ORDER BY score
                    LIMIT ?
                    "#,
                    placeholders
                );

                let mut q = sqlx::query_as::<_, (String, String, Option<String>, f64)>(&query_str);
                q = q.bind(query);
                for url in &f.include_sites {
                    q = q.bind(url);
                }
                q = q.bind(limit as i64);
                q.fetch_all(&self.pool).await.unwrap_or_default()
            }
            _ => {
                sqlx::query_as::<_, (String, String, Option<String>, f64)>(
                    r#"
                    SELECT p.url, p.site_url, p.title, bm25(pages_fts) as score
                    FROM pages_fts f
                    JOIN pages p ON f.url = p.url
                    WHERE pages_fts MATCH ?
                    ORDER BY score
                    LIMIT ?
                    "#,
                )
                .bind(query)
                .bind(limit as i64)
                .fetch_all(&self.pool)
                .await
                .unwrap_or_default()
            }
        };

        Ok(rows
            .into_iter()
            .map(|(url, site_url, title, score)| PageRef {
                url,
                title,
                site_url,
                score: (-score as f32).min(1.0).max(0.0), // BM25 returns negative scores
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn test_store() -> SqliteStore {
        SqliteStore::in_memory().await.unwrap()
    }

    #[tokio::test]
    async fn test_page_storage() {
        let store = test_store().await;
        let page = CachedPage::new(
            "https://example.com/page1",
            "https://example.com",
            "Test content",
        );

        store.store_page(&page).await.unwrap();

        let retrieved = store.get_page("https://example.com/page1").await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().content, "Test content");
    }

    #[tokio::test]
    async fn test_summary_storage() {
        let store = test_store().await;
        let summary = Summary::new(
            "https://example.com/page1",
            "https://example.com",
            "This is a summary",
            "content_hash_123",
            "prompt_hash_456",
        );

        store.store_summary(&summary).await.unwrap();

        let retrieved = store
            .get_summary("https://example.com/page1", "content_hash_123")
            .await
            .unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().text, "This is a summary");

        // Different content hash should return None
        let stale = store
            .get_summary("https://example.com/page1", "different_hash")
            .await
            .unwrap();
        assert!(stale.is_none());
    }

    #[tokio::test]
    async fn test_embedding_storage() {
        let store = test_store().await;

        // First store a page so we have site_url
        let page = CachedPage::new(
            "https://example.com/page1",
            "https://example.com",
            "Content",
        );
        store.store_page(&page).await.unwrap();

        let embedding = vec![0.1, 0.2, 0.3, 0.4];
        store
            .store_embedding("https://example.com/page1", &embedding)
            .await
            .unwrap();

        let retrieved = store
            .get_embedding("https://example.com/page1")
            .await
            .unwrap();
        assert!(retrieved.is_some());

        let retrieved_emb = retrieved.unwrap();
        assert_eq!(retrieved_emb.len(), 4);
        assert!((retrieved_emb[0] - 0.1).abs() < 0.001);
    }

    #[tokio::test]
    async fn test_similarity_search() {
        let store = test_store().await;

        // Store pages and embeddings
        for i in 0..5 {
            let page = CachedPage::new(
                format!("https://example.com/page{}", i),
                "https://example.com",
                format!("Content {}", i),
            );
            store.store_page(&page).await.unwrap();

            let embedding = vec![i as f32 * 0.1, 0.5, 0.5, 0.5];
            store
                .store_embedding(&format!("https://example.com/page{}", i), &embedding)
                .await
                .unwrap();
        }

        // Search for similar to page 4
        let query_embedding = vec![0.4, 0.5, 0.5, 0.5];
        let results = store
            .search_similar(&query_embedding, 3, None)
            .await
            .unwrap();

        assert_eq!(results.len(), 3);
        // Page 4 should be most similar
        assert_eq!(results[0].url, "https://example.com/page4");
    }
}
