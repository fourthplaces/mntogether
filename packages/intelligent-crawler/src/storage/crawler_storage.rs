use async_trait::async_trait;
use sqlx::{PgPool, Postgres, Row, Transaction};
use uuid::Uuid;

use crate::events::{DiscoverySource, FlagSource};
use crate::new_types::*;
use crate::traits::CrawlerStorage;

pub struct PostgresCrawlerStorage {
    pool: PgPool,
}

impl PostgresCrawlerStorage {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl CrawlerStorage for PostgresCrawlerStorage {
    type ResourceId = Uuid;
    type PageId = Uuid;
    type ExtractionRunId = Uuid;
    type ExtractionId = Uuid;
    type Transaction = Transaction<'static, Postgres>;
    type Error = sqlx::Error;

    // ========================================================================
    // TRANSACTION SUPPORT
    // ========================================================================

    async fn begin_transaction(&self) -> Result<Self::Transaction, Self::Error> {
        Ok(self.pool.begin().await?)
    }

    async fn commit_transaction(&self, tx: Self::Transaction) -> Result<(), Self::Error> {
        tx.commit().await?;
        Ok(())
    }

    async fn rollback_transaction(&self, tx: Self::Transaction) -> Result<(), Self::Error> {
        tx.rollback().await?;
        Ok(())
    }

    // ========================================================================
    // RESOURCES
    // ========================================================================

    async fn insert_resource(
        &self,
        resource: Resource,
        tx: Option<&mut Self::Transaction>,
    ) -> Result<Self::ResourceId, Self::Error> {
        let id = Uuid::new_v4();

        let status_str = match resource.discovery_status {
            DiscoveryStatus::Pending => "pending",
            DiscoveryStatus::Discovering => "discovering",
            DiscoveryStatus::Completed => "completed",
            DiscoveryStatus::Failed => "failed",
        };

        let query = sqlx::query(
            r#"
            INSERT INTO resources (
                id, url, domain, submitted_by,
                discovery_version, discovery_status
            )
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
        )
        .bind(id)
        .bind(resource.url.as_str())
        .bind(&resource.domain)
        .bind(resource.submitted_by.as_ref())
        .bind(resource.discovery_version)
        .bind(status_str);

        match tx {
            Some(tx) => query.execute(&mut **tx).await?,
            None => query.execute(&self.pool).await?,
        };

        Ok(id)
    }

    async fn get_resource(&self, id: Self::ResourceId) -> Result<Option<Resource>, Self::Error> {
        let row = sqlx::query(
            r#"
            SELECT id, url, domain, submitted_by, discovery_version, discovery_status
            FROM resources
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| {
            let status_str: &str = r.get("discovery_status");
            let discovery_status = match status_str {
                "pending" => DiscoveryStatus::Pending,
                "discovering" => DiscoveryStatus::Discovering,
                "completed" => DiscoveryStatus::Completed,
                "failed" => DiscoveryStatus::Failed,
                _ => DiscoveryStatus::Pending,
            };

            Resource {
                url: r.get::<String, _>("url").parse().unwrap(),
                domain: r.get("domain"),
                submitted_by: r.get("submitted_by"),
                discovery_version: r.get("discovery_version"),
                discovery_status,
            }
        }))
    }

    async fn update_resource_status(
        &self,
        id: Self::ResourceId,
        status: DiscoveryStatus,
        version: i32,
        tx: Option<&mut Self::Transaction>,
    ) -> Result<(), Self::Error> {
        let status_str = match status {
            DiscoveryStatus::Pending => "pending",
            DiscoveryStatus::Discovering => "discovering",
            DiscoveryStatus::Completed => "completed",
            DiscoveryStatus::Failed => "failed",
        };

        let query = sqlx::query(
            r#"
            UPDATE resources
            SET discovery_status = $1,
                discovery_version = $2,
                last_discovered_at = CASE WHEN $1 = 'completed' THEN NOW() ELSE last_discovered_at END
            WHERE id = $3
            "#,
        )
        .bind(status_str)
        .bind(version)
        .bind(id);

        match tx {
            Some(tx) => query.execute(&mut **tx).await?,
            None => query.execute(&self.pool).await?,
        };

        Ok(())
    }

    // ========================================================================
    // PAGES (CANONICAL)
    // ========================================================================

    async fn upsert_page(
        &self,
        page: DiscoveredPage,
        tx: Option<&mut Self::Transaction>,
    ) -> Result<UpsertResult<Self::PageId>, Self::Error> {
        let id = Uuid::new_v4();

        let flag_status_str = match page.flag_status {
            FlagStatus::Pending => "pending",
            FlagStatus::Flagged => "flagged",
            FlagStatus::Unflagged => "unflagged",
            FlagStatus::Error => "error",
        };

        let flag_source_str = page.flagged_by.as_ref().map(|fs| match fs {
            FlagSource::Ai => "ai",
            FlagSource::Rule => "rule",
            FlagSource::Manual => "manual",
        });

        // Use ON CONFLICT to detect if page was inserted or updated
        // xmax = 0 means INSERT, xmax > 0 means UPDATE
        let row = sqlx::query(
            r#"
            INSERT INTO discovered_pages (
                id, url, domain, flag_status, flagged_by, flag_confidence, flag_reason,
                content_hash, html, markdown
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            ON CONFLICT (url) DO UPDATE SET
                content_hash = EXCLUDED.content_hash,
                html = EXCLUDED.html,
                markdown = EXCLUDED.markdown,
                updated_at = NOW()
            RETURNING id, (xmax = 0) as was_inserted
            "#,
        )
        .bind(id)
        .bind(page.url.as_str())
        .bind(&page.domain)
        .bind(flag_status_str)
        .bind(flag_source_str)
        .bind(page.flag_confidence)
        .bind(page.flag_reason.as_ref())
        .bind(&page.content_hash)
        .bind(page.html.as_ref())
        .bind(&page.markdown);

        let result = match tx {
            Some(tx) => row.fetch_one(&mut **tx).await?,
            None => row.fetch_one(&self.pool).await?,
        };

        let page_id: Uuid = result.get("id");
        let was_inserted: bool = result.get("was_inserted");

        Ok(UpsertResult {
            page_id,
            was_inserted,
        })
    }

    async fn get_page(&self, id: Self::PageId) -> Result<Option<DiscoveredPage>, Self::Error> {
        let row = sqlx::query(
            r#"
            SELECT id, url, domain, flag_status, flagged_by, flag_confidence, flag_reason,
                   content_hash, html, markdown
            FROM discovered_pages
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| {
            let flag_status_str: &str = r.get("flag_status");
            let flag_status = match flag_status_str {
                "pending" => FlagStatus::Pending,
                "flagged" => FlagStatus::Flagged,
                "unflagged" => FlagStatus::Unflagged,
                "error" => FlagStatus::Error,
                _ => FlagStatus::Pending,
            };

            let flagged_by = r.get::<Option<&str>, _>("flagged_by").map(|s| match s {
                "ai" => FlagSource::Ai,
                "rule" => FlagSource::Rule,
                "manual" => FlagSource::Manual,
                _ => FlagSource::Rule,
            });

            DiscoveredPage {
                url: r.get::<String, _>("url").parse().unwrap(),
                domain: r.get("domain"),
                flag_status,
                flagged_by,
                flag_confidence: r.get("flag_confidence"),
                flag_reason: r.get("flag_reason"),
                content_hash: r.get("content_hash"),
                html: r.get("html"),
                markdown: r.get("markdown"),
            }
        }))
    }

    async fn update_page_flag(
        &self,
        id: Self::PageId,
        flag: FlagResult,
        tx: Option<&mut Self::Transaction>,
    ) -> Result<(), Self::Error> {
        let flag_status_str = match flag.status {
            FlagStatus::Pending => "pending",
            FlagStatus::Flagged => "flagged",
            FlagStatus::Unflagged => "unflagged",
            FlagStatus::Error => "error",
        };

        let flag_source_str = flag.source.as_ref().map(|fs| match fs {
            FlagSource::Ai => "ai",
            FlagSource::Rule => "rule",
            FlagSource::Manual => "manual",
        });

        let query = sqlx::query(
            r#"
            UPDATE discovered_pages
            SET flag_status = $1,
                flagged_by = $2,
                flag_confidence = $3,
                flag_reason = $4,
                flagged_at = CASE WHEN $1 = 'flagged' THEN NOW() ELSE flagged_at END
            WHERE id = $5
            "#,
        )
        .bind(flag_status_str)
        .bind(flag_source_str)
        .bind(flag.confidence)
        .bind(flag.reason.as_ref())
        .bind(id);

        match tx {
            Some(tx) => query.execute(&mut **tx).await?,
            None => query.execute(&self.pool).await?,
        };

        Ok(())
    }

    async fn find_pages_to_refresh(
        &self,
        limit: usize,
    ) -> Result<Vec<PageToRefresh<Self::PageId>>, Self::Error> {
        let rows = sqlx::query(
            r#"
            SELECT id, url, domain, content_hash, base_interval_hours, jitter_hours
            FROM get_pages_to_refresh($1)
            "#,
        )
        .bind(limit as i32)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| PageToRefresh {
                page_id: r.get("id"),
                url: r.get::<String, _>("url").parse().unwrap(),
                content_hash: r.get("content_hash"),
                base_interval_hours: r.get("base_interval_hours"),
                jitter_hours: r.get("jitter_hours"),
            })
            .collect())
    }

    // ========================================================================
    // DISCOVERY EDGES
    // ========================================================================

    async fn record_discovery_edge(
        &self,
        edge: ResourcePageEdge<Self::ResourceId, Self::PageId>,
        tx: Option<&mut Self::Transaction>,
    ) -> Result<(), Self::Error> {
        let discovered_via_str = match edge.discovered_via {
            DiscoverySource::DirectSubmission => "direct_submission",
            DiscoverySource::Crawl => "crawl",
        };

        let query = sqlx::query(
            r#"
            INSERT INTO resource_page_edges (
                resource_id, discovered_page_id, crawl_session_id,
                crawl_depth, discovered_via
            )
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (resource_id, discovered_page_id, crawl_session_id) DO NOTHING
            "#,
        )
        .bind(edge.resource_id)
        .bind(edge.page_id)
        .bind(edge.crawl_session_id)
        .bind(edge.crawl_depth)
        .bind(discovered_via_str);

        match tx {
            Some(tx) => query.execute(&mut **tx).await?,
            None => query.execute(&self.pool).await?,
        };

        Ok(())
    }

    // ========================================================================
    // EXTRACTION RUNS
    // ========================================================================

    async fn create_extraction_run(
        &self,
        run: ExtractionRun<Self::PageId>,
    ) -> Result<Self::ExtractionRunId, Self::Error> {
        let id = Uuid::new_v4();

        sqlx::query(
            r#"
            INSERT INTO extraction_runs (
                id, discovered_page_id, page_content_hash,
                extractor_version, prompt_version, model,
                status
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(id)
        .bind(run.page_id)
        .bind(&run.page_content_hash)
        .bind(&run.extractor_version)
        .bind(&run.prompt_version)
        .bind(&run.model)
        .bind("running")
        .execute(&self.pool)
        .await?;

        Ok(id)
    }

    async fn finish_extraction_run(
        &self,
        run_id: Self::ExtractionRunId,
        stats: ExtractionStats,
    ) -> Result<(), Self::Error> {
        sqlx::query(
            r#"
            UPDATE extraction_runs
            SET status = $1,
                finished_at = NOW(),
                items_found = $2,
                items_created = $3,
                items_updated = $4
            WHERE id = $5
            "#,
        )
        .bind("succeeded")
        .bind(stats.items_found as i32)
        .bind(stats.items_created as i32)
        .bind(stats.items_updated as i32)
        .bind(run_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    // ========================================================================
    // EXTRACTIONS
    // ========================================================================

    async fn insert_extraction(
        &self,
        extraction: RawExtraction,
        run_id: Self::ExtractionRunId,
    ) -> Result<Self::ExtractionId, Self::Error> {
        let id = Uuid::new_v4();

        sqlx::query(
            r#"
            INSERT INTO extractions (
                id, extraction_run_id, discovered_page_id,
                data, confidence, fingerprint_hint
            )
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
        )
        .bind(id)
        .bind(run_id)
        .bind(extraction.page_id)
        .bind(&extraction.data)
        .bind(extraction.confidence)
        .bind(extraction.fingerprint_hint)
        .execute(&self.pool)
        .await?;

        Ok(id)
    }

    async fn get_extraction(
        &self,
        id: Self::ExtractionId,
    ) -> Result<Option<RawExtraction>, Self::Error> {
        let row = sqlx::query(
            r#"
            SELECT e.id, e.extraction_run_id, e.discovered_page_id,
                   e.data, e.confidence, e.fingerprint_hint,
                   dp.url
            FROM extractions e
            JOIN discovered_pages dp ON e.discovered_page_id = dp.id
            WHERE e.id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| RawExtraction {
            extraction_run_id: r.get("extraction_run_id"),
            page_id: r.get("discovered_page_id"),
            page_url: r.get::<String, _>("url").parse().unwrap(),
            data: r.get("data"),
            confidence: r.get("confidence"),
            fingerprint_hint: r.get("fingerprint_hint"),
        }))
    }

    async fn list_extractions_for_page(
        &self,
        page_id: Self::PageId,
    ) -> Result<Vec<RawExtraction>, Self::Error> {
        let rows = sqlx::query(
            r#"
            SELECT e.id, e.extraction_run_id, e.discovered_page_id,
                   e.data, e.confidence, e.fingerprint_hint,
                   dp.url
            FROM extractions e
            JOIN discovered_pages dp ON e.discovered_page_id = dp.id
            WHERE e.discovered_page_id = $1
            ORDER BY e.created_at DESC
            "#,
        )
        .bind(page_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| RawExtraction {
                extraction_run_id: r.get("extraction_run_id"),
                page_id: r.get("discovered_page_id"),
                page_url: r.get::<String, _>("url").parse().unwrap(),
                data: r.get("data"),
                confidence: r.get("confidence"),
                fingerprint_hint: r.get("fingerprint_hint"),
            })
            .collect())
    }
}
