//! Media model — represents uploaded files in the media library.
//!
//! All SQL queries live here (never in HTTP handlers or activities).

use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct Media {
    pub id: Uuid,
    pub filename: String,
    pub content_type: String,
    pub size_bytes: i64,
    pub storage_key: String,
    pub url: String,
    pub alt_text: Option<String>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub uploaded_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// Populated by the Root Signal media ingest pipeline; NULL for
    /// editor-uploaded rows.
    pub source_url: Option<String>,
    pub source_ingested_at: Option<DateTime<Utc>>,
    /// SHA-256 of the normalised bytes. Used by the ingest path for
    /// exact-match dedup; NULL for rows that haven't been through ingest.
    pub content_hash: Option<String>,
}

/// Filters for listing media.
#[derive(Debug, Default)]
pub struct MediaFilters<'a> {
    pub content_type_prefix: Option<&'a str>,
    /// Substring match against filename or alt_text (ILIKE).
    pub search: Option<&'a str>,
    /// When true, only return media with zero rows in media_references.
    pub unused_only: bool,
}

/// A Media row with its current usage_count pre-joined — returned by
/// `list_with_usage` for Library UIs that need to show "Used by N" badges.
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct MediaWithUsage {
    pub id: Uuid,
    pub filename: String,
    pub content_type: String,
    pub size_bytes: i64,
    pub storage_key: String,
    pub url: String,
    pub alt_text: Option<String>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub uploaded_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub source_url: Option<String>,
    pub source_ingested_at: Option<DateTime<Utc>>,
    pub content_hash: Option<String>,
    pub usage_count: i64,
}

impl Media {
    /// Create a new media record after a successful upload.
    pub async fn create(
        filename: &str,
        content_type: &str,
        size_bytes: i64,
        storage_key: &str,
        url: &str,
        alt_text: Option<&str>,
        width: Option<i32>,
        height: Option<i32>,
        uploaded_by: Option<Uuid>,
        pool: &PgPool,
    ) -> Result<Self> {
        let row = sqlx::query_as::<_, Self>(
            "INSERT INTO media (filename, content_type, size_bytes, storage_key, url, alt_text, width, height, uploaded_by)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
             RETURNING *",
        )
        .bind(filename)
        .bind(content_type)
        .bind(size_bytes)
        .bind(storage_key)
        .bind(url)
        .bind(alt_text)
        .bind(width)
        .bind(height)
        .bind(uploaded_by)
        .fetch_one(pool)
        .await?;

        Ok(row)
    }

    /// Find a media record by ID.
    pub async fn find_by_id(id: Uuid, pool: &PgPool) -> Result<Option<Self>> {
        let row = sqlx::query_as::<_, Self>("SELECT * FROM media WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await?;

        Ok(row)
    }

    /// List media with offset pagination, ordered by newest first.
    /// Returns (items, total_count). Kept for backward compatibility —
    /// new callers should prefer `list_with_usage` which joins usage counts.
    pub async fn list_paginated(
        filters: &MediaFilters<'_>,
        limit: i64,
        offset: i64,
        pool: &PgPool,
    ) -> Result<(Vec<Self>, i64)> {
        let (items, total) = Self::list_with_usage(filters, limit, offset, pool).await?;
        let bare: Vec<Self> = items
            .into_iter()
            .map(|m| Self {
                id: m.id,
                filename: m.filename,
                content_type: m.content_type,
                size_bytes: m.size_bytes,
                storage_key: m.storage_key,
                url: m.url,
                alt_text: m.alt_text,
                width: m.width,
                height: m.height,
                uploaded_by: m.uploaded_by,
                created_at: m.created_at,
                updated_at: m.updated_at,
                source_url: m.source_url,
                source_ingested_at: m.source_ingested_at,
                content_hash: m.content_hash,
            })
            .collect();
        Ok((bare, total))
    }

    /// List media with pagination + usage_count joined. Honors content-type
    /// prefix, filename/alt-text search, and "unused only" filters.
    pub async fn list_with_usage(
        filters: &MediaFilters<'_>,
        limit: i64,
        offset: i64,
        pool: &PgPool,
    ) -> Result<(Vec<MediaWithUsage>, i64)> {
        let search_pat = filters.search.map(|s| format!("%{}%", s));

        let items = sqlx::query_as::<_, MediaWithUsage>(
            r#"
            SELECT m.*, COALESCE(ref.n, 0) AS usage_count
            FROM media m
            LEFT JOIN (
                SELECT media_id, COUNT(*) AS n
                FROM media_references
                GROUP BY media_id
            ) ref ON ref.media_id = m.id
            WHERE ($1::text IS NULL OR m.content_type LIKE $1 || '%')
              AND ($2::text IS NULL OR m.filename ILIKE $2 OR m.alt_text ILIKE $2)
              AND ($3::bool = FALSE OR COALESCE(ref.n, 0) = 0)
            ORDER BY m.created_at DESC
            LIMIT $4 OFFSET $5
            "#,
        )
        .bind(filters.content_type_prefix)
        .bind(search_pat.as_deref())
        .bind(filters.unused_only)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?;

        let search_pat_count = filters.search.map(|s| format!("%{}%", s));
        let count_row: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM media m
            LEFT JOIN (
                SELECT media_id, COUNT(*) AS n
                FROM media_references
                GROUP BY media_id
            ) ref ON ref.media_id = m.id
            WHERE ($1::text IS NULL OR m.content_type LIKE $1 || '%')
              AND ($2::text IS NULL OR m.filename ILIKE $2 OR m.alt_text ILIKE $2)
              AND ($3::bool = FALSE OR COALESCE(ref.n, 0) = 0)
            "#,
        )
        .bind(filters.content_type_prefix)
        .bind(search_pat_count.as_deref())
        .bind(filters.unused_only)
        .fetch_one(pool)
        .await?;

        Ok((items, count_row.0))
    }

    /// Update the editable metadata (alt_text, filename) on a media item.
    pub async fn update_metadata(
        id: Uuid,
        alt_text: Option<&str>,
        filename: Option<&str>,
        pool: &PgPool,
    ) -> Result<Self> {
        let row = sqlx::query_as::<_, Self>(
            r#"
            UPDATE media
            SET
                alt_text = COALESCE($2, alt_text),
                filename = COALESCE($3, filename),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(alt_text)
        .bind(filename)
        .fetch_one(pool)
        .await?;
        Ok(row)
    }

    /// Replace the underlying file for a media item while keeping the same
    /// row (and therefore all references). Storage_key stays, url stays,
    /// dimensions/size/content_type update.
    pub async fn replace_file(
        id: Uuid,
        size_bytes: i64,
        content_type: &str,
        width: Option<i32>,
        height: Option<i32>,
        pool: &PgPool,
    ) -> Result<Self> {
        let row = sqlx::query_as::<_, Self>(
            r#"
            UPDATE media
            SET
                size_bytes = $2,
                content_type = $3,
                width = $4,
                height = $5,
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(size_bytes)
        .bind(content_type)
        .bind(width)
        .bind(height)
        .fetch_one(pool)
        .await?;
        Ok(row)
    }

    /// Look up a media row by its content_hash (SHA-256 over the
    /// normalised bytes we stored). Used by the Root Signal media
    /// ingest pipeline to reuse an existing row when the same image is
    /// submitted twice.
    pub async fn find_by_content_hash(
        content_hash: &str,
        pool: &PgPool,
    ) -> Result<Option<Self>> {
        let row = sqlx::query_as::<_, Self>(
            "SELECT * FROM media WHERE content_hash = $1 LIMIT 1",
        )
        .bind(content_hash)
        .fetch_optional(pool)
        .await?;
        Ok(row)
    }

    /// Insert a new media row produced by the Root Signal media ingest
    /// pipeline. Differs from [`Media::create`] in that it records the
    /// provenance (`source_url`, `source_ingested_at`) and the content
    /// hash used for dedup.
    #[allow(clippy::too_many_arguments)]
    pub async fn create_ingested(
        filename: &str,
        content_type: &str,
        size_bytes: i64,
        storage_key: &str,
        url: &str,
        width: Option<i32>,
        height: Option<i32>,
        source_url: &str,
        content_hash: &str,
        pool: &PgPool,
    ) -> Result<Self> {
        let row = sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO media (
                filename, content_type, size_bytes, storage_key, url,
                width, height,
                source_url, source_ingested_at, content_hash
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NOW(), $9)
            RETURNING *
            "#,
        )
        .bind(filename)
        .bind(content_type)
        .bind(size_bytes)
        .bind(storage_key)
        .bind(url)
        .bind(width)
        .bind(height)
        .bind(source_url)
        .bind(content_hash)
        .fetch_one(pool)
        .await?;
        Ok(row)
    }

    /// Delete a media record.
    pub async fn delete(id: Uuid, pool: &PgPool) -> Result<()> {
        sqlx::query_as::<_, (Uuid,)>("DELETE FROM media WHERE id = $1 RETURNING id")
            .bind(id)
            .fetch_optional(pool)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Media not found: {}", id))?;

        Ok(())
    }

}
