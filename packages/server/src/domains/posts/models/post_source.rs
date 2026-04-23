use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{PostId, PostSourceId};

/// A source from which a post was discovered. Addendum 01 extends this beyond
/// the original `(source_type, source_id, source_url)` triple with per-citation
/// metadata (`content_hash`, `snippet`, `confidence`, `platform_id`,
/// `platform_post_type_hint`) and a primary-citation flag.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PostSource {
    pub id: PostSourceId,
    pub post_id: PostId,
    pub source_type: String,
    pub source_id: Uuid,
    pub source_url: Option<String>,
    pub first_seen_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
    pub disappeared_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub content_hash: Option<String>,
    pub snippet: Option<String>,
    pub confidence: Option<i32>,
    pub platform_id: Option<String>,
    pub platform_post_type_hint: Option<String>,
    pub is_primary: bool,
}

/// Input for creating a `post_sources` row from the ingest handler. Carries
/// everything Addendum 01 lets Root Signal send per citation.
#[derive(Debug, Clone, Default)]
pub struct PostSourceInsert<'a> {
    pub post_id: PostId,
    pub source_type: &'a str,
    pub source_id: Uuid,
    pub source_url: Option<&'a str>,
    pub content_hash: Option<&'a str>,
    pub snippet: Option<&'a str>,
    pub confidence: Option<i32>,
    pub platform_id: Option<&'a str>,
    pub platform_post_type_hint: Option<&'a str>,
    pub is_primary: bool,
    /// Root Signal's `retrieved_at` for the citation (UTC). If `None`, DB
    /// defaults to NOW().
    pub retrieved_at: Option<DateTime<Utc>>,
}

/// Enriched citation row for the admin Sources panel. Joins
/// `post_sources` with either `organizations` (via `sources`) or
/// `source_individuals` (when that table exists — per Worktree 3).
///
/// The Addendum 01 fields (content_hash, snippet, confidence,
/// platform_id, platform_post_type_hint) are all Options: they'll be
/// null until Worktree 3's migration adds them, after which this
/// loader will surface them as-is.
#[derive(Debug, Clone, Serialize)]
pub struct PostSourceEnriched {
    pub id: Uuid,
    pub source_url: Option<String>,
    /// `organization` | `individual`.
    pub kind: String,
    pub organization_id: Option<Uuid>,
    pub organization_name: Option<String>,
    pub individual_id: Option<Uuid>,
    pub individual_display_name: Option<String>,
    pub retrieved_at: Option<DateTime<Utc>>,
    pub content_hash: Option<String>,
    pub snippet: Option<String>,
    pub confidence: Option<i32>,
    pub platform_id: Option<String>,
    pub platform_post_type_hint: Option<String>,
    /// True for the citation currently feeding `post_source_attribution`.
    /// Derived — see [`find_enriched_by_post`].
    pub is_primary: bool,
    pub first_seen_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
}

impl PostSource {
    /// Find all sources for a post.
    pub async fn find_by_post(post_id: PostId, pool: &PgPool) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM post_sources WHERE post_id = $1 ORDER BY is_primary DESC, created_at ASC",
        )
        .bind(post_id)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    /// The primary source for a post (or first-by-created_at if none is
    /// flagged primary — covers legacy rows inserted before is_primary shipped).
    pub async fn find_primary(post_id: PostId, pool: &PgPool) -> Result<Option<Self>> {
        sqlx::query_as::<_, Self>(
            r#"
            SELECT * FROM post_sources
            WHERE post_id = $1
            ORDER BY is_primary DESC, created_at ASC
            LIMIT 1
            "#,
        )
        .bind(post_id)
        .fetch_optional(pool)
        .await
        .map_err(Into::into)
    }

    /// Legacy 4-arg create, preserved so existing callers (seed, admin
    /// writeback) don't break. New ingest code should use `insert_full`.
    pub async fn create(
        post_id: PostId,
        source_type: &str,
        source_id: Uuid,
        source_url: Option<&str>,
        pool: &PgPool,
    ) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO post_sources (post_id, source_type, source_id, source_url)
            VALUES ($1, $2, $3, $4)
            RETURNING *
            "#,
        )
        .bind(post_id)
        .bind(source_type)
        .bind(source_id)
        .bind(source_url)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    /// Ingest-path insert: full Addendum-01 metadata.
    pub async fn insert_full(input: PostSourceInsert<'_>, pool: &PgPool) -> Result<Self> {
        let retrieved = input.retrieved_at.unwrap_or_else(Utc::now);
        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO post_sources (
                post_id, source_type, source_id, source_url,
                first_seen_at, last_seen_at,
                content_hash, snippet, confidence,
                platform_id, platform_post_type_hint, is_primary
            )
            VALUES ($1, $2, $3, $4, $5, $5, $6, $7, $8, $9, $10, $11)
            RETURNING *
            "#,
        )
        .bind(input.post_id)
        .bind(input.source_type)
        .bind(input.source_id)
        .bind(input.source_url)
        .bind(retrieved)
        .bind(input.content_hash)
        .bind(input.snippet)
        .bind(input.confidence)
        .bind(input.platform_id)
        .bind(input.platform_post_type_hint)
        .bind(input.is_primary)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    /// Find a prior citation with the same `content_hash`. Used for Addendum
    /// §5 "re-verified, unchanged" detection during revision handling.
    pub async fn find_by_content_hash(
        content_hash: &str,
        pool: &PgPool,
    ) -> Result<Option<Self>> {
        sqlx::query_as::<_, Self>(
            r#"
            SELECT * FROM post_sources
            WHERE content_hash = $1
            ORDER BY last_seen_at DESC
            LIMIT 1
            "#,
        )
        .bind(content_hash)
        .fetch_optional(pool)
        .await
        .map_err(Into::into)
    }

    /// Refresh `last_seen_at` on a matched-by-hash row — the underlying
    /// source hasn't changed, we just re-verified it.
    pub async fn touch_last_seen(id: PostSourceId, pool: &PgPool) -> Result<()> {
        sqlx::query("UPDATE post_sources SET last_seen_at = NOW(), updated_at = NOW() WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Find all sources for a post, joined with organisation (and, once
    /// `source_individuals` is wired in, individual) metadata. Primary
    /// citation is read directly from `post_sources.is_primary`, whose
    /// partial unique index enforces exactly one primary per post.
    /// Used by the admin Sources panel.
    pub async fn find_enriched_by_post(
        post_id: PostId,
        pool: &PgPool,
    ) -> Result<Vec<PostSourceEnriched>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            id: Uuid,
            source_url: Option<String>,
            first_seen_at: DateTime<Utc>,
            last_seen_at: DateTime<Utc>,
            organization_id: Option<Uuid>,
            organization_name: Option<String>,
            retrieved_at: Option<DateTime<Utc>>,
            content_hash: Option<String>,
            snippet: Option<String>,
            confidence: Option<i32>,
            platform_id: Option<String>,
            platform_post_type_hint: Option<String>,
            is_primary: bool,
        }

        let rows = sqlx::query_as::<_, Row>(
            r#"
            SELECT
                ps.id                        AS id,
                ps.source_url                AS source_url,
                ps.first_seen_at             AS first_seen_at,
                ps.last_seen_at              AS last_seen_at,
                s.organization_id            AS organization_id,
                o.name                       AS organization_name,
                ps.first_seen_at             AS retrieved_at,
                ps.content_hash              AS content_hash,
                ps.snippet                   AS snippet,
                ps.confidence                AS confidence,
                ps.platform_id               AS platform_id,
                ps.platform_post_type_hint   AS platform_post_type_hint,
                ps.is_primary                AS is_primary
            FROM post_sources ps
            LEFT JOIN sources s       ON s.id = ps.source_id
            LEFT JOIN organizations o ON o.id = s.organization_id
            WHERE ps.post_id = $1
            ORDER BY ps.is_primary DESC, ps.first_seen_at ASC, ps.created_at ASC
            "#,
        )
        .bind(post_id)
        .fetch_all(pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| PostSourceEnriched {
                id: row.id,
                source_url: row.source_url,
                kind: "organization".to_string(),
                organization_id: row.organization_id,
                organization_name: row.organization_name,
                individual_id: None,
                individual_display_name: None,
                retrieved_at: row.retrieved_at,
                content_hash: row.content_hash,
                snippet: row.snippet,
                confidence: row.confidence,
                platform_id: row.platform_id,
                platform_post_type_hint: row.platform_post_type_hint,
                is_primary: row.is_primary,
                first_seen_at: row.first_seen_at,
                last_seen_at: row.last_seen_at,
            })
            .collect())
    }

    /// Look up a single `post_sources` row's attribution info, scoped
    /// to its post (prevents an admin from reassigning a row to a post
    /// it doesn't belong to). Returns `(organization_name, source_url)`.
    pub async fn find_attribution_info(
        post_id: PostId,
        post_source_id: PostSourceId,
        pool: &PgPool,
    ) -> Result<Option<(Option<String>, Option<String>)>> {
        let row: Option<(Option<String>, Option<String>)> = sqlx::query_as(
            r#"
            SELECT o.name AS organization_name, ps.source_url AS source_url
            FROM post_sources ps
            LEFT JOIN sources s       ON s.id = ps.source_id
            LEFT JOIN organizations o ON o.id = s.organization_id
            WHERE ps.id = $1 AND ps.post_id = $2
            "#,
        )
        .bind(post_source_id)
        .bind(post_id)
        .fetch_optional(pool)
        .await?;
        Ok(row)
    }

    /// Atomically flip `is_primary` on a post: unset every current primary
    /// then set the chosen row. Partial unique index enforces invariant.
    pub async fn set_primary(
        post_id: PostId,
        post_source_id: PostSourceId,
        pool: &PgPool,
    ) -> Result<()> {
        let mut tx = pool.begin().await?;
        sqlx::query("UPDATE post_sources SET is_primary = false, updated_at = NOW() WHERE post_id = $1 AND is_primary = true")
            .bind(post_id)
            .execute(&mut *tx)
            .await?;
        let updated = sqlx::query("UPDATE post_sources SET is_primary = true, updated_at = NOW() WHERE id = $1 AND post_id = $2")
            .bind(post_source_id)
            .bind(post_id)
            .execute(&mut *tx)
            .await?;
        if updated.rows_affected() == 0 {
            tx.rollback().await?;
            return Err(anyhow::anyhow!("post_source_id does not belong to post_id"));
        }
        tx.commit().await?;
        Ok(())
    }

    /// Copy all sources from one post to another (for revision creation).
    pub async fn copy_sources(
        from_post_id: PostId,
        to_post_id: PostId,
        pool: &PgPool,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO post_sources (
                post_id, source_type, source_id, source_url,
                first_seen_at, last_seen_at, disappeared_at,
                content_hash, snippet, confidence,
                platform_id, platform_post_type_hint, is_primary
            )
            SELECT $2, source_type, source_id, source_url,
                first_seen_at, last_seen_at, disappeared_at,
                content_hash, snippet, confidence,
                platform_id, platform_post_type_hint, is_primary
            FROM post_sources
            WHERE post_id = $1
            ON CONFLICT (post_id, source_type, source_id) DO NOTHING
            "#,
        )
        .bind(from_post_id)
        .bind(to_post_id)
        .execute(pool)
        .await?;
        Ok(())
    }
}
