//! Media model — represents uploaded files in the media library.
//!
//! All SQL queries live here (never in Restate handlers or activities).

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
}

/// Filters for listing media.
#[derive(Debug, Default)]
pub struct MediaFilters<'a> {
    pub content_type_prefix: Option<&'a str>,
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
    /// Returns (items, total_count).
    pub async fn list_paginated(
        filters: &MediaFilters<'_>,
        limit: i64,
        offset: i64,
        pool: &PgPool,
    ) -> Result<(Vec<Self>, i64)> {
        let items = sqlx::query_as::<_, Self>(
            "SELECT * FROM media
             WHERE ($1::text IS NULL OR content_type LIKE $1 || '%')
             ORDER BY created_at DESC
             LIMIT $2 OFFSET $3",
        )
        .bind(filters.content_type_prefix)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?;

        let count_row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM media
             WHERE ($1::text IS NULL OR content_type LIKE $1 || '%')",
        )
        .bind(filters.content_type_prefix)
        .fetch_one(pool)
        .await?;

        Ok((items, count_row.0))
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

    /// Update alt text for a media record.
    pub async fn update_alt_text(id: Uuid, alt_text: &str, pool: &PgPool) -> Result<Self> {
        let row = sqlx::query_as::<_, Self>(
            "UPDATE media SET alt_text = $2, updated_at = NOW() WHERE id = $1 RETURNING *",
        )
        .bind(id)
        .bind(alt_text)
        .fetch_one(pool)
        .await?;

        Ok(row)
    }
}
