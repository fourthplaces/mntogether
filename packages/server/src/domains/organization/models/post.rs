use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sqlx::PgPool;
use uuid::Uuid;

/// Post status
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq, Eq)]
#[sqlx(type_name = "post_status", rename_all = "lowercase")]
pub enum PostStatus {
    Draft,
    Published,
    Expired,
    Archived,
}

impl ToString for PostStatus {
    fn to_string(&self) -> String {
        match self {
            PostStatus::Draft => "draft".to_string(),
            PostStatus::Published => "published".to_string(),
            PostStatus::Expired => "expired".to_string(),
            PostStatus::Archived => "archived".to_string(),
        }
    }
}

/// Post - temporal announcement created when need is approved
///
/// Key concept: Needs = reality, Posts = announcements about that reality
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Post {
    pub id: Uuid,
    pub need_id: Uuid,
    pub status: PostStatus,
    pub published_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub custom_title: Option<String>,
    pub custom_description: Option<String>,
    pub custom_tldr: Option<String>,
    pub targeting_hints: Option<JsonValue>,
    pub outreach_copy: Option<String>,
    pub view_count: i32,
    pub click_count: i32,
    pub response_count: i32,
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// =============================================================================
// SQL Queries - ALL queries must be in models/
// =============================================================================

impl Post {
    /// Create and publish a post (default: expires in 5 days)
    pub async fn create_and_publish(
        need_id: Uuid,
        created_by: Option<Uuid>,
        expires_in_days: Option<i64>,
        pool: &PgPool,
    ) -> Result<Self> {
        let now = Utc::now();
        let expires_at = now + Duration::days(expires_in_days.unwrap_or(5));

        let post = sqlx::query_as::<_, Post>(
            r#"
            INSERT INTO posts (
                need_id,
                status,
                published_at,
                expires_at,
                created_by
            )
            VALUES ($1, 'published', $2, $3, $4)
            RETURNING *
            "#,
        )
        .bind(need_id)
        .bind(now)
        .bind(expires_at)
        .bind(created_by)
        .fetch_one(pool)
        .await?;
        Ok(post)
    }

    /// Create and publish a post with customizations
    pub async fn create_and_publish_custom(
        need_id: Uuid,
        created_by: Option<Uuid>,
        custom_title: Option<String>,
        custom_description: Option<String>,
        custom_tldr: Option<String>,
        targeting_hints: Option<JsonValue>,
        expires_in_days: Option<i64>,
        pool: &PgPool,
    ) -> Result<Self> {
        let now = Utc::now();
        let expires_at = now + Duration::days(expires_in_days.unwrap_or(5));

        let post = sqlx::query_as::<_, Post>(
            r#"
            INSERT INTO posts (
                need_id,
                status,
                published_at,
                expires_at,
                custom_title,
                custom_description,
                custom_tldr,
                targeting_hints,
                created_by
            )
            VALUES ($1, 'published', $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
            "#,
        )
        .bind(need_id)
        .bind(now)
        .bind(expires_at)
        .bind(custom_title)
        .bind(custom_description)
        .bind(custom_tldr)
        .bind(targeting_hints)
        .bind(created_by)
        .fetch_one(pool)
        .await?;
        Ok(post)
    }

    /// Find post by ID
    pub async fn find_by_id(id: Uuid, pool: &PgPool) -> Result<Option<Self>> {
        let post = sqlx::query_as::<_, Post>("SELECT * FROM posts WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await?;
        Ok(post)
    }

    /// Find all posts for a need
    pub async fn find_by_need_id(need_id: Uuid, pool: &PgPool) -> Result<Vec<Self>> {
        let posts = sqlx::query_as::<_, Post>(
            "SELECT * FROM posts WHERE need_id = $1 ORDER BY published_at DESC",
        )
        .bind(need_id)
        .fetch_all(pool)
        .await?;
        Ok(posts)
    }

    /// Find latest post for a need
    pub async fn find_latest_by_need_id(need_id: Uuid, pool: &PgPool) -> Result<Option<Self>> {
        let post = sqlx::query_as::<_, Post>(
            "SELECT * FROM posts WHERE need_id = $1 ORDER BY published_at DESC LIMIT 1",
        )
        .bind(need_id)
        .fetch_optional(pool)
        .await?;
        Ok(post)
    }

    /// Find all published posts (for notification engine)
    pub async fn find_published(limit: Option<i64>, pool: &PgPool) -> Result<Vec<Self>> {
        let posts = sqlx::query_as::<_, Post>(
            r#"
            SELECT * FROM posts
            WHERE status = 'published'
              AND (expires_at IS NULL OR expires_at > NOW())
            ORDER BY published_at DESC
            LIMIT $1
            "#,
        )
        .bind(limit.unwrap_or(100))
        .fetch_all(pool)
        .await?;
        Ok(posts)
    }

    /// Find posts that need to be expired
    pub async fn find_needing_expiration(pool: &PgPool) -> Result<Vec<Self>> {
        let posts = sqlx::query_as::<_, Post>(
            r#"
            SELECT * FROM posts
            WHERE status = 'published'
              AND expires_at IS NOT NULL
              AND expires_at <= NOW()
            "#,
        )
        .fetch_all(pool)
        .await?;
        Ok(posts)
    }

    /// Update post status
    pub async fn update_status(id: Uuid, status: PostStatus, pool: &PgPool) -> Result<Self> {
        let post = sqlx::query_as::<_, Post>(
            r#"
            UPDATE posts
            SET status = $2
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(status)
        .fetch_one(pool)
        .await?;
        Ok(post)
    }

    /// Expire a post
    pub async fn expire(id: Uuid, pool: &PgPool) -> Result<Self> {
        Self::update_status(id, PostStatus::Expired, pool).await
    }

    /// Archive a post
    pub async fn archive(id: Uuid, pool: &PgPool) -> Result<Self> {
        Self::update_status(id, PostStatus::Archived, pool).await
    }

    /// Increment view count
    pub async fn increment_view_count(id: Uuid, pool: &PgPool) -> Result<()> {
        sqlx::query("UPDATE posts SET view_count = view_count + 1 WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Increment click count
    pub async fn increment_click_count(id: Uuid, pool: &PgPool) -> Result<()> {
        sqlx::query("UPDATE posts SET click_count = click_count + 1 WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Increment response count
    pub async fn increment_response_count(id: Uuid, pool: &PgPool) -> Result<()> {
        sqlx::query("UPDATE posts SET response_count = response_count + 1 WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Expire all posts past their expiration date (background job)
    pub async fn expire_old_posts(pool: &PgPool) -> Result<usize> {
        let result = sqlx::query(
            r#"
            UPDATE posts
            SET status = 'expired'
            WHERE status = 'published'
              AND expires_at IS NOT NULL
              AND expires_at <= NOW()
            "#,
        )
        .execute(pool)
        .await?;

        Ok(result.rows_affected() as usize)
    }

    /// Update outreach copy for a post
    pub async fn update_outreach_copy(
        id: Uuid,
        outreach_copy: String,
        pool: &PgPool,
    ) -> Result<Self> {
        let post = sqlx::query_as::<_, Post>(
            r#"
            UPDATE posts
            SET outreach_copy = $2
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(outreach_copy)
        .fetch_one(pool)
        .await?;
        Ok(post)
    }
}
