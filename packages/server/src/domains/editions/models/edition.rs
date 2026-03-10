use anyhow::Result;
use chrono::{DateTime, NaiveDate, Utc};
use sqlx::PgPool;
use uuid::Uuid;

/// A county-scoped weekly edition (broadsheet). One edition per county per period.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Edition {
    pub id: Uuid,
    pub county_id: Uuid,
    pub title: Option<String>,
    pub period_start: NaiveDate,
    pub period_end: NaiveDate,
    pub status: String,
    pub published_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Filters for listing editions.
#[derive(Debug, Default)]
pub struct EditionFilters {
    pub county_id: Option<Uuid>,
    pub status: Option<String>,
    pub period_start: Option<NaiveDate>,
    pub period_end: Option<NaiveDate>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

impl Edition {
    /// Create a new draft edition for a county and period.
    pub async fn create(
        county_id: Uuid,
        period_start: NaiveDate,
        period_end: NaiveDate,
        title: Option<&str>,
        pool: &PgPool,
    ) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO editions (county_id, period_start, period_end, title)
            VALUES ($1, $2, $3, $4)
            RETURNING *
            "#,
        )
        .bind(county_id)
        .bind(period_start)
        .bind(period_end)
        .bind(title)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    /// Find an edition by ID.
    pub async fn find_by_id(id: Uuid, pool: &PgPool) -> Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM editions WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await
            .map_err(Into::into)
    }

    /// Find the current (latest) published edition for a county.
    pub async fn find_published(county_id: Uuid, pool: &PgPool) -> Result<Option<Self>> {
        sqlx::query_as::<_, Self>(
            r#"
            SELECT * FROM editions
            WHERE county_id = $1 AND status = 'published'
            ORDER BY period_start DESC
            LIMIT 1
            "#,
        )
        .bind(county_id)
        .fetch_optional(pool)
        .await
        .map_err(Into::into)
    }

    /// Find the latest edition for a county (any status — draft or published).
    pub async fn find_current_for_county(county_id: Uuid, pool: &PgPool) -> Result<Option<Self>> {
        sqlx::query_as::<_, Self>(
            r#"
            SELECT * FROM editions
            WHERE county_id = $1
            ORDER BY period_start DESC
            LIMIT 1
            "#,
        )
        .bind(county_id)
        .fetch_optional(pool)
        .await
        .map_err(Into::into)
    }

    /// Returns the most recent edition for every county (one row per county).
    /// Uses PostgreSQL DISTINCT ON to efficiently pick the latest per county_id.
    pub async fn latest_per_county(pool: &PgPool) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            r#"
            SELECT DISTINCT ON (county_id) *
            FROM editions
            ORDER BY county_id, period_start DESC, created_at DESC
            "#,
        )
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    /// List editions with optional filters.
    pub async fn list(filters: &EditionFilters, pool: &PgPool) -> Result<(Vec<Self>, i64)> {
        let limit = filters.limit.unwrap_or(20);
        let offset = filters.offset.unwrap_or(0);

        let count_row: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*)::bigint FROM editions
            WHERE ($1::uuid IS NULL OR county_id = $1)
              AND ($2::text IS NULL OR status = $2)
              AND ($3::date IS NULL OR period_start = $3)
              AND ($4::date IS NULL OR period_end = $4)
            "#,
        )
        .bind(filters.county_id)
        .bind(&filters.status)
        .bind(filters.period_start)
        .bind(filters.period_end)
        .fetch_one(pool)
        .await?;

        let editions = sqlx::query_as::<_, Self>(
            r#"
            SELECT * FROM editions
            WHERE ($1::uuid IS NULL OR county_id = $1)
              AND ($2::text IS NULL OR status = $2)
              AND ($5::date IS NULL OR period_start = $5)
              AND ($6::date IS NULL OR period_end = $6)
            ORDER BY period_start DESC, created_at DESC
            LIMIT $3 OFFSET $4
            "#,
        )
        .bind(filters.county_id)
        .bind(&filters.status)
        .bind(limit)
        .bind(offset)
        .bind(filters.period_start)
        .bind(filters.period_end)
        .fetch_all(pool)
        .await?;

        Ok((editions, count_row.0))
    }

    /// Transition a draft edition to in_review (editor has opened it).
    pub async fn review(id: Uuid, pool: &PgPool) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            UPDATE editions
            SET status = 'in_review', updated_at = NOW()
            WHERE id = $1 AND status = 'draft'
            RETURNING *
            "#,
        )
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    /// Approve an in_review edition (ready for publication).
    pub async fn approve(id: Uuid, pool: &PgPool) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            UPDATE editions
            SET status = 'approved', updated_at = NOW()
            WHERE id = $1 AND status = 'in_review'
            RETURNING *
            "#,
        )
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    /// Publish an approved edition: set status to 'published' and record the timestamp.
    pub async fn publish(id: Uuid, pool: &PgPool) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            UPDATE editions
            SET status = 'published', published_at = NOW(), updated_at = NOW()
            WHERE id = $1 AND status IN ('approved', 'draft')
            RETURNING *
            "#,
        )
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    /// Archive a published edition.
    pub async fn archive(id: Uuid, pool: &PgPool) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            UPDATE editions
            SET status = 'archived', updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    /// Batch approve multiple in_review editions at once.
    pub async fn batch_approve(ids: &[Uuid], pool: &PgPool) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            r#"
            UPDATE editions
            SET status = 'approved', updated_at = NOW()
            WHERE id = ANY($1) AND status = 'in_review'
            RETURNING *
            "#,
        )
        .bind(ids)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    /// Batch publish multiple approved editions at once.
    pub async fn batch_publish(ids: &[Uuid], pool: &PgPool) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            r#"
            UPDATE editions
            SET status = 'published', published_at = NOW(), updated_at = NOW()
            WHERE id = ANY($1) AND status = 'approved'
            RETURNING *
            "#,
        )
        .bind(ids)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    /// Count editions grouped by status for a given period.
    pub async fn count_by_status(
        period_start: NaiveDate,
        period_end: NaiveDate,
        pool: &PgPool,
    ) -> Result<Vec<(String, i64)>> {
        sqlx::query_as::<_, (String, i64)>(
            r#"
            SELECT status, COUNT(*)::bigint
            FROM editions
            WHERE period_start = $1 AND period_end = $2
            GROUP BY status
            "#,
        )
        .bind(period_start)
        .bind(period_end)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    /// Delete an edition (cascades to rows + slots).
    pub async fn delete(id: Uuid, pool: &PgPool) -> Result<()> {
        sqlx::query("DELETE FROM editions WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Reset an edition back to draft status (used when regenerating a reviewed/approved edition).
    pub async fn reset_to_draft(id: Uuid, pool: &PgPool) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            UPDATE editions
            SET status = 'draft', updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    /// Clear all rows and slots for an edition (used before re-generating).
    pub async fn clear_layout(id: Uuid, pool: &PgPool) -> Result<()> {
        // Delete sections first (ON DELETE SET NULL will unlink rows from sections)
        sqlx::query("DELETE FROM edition_sections WHERE edition_id = $1")
            .bind(id)
            .execute(pool)
            .await?;
        // Then delete rows (cascades to slots and widgets)
        sqlx::query("DELETE FROM edition_rows WHERE edition_id = $1")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }
}
