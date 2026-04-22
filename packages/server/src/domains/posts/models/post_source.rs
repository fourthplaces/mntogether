use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{PostId, PostSourceId};

/// A source from which a post was discovered (website, Instagram, etc.)
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
    /// Find all sources for a post
    pub async fn find_by_post(post_id: PostId, pool: &PgPool) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM post_sources WHERE post_id = $1 ORDER BY created_at ASC",
        )
        .bind(post_id)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    /// Create a new post source link
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

    /// Find all sources for a post, joined with organisation (and
    /// individual, once Worktree 3's `source_individuals` table
    /// lands) metadata. Used by the admin Sources panel.
    ///
    /// **Primary derivation.** `is_primary` is computed from
    /// `post_source_attribution.source_name`: the row whose org
    /// name, individual display name, or source URL matches it wins.
    /// Fallback: earliest `first_seen_at`. Once Worktree 3 lands an
    /// `is_primary` column on `post_sources`, switch to reading that
    /// directly and drop this derivation.
    pub async fn find_enriched_by_post(
        post_id: PostId,
        pool: &PgPool,
    ) -> Result<Vec<PostSourceEnriched>> {
        // Left-join organizations; source_individuals is not live
        // yet (Worktree 3 will add it), so kind currently resolves
        // to 'organization' for every row. The query is structured
        // to accept WT3's columns without changing shape — missing
        // columns are returned as NULL until the migration lands.
        #[derive(sqlx::FromRow)]
        struct Row {
            id: Uuid,
            source_url: Option<String>,
            first_seen_at: DateTime<Utc>,
            last_seen_at: DateTime<Utc>,
            organization_id: Option<Uuid>,
            organization_name: Option<String>,
        }

        let rows = sqlx::query_as::<_, Row>(
            r#"
            SELECT
                ps.id                        AS id,
                ps.source_url                AS source_url,
                ps.first_seen_at             AS first_seen_at,
                ps.last_seen_at              AS last_seen_at,
                s.organization_id            AS organization_id,
                o.name                       AS organization_name
            FROM post_sources ps
            LEFT JOIN sources s       ON s.id = ps.source_id
            LEFT JOIN organizations o ON o.id = s.organization_id
            WHERE ps.post_id = $1
            ORDER BY ps.first_seen_at ASC, ps.created_at ASC
            "#,
        )
        .bind(post_id)
        .fetch_all(pool)
        .await?;

        // Read the existing post_source_attribution (if any) so we
        // can flag the matching post_sources row as primary.
        let attr_source_name: Option<String> = sqlx::query_scalar::<_, Option<String>>(
            "SELECT source_name FROM post_source_attribution WHERE post_id = $1",
        )
        .bind(post_id)
        .fetch_optional(pool)
        .await?
        .flatten();

        let mut enriched: Vec<PostSourceEnriched> = rows
            .into_iter()
            .map(|row| PostSourceEnriched {
                id: row.id,
                source_url: row.source_url,
                kind: "organization".to_string(),
                organization_id: row.organization_id,
                organization_name: row.organization_name,
                individual_id: None,
                individual_display_name: None,
                retrieved_at: None,
                content_hash: None,
                snippet: None,
                confidence: None,
                platform_id: None,
                platform_post_type_hint: None,
                is_primary: false,
                first_seen_at: row.first_seen_at,
                last_seen_at: row.last_seen_at,
            })
            .collect();

        // Primary pick: prefer the row whose org name / source_url
        // matches post_source_attribution.source_name. Fallback to
        // the first row (earliest first_seen_at).
        let primary_idx = match attr_source_name.as_deref() {
            Some(name) if !name.is_empty() => enriched.iter().position(|r| {
                r.organization_name.as_deref() == Some(name)
                    || r.source_url.as_deref() == Some(name)
            }),
            _ => None,
        }
        .or({
            if enriched.is_empty() {
                None
            } else {
                Some(0)
            }
        });

        if let Some(idx) = primary_idx {
            enriched[idx].is_primary = true;
        }

        Ok(enriched)
    }

    /// Look up a single post_sources row's identifying info for
    /// updating post_source_attribution when an editor reassigns
    /// primary. Returns (organization_name, source_url).
    pub async fn find_attribution_info(
        post_source_id: PostSourceId,
        pool: &PgPool,
    ) -> Result<Option<(Option<String>, Option<String>)>> {
        let row: Option<(Option<String>, Option<String>)> = sqlx::query_as(
            r#"
            SELECT o.name AS organization_name, ps.source_url AS source_url
            FROM post_sources ps
            LEFT JOIN sources s       ON s.id = ps.source_id
            LEFT JOIN organizations o ON o.id = s.organization_id
            WHERE ps.id = $1
            "#,
        )
        .bind(post_source_id)
        .fetch_optional(pool)
        .await?;
        Ok(row)
    }

    /// Copy all sources from one post to another (for revision creation)
    pub async fn copy_sources(
        from_post_id: PostId,
        to_post_id: PostId,
        pool: &PgPool,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO post_sources (post_id, source_type, source_id, source_url,
                first_seen_at, last_seen_at, disappeared_at)
            SELECT $2, source_type, source_id, source_url,
                first_seen_at, last_seen_at, disappeared_at
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
