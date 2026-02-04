//! PostgreSQL storage implementation.
//!
//! A production-ready storage backend using PostgreSQL. Good for:
//! - Multi-server deployments
//! - High-volume workloads
//! - Production environments with pgvector for native vector search
//!
//! # Features
//!
//! - **Hybrid Search (RRF)**: Combines semantic and keyword search using Reciprocal Rank Fusion
//! - **HNSW Indexes**: Production-grade vector search for 10M+ page scale
//! - **Investigation Logging**: Tracks detective loop activity for debugging/auditing
//! - **Versioned Migrations**: Safely evolves schema without data loss

use async_trait::async_trait;
use sha2::{Digest, Sha256};
use sqlx::postgres::{PgPool, PgPoolOptions};
use sqlx::FromRow;
use tracing::{debug, info, instrument, warn};
use uuid::Uuid;

use crate::error::{ExtractionError, Result};
use crate::traits::store::{EmbeddingStore, KeywordSearch, PageCache, SummaryCache};
use crate::types::{
    config::QueryFilter,
    extraction::GroundingGrade,
    investigation::InvestigationStep,
    page::{CachedPage, PageRef},
    signals::ExtractedSignal,
    summary::{RecallSignals, Summary},
};

/// Default RRF constant (k=60) from the original RRF paper.
/// Prevents low-ranked results from vanishing while balancing result quality.
/// Tune this if keyword results are drowning out semantic results (or vice versa).
pub const DEFAULT_RRF_K: f32 = 60.0;

/// PostgreSQL-based page store.
///
/// Supports native vector search via pgvector extension and hybrid search
/// combining semantic and keyword search with Reciprocal Rank Fusion.
pub struct PostgresStore {
    pool: PgPool,
    has_pgvector: bool,
    has_hnsw: bool,
    /// RRF constant for hybrid search (default: 60.0)
    pub rrf_k: f32,
}

impl PostgresStore {
    /// Create a new PostgreSQL store with the given connection URL.
    ///
    /// # Example URL
    /// `postgres://user:password@localhost/extraction`
    pub async fn new(database_url: &str) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(10)
            .connect(database_url)
            .await
            .map_err(|e| ExtractionError::Storage(e.to_string().into()))?;

        Self::from_pool(pool).await
    }

    /// Create a PostgreSQL store from an existing connection pool.
    ///
    /// Use this when your application already has a connection pool (e.g., from
    /// the server's `PgPool`). This avoids creating duplicate connections.
    ///
    /// # Example
    /// ```rust,ignore
    /// use extraction::PostgresStore;
    /// use sqlx::PgPool;
    ///
    /// // Reuse server's pool
    /// let store = PostgresStore::from_pool(server_pool.clone()).await?;
    /// ```
    pub async fn from_pool(pool: PgPool) -> Result<Self> {
        let mut store = Self {
            pool,
            has_pgvector: false,
            has_hnsw: false,
            rrf_k: DEFAULT_RRF_K,
        };
        store.detect_capabilities().await?;
        store.run_migrations().await?;
        Ok(store)
    }

    /// Create with a custom RRF constant for hybrid search tuning.
    pub async fn with_rrf_k(database_url: &str, rrf_k: f32) -> Result<Self> {
        let mut store = Self::new(database_url).await?;
        store.rrf_k = rrf_k;
        Ok(store)
    }

    /// Detect pgvector and HNSW capabilities.
    async fn detect_capabilities(&mut self) -> Result<()> {
        // Check for pgvector extension
        let pgvector_check: Option<(String,)> =
            sqlx::query_as("SELECT extname FROM pg_extension WHERE extname = 'vector'")
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| ExtractionError::Storage(e.to_string().into()))?;

        self.has_pgvector = pgvector_check.is_some();

        // Check pgvector version for HNSW support (0.5.0+)
        if self.has_pgvector {
            let version: Option<(String,)> =
                sqlx::query_as("SELECT extversion FROM pg_extension WHERE extname = 'vector'")
                    .fetch_optional(&self.pool)
                    .await
                    .map_err(|e| ExtractionError::Storage(e.to_string().into()))?;

            if let Some((ver,)) = version {
                self.has_hnsw = ver >= "0.5.0".to_string();
            }
        }

        Ok(())
    }

    /// Run database migrations (base schema).
    ///
    /// Note: `detect_capabilities()` must be called before this to set
    /// `has_pgvector` and `has_hnsw` flags correctly.
    async fn run_migrations(&mut self) -> Result<()> {
        // Create pages table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS extraction_pages (
                url TEXT PRIMARY KEY,
                site_url TEXT NOT NULL,
                content TEXT NOT NULL,
                content_hash TEXT NOT NULL,
                fetched_at TIMESTAMPTZ NOT NULL,
                title TEXT,
                http_headers JSONB NOT NULL DEFAULT '{}',
                metadata JSONB NOT NULL DEFAULT '{}'
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| ExtractionError::Storage(e.to_string().into()))?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_extraction_pages_site_url ON extraction_pages(site_url)")
            .execute(&self.pool)
            .await
            .ok();

        // Create summaries table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS extraction_summaries (
                url TEXT PRIMARY KEY,
                site_url TEXT NOT NULL,
                text TEXT NOT NULL,
                signals JSONB NOT NULL DEFAULT '{}',
                language TEXT,
                created_at TIMESTAMPTZ NOT NULL,
                prompt_hash TEXT NOT NULL,
                content_hash TEXT NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| ExtractionError::Storage(e.to_string().into()))?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_extraction_summaries_site_url ON extraction_summaries(site_url)")
            .execute(&self.pool)
            .await
            .ok();

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_extraction_summaries_prompt_hash ON extraction_summaries(prompt_hash)")
            .execute(&self.pool)
            .await
            .ok();

        // Create embeddings table
        if self.has_pgvector {
            // Use native vector type
            sqlx::query(
                r#"
                CREATE TABLE IF NOT EXISTS extraction_embeddings (
                    url TEXT PRIMARY KEY,
                    site_url TEXT NOT NULL,
                    embedding vector(1536)
                )
                "#,
            )
            .execute(&self.pool)
            .await
            .map_err(|e| ExtractionError::Storage(e.to_string().into()))?;

            // Create vector index for fast similarity search
            // Use HNSW if available (0.5.0+), otherwise IVFFLAT
            if self.has_hnsw {
                sqlx::query(
                    r#"
                    CREATE INDEX IF NOT EXISTS idx_extraction_embeddings_hnsw
                    ON extraction_embeddings USING hnsw (embedding vector_cosine_ops)
                    WITH (m = 24, ef_construction = 128)
                    "#,
                )
                .execute(&self.pool)
                .await
                .ok();
            } else {
                sqlx::query(
                    r#"
                    CREATE INDEX IF NOT EXISTS idx_extraction_embeddings_vector
                    ON extraction_embeddings USING ivfflat (embedding vector_cosine_ops)
                    WITH (lists = 100)
                    "#,
                )
                .execute(&self.pool)
                .await
                .ok();
            }
        } else {
            // Fallback: store as BYTEA
            sqlx::query(
                r#"
                CREATE TABLE IF NOT EXISTS extraction_embeddings (
                    url TEXT PRIMARY KEY,
                    site_url TEXT NOT NULL,
                    embedding BYTEA NOT NULL
                )
                "#,
            )
            .execute(&self.pool)
            .await
            .map_err(|e| ExtractionError::Storage(e.to_string().into()))?;
        }

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_extraction_embeddings_site_url ON extraction_embeddings(site_url)")
            .execute(&self.pool)
            .await
            .ok();

        // Create text search index for keyword search
        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_extraction_pages_content_tsvector
            ON extraction_pages USING gin(to_tsvector('english', content))
            "#,
        )
        .execute(&self.pool)
        .await
        .ok();

        Ok(())
    }

    /// Run additional migrations from SQL files.
    ///
    /// This runs the Detective Engine schema migrations.
    /// Call this after `new()` to enable investigation tracking.
    ///
    /// # Example
    /// ```rust,ignore
    /// let store = PostgresStore::new("postgres://...").await?;
    /// store.run_detective_migrations().await?;
    /// ```
    pub async fn run_detective_migrations(&self) -> Result<()> {
        // Create migration tracking table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS extraction_migrations (
                name TEXT PRIMARY KEY,
                applied_at TIMESTAMPTZ DEFAULT NOW()
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| ExtractionError::Storage(e.to_string().into()))?;

        // Run each migration if not already applied
        let migrations = [
            (
                "001_normalize_signals",
                include_str!("../../migrations/001_normalize_signals.sql"),
            ),
            (
                "002_hnsw_index",
                include_str!("../../migrations/002_hnsw_index.sql"),
            ),
            (
                "003_multilang_fts",
                include_str!("../../migrations/003_multilang_fts.sql"),
            ),
            (
                "004_investigation_tables",
                include_str!("../../migrations/004_investigation_tables.sql"),
            ),
            (
                "005_generalize_signal_type",
                include_str!("../../migrations/005_generalize_signal_type.sql"),
            ),
        ];

        for (name, sql) in migrations {
            let applied: Option<(String,)> =
                sqlx::query_as("SELECT name FROM extraction_migrations WHERE name = $1")
                    .bind(name)
                    .fetch_optional(&self.pool)
                    .await
                    .map_err(|e| ExtractionError::Storage(e.to_string().into()))?;

            if applied.is_none() {
                // Run the migration
                sqlx::raw_sql(sql).execute(&self.pool).await.map_err(|e| {
                    ExtractionError::Storage(format!("Migration {} failed: {}", name, e).into())
                })?;

                // Mark as applied
                sqlx::query("INSERT INTO extraction_migrations (name) VALUES ($1)")
                    .bind(name)
                    .execute(&self.pool)
                    .await
                    .map_err(|e| ExtractionError::Storage(e.to_string().into()))?;
            }
        }

        Ok(())
    }

    /// Check if pgvector extension is available.
    pub fn has_pgvector(&self) -> bool {
        self.has_pgvector
    }

    /// Check if HNSW indexes are available (pgvector 0.5.0+).
    pub fn has_hnsw(&self) -> bool {
        self.has_hnsw
    }

    /// Get the underlying connection pool.
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    // =========================================================================
    // Hybrid Search (RRF)
    // =========================================================================

    /// Hybrid search combining semantic and keyword search with Reciprocal Rank Fusion.
    ///
    /// # Arguments
    /// * `query` - The search query
    /// * `query_embedding` - Pre-computed embedding for semantic search
    /// * `limit` - Maximum results to return
    /// * `filter` - Optional site filter
    /// * `semantic_weight` - Balance between semantic (1.0) and keyword (0.0) search
    ///
    /// # RRF Formula
    /// `score = weight/(k + semantic_rank) + (1-weight)/(k + keyword_rank)`
    ///
    /// Where k is the RRF constant (default 60.0).
    #[instrument(skip(self, query_embedding, filter), fields(query = %query, limit = limit, semantic_weight = semantic_weight))]
    pub async fn hybrid_search_rrf(
        &self,
        query: &str,
        query_embedding: &[f32],
        limit: usize,
        filter: Option<&QueryFilter>,
        semantic_weight: f32,
    ) -> Result<Vec<PageRef>> {
        let filter_sites = filter.map(|f| f.include_sites.len()).unwrap_or(0);
        debug!(
            query = %query,
            limit = limit,
            semantic_weight = semantic_weight,
            filter_sites = filter_sites,
            has_pgvector = self.has_pgvector,
            "Starting hybrid RRF search"
        );

        if !self.has_pgvector {
            // Fallback to keyword-only search
            warn!("pgvector not available, falling back to keyword-only search");
            return self.keyword_search(query, limit, filter).await;
        }

        let vector_str = format!(
            "[{}]",
            query_embedding
                .iter()
                .map(|f| f.to_string())
                .collect::<Vec<_>>()
                .join(",")
        );

        let k = self.rrf_k;
        let semantic_w = semantic_weight.clamp(0.0, 1.0);
        let keyword_w = 1.0 - semantic_w;
        debug!(
            rrf_k = k,
            semantic_w = semantic_w,
            keyword_w = keyword_w,
            "RRF parameters"
        );

        // Use a CTE-based RRF query
        let rows = match filter {
            Some(f) if !f.include_sites.is_empty() => {
                sqlx::query_as::<_, (String, String, Option<String>, f64)>(
                    r#"
                    WITH vector_results AS (
                        SELECT e.url, e.site_url, p.title,
                               ROW_NUMBER() OVER (ORDER BY e.embedding <=> $1::vector) AS rank
                        FROM extraction_embeddings e
                        LEFT JOIN extraction_pages p ON e.url = p.url
                        WHERE e.site_url = ANY($4)
                        LIMIT 100
                    ),
                    fts_results AS (
                        SELECT url, site_url, title,
                               ROW_NUMBER() OVER (ORDER BY ts_rank(to_tsvector('english', content), plainto_tsquery('english', $2)) DESC) AS rank
                        FROM extraction_pages
                        WHERE to_tsvector('english', content) @@ plainto_tsquery('english', $2)
                          AND site_url = ANY($4)
                        LIMIT 100
                    )
                    SELECT COALESCE(v.url, f.url) AS url,
                           COALESCE(v.site_url, f.site_url) AS site_url,
                           COALESCE(v.title, f.title) AS title,
                           COALESCE($5::float / ($3::float + v.rank), 0) +
                           COALESCE($6::float / ($3::float + f.rank), 0) AS score
                    FROM vector_results v
                    FULL OUTER JOIN fts_results f ON v.url = f.url
                    ORDER BY score DESC
                    LIMIT $7
                    "#,
                )
                .bind(&vector_str)
                .bind(query)
                .bind(k as f64)
                .bind(&f.include_sites)
                .bind(semantic_w as f64)
                .bind(keyword_w as f64)
                .bind(limit as i64)
                .fetch_all(&self.pool)
                .await
                .map_err(|e| ExtractionError::Storage(e.to_string().into()))?
            }
            _ => {
                sqlx::query_as::<_, (String, String, Option<String>, f64)>(
                    r#"
                    WITH vector_results AS (
                        SELECT e.url, e.site_url, p.title,
                               ROW_NUMBER() OVER (ORDER BY e.embedding <=> $1::vector) AS rank
                        FROM extraction_embeddings e
                        LEFT JOIN extraction_pages p ON e.url = p.url
                        LIMIT 100
                    ),
                    fts_results AS (
                        SELECT url, site_url, title,
                               ROW_NUMBER() OVER (ORDER BY ts_rank(to_tsvector('english', content), plainto_tsquery('english', $2)) DESC) AS rank
                        FROM extraction_pages
                        WHERE to_tsvector('english', content) @@ plainto_tsquery('english', $2)
                        LIMIT 100
                    )
                    SELECT COALESCE(v.url, f.url) AS url,
                           COALESCE(v.site_url, f.site_url) AS site_url,
                           COALESCE(v.title, f.title) AS title,
                           COALESCE($4::float / ($3::float + v.rank), 0) +
                           COALESCE($5::float / ($3::float + f.rank), 0) AS score
                    FROM vector_results v
                    FULL OUTER JOIN fts_results f ON v.url = f.url
                    ORDER BY score DESC
                    LIMIT $6
                    "#,
                )
                .bind(&vector_str)
                .bind(query)
                .bind(k as f64)
                .bind(semantic_w as f64)
                .bind(keyword_w as f64)
                .bind(limit as i64)
                .fetch_all(&self.pool)
                .await
                .map_err(|e| ExtractionError::Storage(e.to_string().into()))?
            }
        };

        let results: Vec<PageRef> = rows
            .into_iter()
            .map(|(url, site_url, title, score)| PageRef {
                url,
                title,
                site_url,
                score: score as f32,
            })
            .collect();

        debug!(
            query = %query,
            result_count = results.len(),
            top_score = results.first().map(|r| r.score).unwrap_or(0.0),
            "Hybrid RRF search completed"
        );
        Ok(results)
    }

    // =========================================================================
    // Investigation Tracking (Detective Engine)
    // =========================================================================

    /// Create a new extraction job for tracking.
    ///
    /// Returns the job ID for use with gap and log tracking.
    pub async fn create_job(&self, query: &str, strategy: &str) -> Result<Uuid> {
        let job_id = Uuid::new_v4();
        let query_hash = Self::hash_query(query);

        sqlx::query(
            r#"
            INSERT INTO extraction_jobs (id, query, query_hash, strategy)
            VALUES ($1, $2, $3, $4)
            "#,
        )
        .bind(job_id)
        .bind(query)
        .bind(&query_hash)
        .bind(strategy)
        .execute(&self.pool)
        .await
        .map_err(|e| ExtractionError::Storage(e.to_string().into()))?;

        Ok(job_id)
    }

    /// Update job completion status.
    pub async fn complete_job(
        &self,
        job_id: Uuid,
        grounding: GroundingGrade,
        has_gaps: bool,
        tokens_used: i32,
    ) -> Result<()> {
        let grounding_str = match grounding {
            GroundingGrade::Verified => "verified",
            GroundingGrade::SingleSource => "single_source",
            GroundingGrade::Conflicted => "conflicted",
            GroundingGrade::Inferred => "inferred",
        };

        sqlx::query(
            r#"
            UPDATE extraction_jobs
            SET grounding = $2::grounding_grade,
                has_gaps = $3,
                tokens_used = $4,
                completed_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(job_id)
        .bind(grounding_str)
        .bind(has_gaps)
        .bind(tokens_used)
        .execute(&self.pool)
        .await
        .map_err(|e| ExtractionError::Storage(e.to_string().into()))?;

        Ok(())
    }

    /// Record a gap detected during extraction.
    pub async fn record_gap(
        &self,
        job_id: Uuid,
        field: &str,
        query: &str,
        gap_type: Option<&str>,
        parent_gap_id: Option<Uuid>,
        depth: i32,
    ) -> Result<Uuid> {
        let gap_id = Uuid::new_v4();

        sqlx::query(
            r#"
            INSERT INTO extraction_gaps (id, job_id, parent_gap_id, depth, field, query, gap_type)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(gap_id)
        .bind(job_id)
        .bind(parent_gap_id)
        .bind(depth)
        .bind(field)
        .bind(query)
        .bind(gap_type)
        .execute(&self.pool)
        .await
        .map_err(|e| ExtractionError::Storage(e.to_string().into()))?;

        Ok(gap_id)
    }

    /// Mark a gap as resolved.
    pub async fn resolve_gap(&self, gap_id: Uuid, resolution_source: &str) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE extraction_gaps
            SET status = 'resolved',
                resolved_at = NOW(),
                resolution_source = $2
            WHERE id = $1
            "#,
        )
        .bind(gap_id)
        .bind(resolution_source)
        .execute(&self.pool)
        .await
        .map_err(|e| ExtractionError::Storage(e.to_string().into()))?;

        Ok(())
    }

    /// Log an investigation attempt (for auditing/debugging).
    pub async fn log_investigation(
        &self,
        gap_id: Uuid,
        step: &InvestigationStep,
        pages_found: i32,
        tokens_used: i32,
        duration_ms: Option<i32>,
    ) -> Result<()> {
        let action_params = serde_json::to_value(&step.recommended_action)
            .map_err(|e| ExtractionError::Storage(e.to_string().into()))?;

        sqlx::query(
            r#"
            INSERT INTO extraction_investigation_logs
                (gap_id, action_type, action_params, pages_found, tokens_used, duration_ms)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
        )
        .bind(gap_id)
        .bind(step.recommended_action.action_type())
        .bind(&action_params)
        .bind(pages_found)
        .bind(tokens_used)
        .bind(duration_ms)
        .execute(&self.pool)
        .await
        .map_err(|e| ExtractionError::Storage(e.to_string().into()))?;

        Ok(())
    }

    /// Extend gap expiration when actively investigating (prevents garbage collection).
    pub async fn extend_gap_expiration(&self, gap_id: Uuid, hours: i32) -> Result<()> {
        sqlx::query("SELECT extend_gap_expiration($1, $2)")
            .bind(gap_id)
            .bind(hours)
            .execute(&self.pool)
            .await
            .map_err(|e| ExtractionError::Storage(e.to_string().into()))?;

        Ok(())
    }

    /// Clean up expired gaps and cache entries.
    ///
    /// Returns the number of gaps marked as abandoned.
    /// Call this periodically to prevent orphaned gaps from failed jobs.
    pub async fn cleanup_expired_gaps(&self) -> Result<usize> {
        let result: (i32,) = sqlx::query_as("SELECT cleanup_expired_gaps()")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| ExtractionError::Storage(e.to_string().into()))?;

        Ok(result.0 as usize)
    }

    // =========================================================================
    // Structured Signals (Normalized Table)
    // =========================================================================

    /// Store extracted signals in the normalized table.
    ///
    /// This writes directly to `extraction_signals` (not the JSONB column),
    /// providing better queryability and performance.
    ///
    /// # Example
    /// ```rust,ignore
    /// let signals = vec![
    ///     ExtractedSignal::new("product", "iPhone 15")
    ///         .with_source(summary_id)
    ///         .with_confidence(0.95),
    ///     ExtractedSignal::new("price", "$999")
    ///         .with_source(summary_id),
    /// ];
    /// store.store_signals("https://example.com/page", &signals).await?;
    /// ```
    pub async fn store_signals(
        &self,
        summary_url: &str,
        signals: &[ExtractedSignal],
    ) -> Result<()> {
        // Clear existing signals for this summary
        sqlx::query("DELETE FROM extraction_signals WHERE summary_url = $1")
            .bind(summary_url)
            .execute(&self.pool)
            .await
            .map_err(|e| ExtractionError::Storage(e.to_string().into()))?;

        // Insert new signals
        for signal in signals {
            let tags: Option<Vec<&str>> = if signal.tags.is_empty() {
                None
            } else {
                Some(signal.tags.iter().map(|s| s.as_str()).collect())
            };

            sqlx::query(
                r#"
                INSERT INTO extraction_signals
                    (summary_url, signal_type, value, subtype, source_id, confidence, context_snippet, group_id, tags)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                "#,
            )
            .bind(summary_url)
            .bind(&signal.signal_type)
            .bind(&signal.value)
            .bind(&signal.subtype)
            .bind(signal.source_id)
            .bind(signal.confidence)
            .bind(&signal.context_snippet)
            .bind(signal.group_id)
            .bind(&tags)
            .execute(&self.pool)
            .await
            .map_err(|e| ExtractionError::Storage(e.to_string().into()))?;
        }

        Ok(())
    }

    /// Get signals for a summary URL.
    pub async fn get_signals(&self, summary_url: &str) -> Result<Vec<ExtractedSignal>> {
        let rows = sqlx::query_as::<_, SignalRow>(
            r#"
            SELECT signal_type, value, subtype, source_id, confidence, context_snippet, group_id, tags
            FROM extraction_signals
            WHERE summary_url = $1
            "#,
        )
        .bind(summary_url)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ExtractionError::Storage(e.to_string().into()))?;

        Ok(rows.into_iter().map(|r| r.into_signal()).collect())
    }

    /// Get signals by type across all summaries.
    pub async fn get_signals_by_type(
        &self,
        signal_type: &str,
        limit: usize,
        min_confidence: Option<f32>,
    ) -> Result<Vec<ExtractedSignal>> {
        let min_conf = min_confidence.unwrap_or(0.0);

        let rows = sqlx::query_as::<_, SignalRow>(
            r#"
            SELECT signal_type, value, subtype, source_id, confidence, context_snippet, group_id, tags
            FROM extraction_signals
            WHERE signal_type = $1 AND confidence >= $2
            ORDER BY confidence DESC
            LIMIT $3
            "#,
        )
        .bind(signal_type)
        .bind(min_conf)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ExtractionError::Storage(e.to_string().into()))?;

        Ok(rows.into_iter().map(|r| r.into_signal()).collect())
    }

    /// Get pending gaps for a job (for orchestrator to process).
    pub async fn get_pending_gaps(&self, job_id: Uuid) -> Result<Vec<GapRecord>> {
        let rows = sqlx::query_as::<_, GapRow>(
            r#"
            SELECT id, job_id, parent_gap_id, depth, field, query, gap_type, status
            FROM extraction_gaps
            WHERE job_id = $1 AND status = 'pending'
            ORDER BY depth ASC, created_at ASC
            "#,
        )
        .bind(job_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ExtractionError::Storage(e.to_string().into()))?;

        Ok(rows.into_iter().map(|r| r.into_record()).collect())
    }

    /// Hash a query for caching/deduplication.
    fn hash_query(query: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(query.as_bytes());
        format!("{:x}", hasher.finalize())
    }
}

// Row types
#[derive(Debug, FromRow)]
struct PageRow {
    url: String,
    site_url: String,
    content: String,
    content_hash: String,
    fetched_at: chrono::DateTime<chrono::Utc>,
    title: Option<String>,
    http_headers: serde_json::Value,
    metadata: serde_json::Value,
}

impl PageRow {
    fn into_cached_page(self) -> Result<CachedPage> {
        let http_headers: std::collections::HashMap<String, String> =
            serde_json::from_value(self.http_headers)
                .map_err(|e| ExtractionError::Storage(format!("Invalid headers: {}", e).into()))?;

        let metadata: std::collections::HashMap<String, String> =
            serde_json::from_value(self.metadata)
                .map_err(|e| ExtractionError::Storage(format!("Invalid metadata: {}", e).into()))?;

        Ok(CachedPage {
            url: self.url,
            site_url: self.site_url,
            content: self.content,
            content_hash: self.content_hash,
            fetched_at: self.fetched_at,
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
    signals: serde_json::Value,
    language: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
    prompt_hash: String,
    content_hash: String,
}

impl SummaryRow {
    fn into_summary(self) -> Result<Summary> {
        let signals: RecallSignals = serde_json::from_value(self.signals)
            .map_err(|e| ExtractionError::Storage(format!("Invalid signals: {}", e).into()))?;

        Ok(Summary {
            url: self.url,
            site_url: self.site_url,
            text: self.text,
            signals,
            language: self.language,
            created_at: self.created_at,
            prompt_hash: self.prompt_hash,
            content_hash: self.content_hash,
            embedding: None,
        })
    }
}

#[derive(Debug, FromRow)]
struct GapRow {
    id: Uuid,
    job_id: Uuid,
    parent_gap_id: Option<Uuid>,
    depth: i32,
    field: String,
    query: String,
    gap_type: Option<String>,
    status: String,
}

impl GapRow {
    fn into_record(self) -> GapRecord {
        GapRecord {
            id: self.id,
            job_id: self.job_id,
            parent_gap_id: self.parent_gap_id,
            depth: self.depth,
            field: self.field,
            query: self.query,
            gap_type: self.gap_type,
            status: self.status,
        }
    }
}

#[derive(Debug, FromRow)]
struct SignalRow {
    signal_type: String,
    value: String,
    subtype: Option<String>,
    source_id: Option<Uuid>,
    confidence: f32,
    context_snippet: Option<String>,
    group_id: Option<Uuid>,
    tags: Option<Vec<String>>,
}

impl SignalRow {
    fn into_signal(self) -> ExtractedSignal {
        ExtractedSignal {
            signal_type: self.signal_type,
            value: self.value,
            subtype: self.subtype,
            source_id: self.source_id,
            confidence: self.confidence,
            context_snippet: self.context_snippet,
            group_id: self.group_id,
            tags: self.tags.unwrap_or_default(),
        }
    }
}

/// A gap record from the database.
#[derive(Debug, Clone)]
pub struct GapRecord {
    pub id: Uuid,
    pub job_id: Uuid,
    pub parent_gap_id: Option<Uuid>,
    pub depth: i32,
    pub field: String,
    pub query: String,
    pub gap_type: Option<String>,
    pub status: String,
}

#[async_trait]
impl PageCache for PostgresStore {
    #[instrument(skip(self), fields(url = %url))]
    async fn get_page(&self, url: &str) -> Result<Option<CachedPage>> {
        debug!(url = %url, "Getting page from cache");
        let row = sqlx::query_as::<_, PageRow>(
            "SELECT url, site_url, content, content_hash, fetched_at, title, http_headers, metadata FROM extraction_pages WHERE url = $1",
        )
        .bind(url)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ExtractionError::Storage(e.to_string().into()))?;

        match row {
            Some(r) => {
                debug!(url = %url, "Page found in cache");
                Ok(Some(r.into_cached_page()?))
            }
            None => {
                debug!(url = %url, "Page not in cache");
                Ok(None)
            }
        }
    }

    #[instrument(skip(self, page), fields(url = %page.url, content_len = page.content.len()))]
    async fn store_page(&self, page: &CachedPage) -> Result<()> {
        debug!(url = %page.url, content_len = page.content.len(), "Storing page");
        let http_headers = serde_json::to_value(&page.http_headers)
            .map_err(|e| ExtractionError::Storage(e.to_string().into()))?;
        let metadata = serde_json::to_value(&page.metadata)
            .map_err(|e| ExtractionError::Storage(e.to_string().into()))?;

        sqlx::query(
            r#"
            INSERT INTO extraction_pages (url, site_url, content, content_hash, fetched_at, title, http_headers, metadata)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT(url) DO UPDATE SET
                site_url = EXCLUDED.site_url,
                content = EXCLUDED.content,
                content_hash = EXCLUDED.content_hash,
                fetched_at = EXCLUDED.fetched_at,
                title = EXCLUDED.title,
                http_headers = EXCLUDED.http_headers,
                metadata = EXCLUDED.metadata
            "#,
        )
        .bind(&page.url)
        .bind(&page.site_url)
        .bind(&page.content)
        .bind(&page.content_hash)
        .bind(page.fetched_at)
        .bind(&page.title)
        .bind(&http_headers)
        .bind(&metadata)
        .execute(&self.pool)
        .await
        .map_err(|e| ExtractionError::Storage(e.to_string().into()))?;

        Ok(())
    }

    #[instrument(skip(self), fields(site_url = %site_url))]
    async fn get_pages_for_site(&self, site_url: &str) -> Result<Vec<CachedPage>> {
        debug!(site_url = %site_url, "Getting all pages for site");
        let rows = sqlx::query_as::<_, PageRow>(
            "SELECT url, site_url, content, content_hash, fetched_at, title, http_headers, metadata FROM extraction_pages WHERE site_url = $1",
        )
        .bind(site_url)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ExtractionError::Storage(e.to_string().into()))?;

        let count = rows.len();
        debug!(site_url = %site_url, page_count = count, "Retrieved pages for site");
        rows.into_iter().map(|r| r.into_cached_page()).collect()
    }

    #[instrument(skip(self), fields(url = %url))]
    async fn delete_page(&self, url: &str) -> Result<()> {
        debug!(url = %url, "Deleting page from cache");
        sqlx::query("DELETE FROM extraction_pages WHERE url = $1")
            .bind(url)
            .execute(&self.pool)
            .await
            .map_err(|e| ExtractionError::Storage(e.to_string().into()))?;

        info!(url = %url, "Deleted page from cache");
        Ok(())
    }

    #[instrument(skip(self), fields(site_url = %site_url))]
    async fn count_pages(&self, site_url: &str) -> Result<usize> {
        debug!(site_url = %site_url, "Counting pages for site");
        let count: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM extraction_pages WHERE site_url = $1")
                .bind(site_url)
                .fetch_one(&self.pool)
                .await
                .map_err(|e| ExtractionError::Storage(e.to_string().into()))?;

        let page_count = count.0 as usize;
        debug!(site_url = %site_url, page_count = page_count, "Page count for site");
        Ok(page_count)
    }
}

#[async_trait]
impl SummaryCache for PostgresStore {
    #[instrument(skip(self), fields(url = %url, content_hash = %content_hash))]
    async fn get_summary(&self, url: &str, content_hash: &str) -> Result<Option<Summary>> {
        debug!(url = %url, content_hash = %content_hash, "Getting summary from cache");
        let row = sqlx::query_as::<_, SummaryRow>(
            "SELECT url, site_url, text, signals, language, created_at, prompt_hash, content_hash FROM extraction_summaries WHERE url = $1 AND content_hash = $2",
        )
        .bind(url)
        .bind(content_hash)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ExtractionError::Storage(e.to_string().into()))?;

        match row {
            Some(r) => {
                debug!(url = %url, "Summary found in cache");
                Ok(Some(r.into_summary()?))
            }
            None => {
                debug!(url = %url, "Summary not in cache");
                Ok(None)
            }
        }
    }

    #[instrument(skip(self, summary), fields(url = %summary.url, summary_len = summary.text.len()))]
    async fn store_summary(&self, summary: &Summary) -> Result<()> {
        debug!(url = %summary.url, summary_len = summary.text.len(), "Storing summary");
        let signals = serde_json::to_value(&summary.signals)
            .map_err(|e| ExtractionError::Storage(e.to_string().into()))?;

        sqlx::query(
            r#"
            INSERT INTO extraction_summaries (url, site_url, text, signals, language, created_at, prompt_hash, content_hash)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT(url) DO UPDATE SET
                site_url = EXCLUDED.site_url,
                text = EXCLUDED.text,
                signals = EXCLUDED.signals,
                language = EXCLUDED.language,
                created_at = EXCLUDED.created_at,
                prompt_hash = EXCLUDED.prompt_hash,
                content_hash = EXCLUDED.content_hash
            "#,
        )
        .bind(&summary.url)
        .bind(&summary.site_url)
        .bind(&summary.text)
        .bind(&signals)
        .bind(&summary.language)
        .bind(summary.created_at)
        .bind(&summary.prompt_hash)
        .bind(&summary.content_hash)
        .execute(&self.pool)
        .await
        .map_err(|e| ExtractionError::Storage(e.to_string().into()))?;

        debug!(url = %summary.url, "Summary stored successfully");
        Ok(())
    }

    #[instrument(skip(self), fields(site_url = %site_url))]
    async fn get_summaries_for_site(&self, site_url: &str) -> Result<Vec<Summary>> {
        debug!(site_url = %site_url, "Getting all summaries for site");
        let rows = sqlx::query_as::<_, SummaryRow>(
            "SELECT url, site_url, text, signals, language, created_at, prompt_hash, content_hash FROM extraction_summaries WHERE site_url = $1",
        )
        .bind(site_url)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ExtractionError::Storage(e.to_string().into()))?;

        let count = rows.len();
        debug!(site_url = %site_url, summary_count = count, "Retrieved summaries for site");
        rows.into_iter().map(|r| r.into_summary()).collect()
    }

    #[instrument(skip(self, filter))]
    async fn get_summaries(&self, filter: Option<&QueryFilter>) -> Result<Vec<Summary>> {
        let filter_sites = filter.map(|f| f.include_sites.len()).unwrap_or(0);
        debug!(filter_sites = filter_sites, "Getting summaries with filter");

        let rows = match filter {
            Some(f) if !f.include_sites.is_empty() => {
                sqlx::query_as::<_, SummaryRow>(
                    "SELECT url, site_url, text, signals, language, created_at, prompt_hash, content_hash FROM extraction_summaries WHERE site_url = ANY($1)",
                )
                .bind(&f.include_sites)
                .fetch_all(&self.pool)
                .await
                .map_err(|e| ExtractionError::Storage(e.to_string().into()))?
            }
            _ => {
                sqlx::query_as::<_, SummaryRow>(
                    "SELECT url, site_url, text, signals, language, created_at, prompt_hash, content_hash FROM extraction_summaries",
                )
                .fetch_all(&self.pool)
                .await
                .map_err(|e| ExtractionError::Storage(e.to_string().into()))?
            }
        };

        let count = rows.len();
        debug!(summary_count = count, "Retrieved summaries");
        rows.into_iter().map(|r| r.into_summary()).collect()
    }

    #[instrument(skip(self), fields(url = %url))]
    async fn delete_summary(&self, url: &str) -> Result<()> {
        debug!(url = %url, "Deleting summary");
        sqlx::query("DELETE FROM extraction_summaries WHERE url = $1")
            .bind(url)
            .execute(&self.pool)
            .await
            .map_err(|e| ExtractionError::Storage(e.to_string().into()))?;

        info!(url = %url, "Deleted summary");
        Ok(())
    }

    #[instrument(skip(self), fields(current_prompt_hash = %current_prompt_hash))]
    async fn invalidate_stale_summaries(&self, current_prompt_hash: &str) -> Result<usize> {
        debug!(current_prompt_hash = %current_prompt_hash, "Invalidating stale summaries");
        let result = sqlx::query("DELETE FROM extraction_summaries WHERE prompt_hash != $1")
            .bind(current_prompt_hash)
            .execute(&self.pool)
            .await
            .map_err(|e| ExtractionError::Storage(e.to_string().into()))?;

        let deleted = result.rows_affected() as usize;
        info!(deleted_count = deleted, "Invalidated stale summaries");
        Ok(deleted)
    }
}

#[async_trait]
impl EmbeddingStore for PostgresStore {
    #[instrument(skip(self, embedding), fields(url = %url, embedding_dim = embedding.len()))]
    async fn store_embedding(&self, url: &str, embedding: &[f32]) -> Result<()> {
        debug!(url = %url, embedding_dim = embedding.len(), has_pgvector = self.has_pgvector, "Storing embedding");
        // Get site_url from pages table
        let site_url: Option<(String,)> =
            sqlx::query_as("SELECT site_url FROM extraction_pages WHERE url = $1")
                .bind(url)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| ExtractionError::Storage(e.to_string().into()))?;

        let site_url = site_url.map(|(s,)| s).unwrap_or_else(|| url.to_string());

        if self.has_pgvector {
            // Use native vector type
            let vector_str = format!(
                "[{}]",
                embedding
                    .iter()
                    .map(|f| f.to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            );

            sqlx::query(
                r#"
                INSERT INTO extraction_embeddings (url, site_url, embedding)
                VALUES ($1, $2, $3::vector)
                ON CONFLICT(url) DO UPDATE SET
                    site_url = EXCLUDED.site_url,
                    embedding = EXCLUDED.embedding
                "#,
            )
            .bind(url)
            .bind(&site_url)
            .bind(&vector_str)
            .execute(&self.pool)
            .await
            .map_err(|e| ExtractionError::Storage(e.to_string().into()))?;
        } else {
            // Fallback: store as BYTEA
            let embedding_bytes: Vec<u8> = embedding.iter().flat_map(|f| f.to_le_bytes()).collect();

            sqlx::query(
                r#"
                INSERT INTO extraction_embeddings (url, site_url, embedding)
                VALUES ($1, $2, $3)
                ON CONFLICT(url) DO UPDATE SET
                    site_url = EXCLUDED.site_url,
                    embedding = EXCLUDED.embedding
                "#,
            )
            .bind(url)
            .bind(&site_url)
            .bind(&embedding_bytes)
            .execute(&self.pool)
            .await
            .map_err(|e| ExtractionError::Storage(e.to_string().into()))?;
        }

        debug!(url = %url, "Embedding stored successfully");
        Ok(())
    }

    #[instrument(skip(self), fields(url = %url))]
    async fn get_embedding(&self, url: &str) -> Result<Option<Vec<f32>>> {
        debug!(url = %url, has_pgvector = self.has_pgvector, "Getting embedding");
        if self.has_pgvector {
            // Extract from vector type
            let row: Option<(String,)> =
                sqlx::query_as("SELECT embedding::text FROM extraction_embeddings WHERE url = $1")
                    .bind(url)
                    .fetch_optional(&self.pool)
                    .await
                    .map_err(|e| ExtractionError::Storage(e.to_string().into()))?;

            match row {
                Some((vector_str,)) => {
                    // Parse "[0.1,0.2,0.3]" format
                    let trimmed = vector_str.trim_start_matches('[').trim_end_matches(']');
                    let embedding: Vec<f32> = trimmed
                        .split(',')
                        .filter_map(|s| s.trim().parse().ok())
                        .collect();
                    debug!(url = %url, embedding_dim = embedding.len(), "Embedding found");
                    Ok(Some(embedding))
                }
                None => {
                    debug!(url = %url, "Embedding not found");
                    Ok(None)
                }
            }
        } else {
            // Extract from BYTEA
            let row: Option<(Vec<u8>,)> =
                sqlx::query_as("SELECT embedding FROM extraction_embeddings WHERE url = $1")
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
                    debug!(url = %url, embedding_dim = embedding.len(), "Embedding found (BYTEA)");
                    Ok(Some(embedding))
                }
                None => {
                    debug!(url = %url, "Embedding not found");
                    Ok(None)
                }
            }
        }
    }

    #[instrument(skip(self, query_embedding, filter), fields(limit = limit, embedding_dim = query_embedding.len()))]
    async fn search_similar(
        &self,
        query_embedding: &[f32],
        limit: usize,
        filter: Option<&QueryFilter>,
    ) -> Result<Vec<PageRef>> {
        let filter_sites = filter.map(|f| f.include_sites.len()).unwrap_or(0);
        debug!(
            limit = limit,
            filter_sites = filter_sites,
            has_pgvector = self.has_pgvector,
            "Searching similar embeddings"
        );

        if self.has_pgvector {
            // Use native vector similarity
            let vector_str = format!(
                "[{}]",
                query_embedding
                    .iter()
                    .map(|f| f.to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            );

            let rows = match filter {
                Some(f) if !f.include_sites.is_empty() => {
                    sqlx::query_as::<_, (String, String, Option<String>, f64)>(
                        r#"
                        SELECT e.url, e.site_url, p.title, 1 - (e.embedding <=> $1::vector) as score
                        FROM extraction_embeddings e
                        LEFT JOIN extraction_pages p ON e.url = p.url
                        WHERE e.site_url = ANY($2)
                        ORDER BY e.embedding <=> $1::vector
                        LIMIT $3
                        "#,
                    )
                    .bind(&vector_str)
                    .bind(&f.include_sites)
                    .bind(limit as i64)
                    .fetch_all(&self.pool)
                    .await
                    .map_err(|e| ExtractionError::Storage(e.to_string().into()))?
                }
                _ => sqlx::query_as::<_, (String, String, Option<String>, f64)>(
                    r#"
                        SELECT e.url, e.site_url, p.title, 1 - (e.embedding <=> $1::vector) as score
                        FROM extraction_embeddings e
                        LEFT JOIN extraction_pages p ON e.url = p.url
                        ORDER BY e.embedding <=> $1::vector
                        LIMIT $2
                        "#,
                )
                .bind(&vector_str)
                .bind(limit as i64)
                .fetch_all(&self.pool)
                .await
                .map_err(|e| ExtractionError::Storage(e.to_string().into()))?,
            };

            let results: Vec<PageRef> = rows
                .into_iter()
                .map(|(url, site_url, title, score)| PageRef {
                    url,
                    title,
                    site_url,
                    score: score as f32,
                })
                .collect();
            debug!(
                result_count = results.len(),
                "Semantic search completed (pgvector)"
            );
            Ok(results)
        } else {
            // Fallback: compute similarity in Rust
            debug!("Using BYTEA fallback for similarity search");
            let rows = match filter {
                Some(f) if !f.include_sites.is_empty() => {
                    sqlx::query_as::<_, (String, String, Vec<u8>, Option<String>)>(
                        r#"
                        SELECT e.url, e.site_url, e.embedding, p.title
                        FROM extraction_embeddings e
                        LEFT JOIN extraction_pages p ON e.url = p.url
                        WHERE e.site_url = ANY($1)
                        "#,
                    )
                    .bind(&f.include_sites)
                    .fetch_all(&self.pool)
                    .await
                    .map_err(|e| ExtractionError::Storage(e.to_string().into()))?
                }
                _ => sqlx::query_as::<_, (String, String, Vec<u8>, Option<String>)>(
                    r#"
                        SELECT e.url, e.site_url, e.embedding, p.title
                        FROM extraction_embeddings e
                        LEFT JOIN extraction_pages p ON e.url = p.url
                        "#,
                )
                .fetch_all(&self.pool)
                .await
                .map_err(|e| ExtractionError::Storage(e.to_string().into()))?,
            };

            let candidate_count = rows.len();
            debug!(
                candidate_count = candidate_count,
                "Computing similarity in Rust"
            );

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

                    let score =
                        crate::traits::store::cosine_similarity(query_embedding, &embedding);

                    PageRef {
                        url,
                        title,
                        site_url,
                        score,
                    }
                })
                .collect();

            results.sort_by(|a, b| {
                b.score
                    .partial_cmp(&a.score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            results.truncate(limit);

            debug!(
                result_count = results.len(),
                "Semantic search completed (BYTEA fallback)"
            );
            Ok(results)
        }
    }

    #[instrument(skip(self), fields(url = %url))]
    async fn delete_embedding(&self, url: &str) -> Result<()> {
        debug!(url = %url, "Deleting embedding");
        sqlx::query("DELETE FROM extraction_embeddings WHERE url = $1")
            .bind(url)
            .execute(&self.pool)
            .await
            .map_err(|e| ExtractionError::Storage(e.to_string().into()))?;

        info!(url = %url, "Deleted embedding");
        Ok(())
    }
}

#[async_trait]
impl KeywordSearch for PostgresStore {
    #[instrument(skip(self, filter), fields(query = %query, limit = limit))]
    async fn keyword_search(
        &self,
        query: &str,
        limit: usize,
        filter: Option<&QueryFilter>,
    ) -> Result<Vec<PageRef>> {
        let filter_sites = filter.map(|f| f.include_sites.len()).unwrap_or(0);
        debug!(query = %query, limit = limit, filter_sites = filter_sites, "Keyword search");

        // Use PostgreSQL full-text search
        let rows = match filter {
            Some(f) if !f.include_sites.is_empty() => {
                sqlx::query_as::<_, (String, String, Option<String>, f32)>(
                    r#"
                    SELECT url, site_url, title,
                           ts_rank(to_tsvector('english', content), plainto_tsquery('english', $1)) as score
                    FROM extraction_pages
                    WHERE to_tsvector('english', content) @@ plainto_tsquery('english', $1)
                      AND site_url = ANY($2)
                    ORDER BY score DESC
                    LIMIT $3
                    "#,
                )
                .bind(query)
                .bind(&f.include_sites)
                .bind(limit as i64)
                .fetch_all(&self.pool)
                .await
                .map_err(|e| ExtractionError::Storage(e.to_string().into()))?
            }
            _ => {
                sqlx::query_as::<_, (String, String, Option<String>, f32)>(
                    r#"
                    SELECT url, site_url, title,
                           ts_rank(to_tsvector('english', content), plainto_tsquery('english', $1)) as score
                    FROM extraction_pages
                    WHERE to_tsvector('english', content) @@ plainto_tsquery('english', $1)
                    ORDER BY score DESC
                    LIMIT $2
                    "#,
                )
                .bind(query)
                .bind(limit as i64)
                .fetch_all(&self.pool)
                .await
                .map_err(|e| ExtractionError::Storage(e.to_string().into()))?
            }
        };

        let results: Vec<PageRef> = rows
            .into_iter()
            .map(|(url, site_url, title, score)| PageRef {
                url,
                title,
                site_url,
                score,
            })
            .collect();

        debug!(query = %query, result_count = results.len(), "Keyword search completed");
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_postgres_compile() {
        // Just verify the module compiles
    }

    #[test]
    fn test_rrf_constant() {
        // Verify default RRF constant matches paper recommendation
        assert_eq!(DEFAULT_RRF_K, 60.0);
    }

    #[test]
    fn test_query_hash() {
        let hash1 = PostgresStore::hash_query("find board members");
        let hash2 = PostgresStore::hash_query("find board members");
        let hash3 = PostgresStore::hash_query("find contact info");

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
    }
}
