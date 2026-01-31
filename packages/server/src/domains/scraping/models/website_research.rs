use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::FromRow;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow)]
pub struct WebsiteResearch {
    pub id: Uuid,
    pub website_id: Uuid,
    pub homepage_url: String,
    pub homepage_fetched_at: DateTime<Utc>,
    pub tavily_searches_completed_at: Option<DateTime<Utc>>,
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl WebsiteResearch {
    pub async fn find_latest_by_website_id(
        website_id: Uuid,
        pool: &PgPool,
    ) -> Result<Option<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM website_research WHERE website_id = $1 ORDER BY created_at DESC LIMIT 1",
        )
        .bind(website_id)
        .fetch_optional(pool)
        .await
        .map_err(Into::into)
    }

    pub async fn create(
        website_id: Uuid,
        homepage_url: String,
        created_by: Option<Uuid>,
        pool: &PgPool,
    ) -> Result<Self> {
        let now = chrono::Utc::now();
        sqlx::query_as::<_, Self>(
            "INSERT INTO website_research (website_id, homepage_url, homepage_fetched_at, created_by)
             VALUES ($1, $2, $3, $4) RETURNING *"
        )
        .bind(website_id)
        .bind(homepage_url)
        .bind(now)
        .bind(created_by)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    pub async fn mark_tavily_complete(&self, pool: &PgPool) -> Result<()> {
        sqlx::query(
            "UPDATE website_research SET tavily_searches_completed_at = NOW() WHERE id = $1",
        )
        .bind(self.id)
        .execute(pool)
        .await?;
        Ok(())
    }
}

#[derive(Debug, Clone, FromRow)]
pub struct WebsiteResearchHomepage {
    pub id: Uuid,
    pub website_research_id: Uuid,
    pub html: Option<String>,
    pub markdown: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl WebsiteResearchHomepage {
    pub async fn create(
        website_research_id: Uuid,
        html: Option<String>,
        markdown: Option<String>,
        pool: &PgPool,
    ) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            "INSERT INTO website_research_homepage (website_research_id, html, markdown)
             VALUES ($1, $2, $3) RETURNING *",
        )
        .bind(website_research_id)
        .bind(html)
        .bind(markdown)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    pub async fn find_by_research_id(research_id: Uuid, pool: &PgPool) -> Result<Option<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM website_research_homepage WHERE website_research_id = $1",
        )
        .bind(research_id)
        .fetch_optional(pool)
        .await
        .map_err(Into::into)
    }
}

#[derive(Debug, Clone, FromRow)]
pub struct TavilySearchQuery {
    pub id: Uuid,
    pub website_research_id: Uuid,
    pub query: String,
    pub search_depth: Option<String>,
    pub max_results: Option<i32>,
    pub days_filter: Option<i32>,
    pub executed_at: DateTime<Utc>,
}

impl TavilySearchQuery {
    pub async fn create(
        website_research_id: Uuid,
        query: String,
        search_depth: Option<String>,
        max_results: Option<i32>,
        days_filter: Option<i32>,
        pool: &PgPool,
    ) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            "INSERT INTO tavily_search_queries (website_research_id, query, search_depth, max_results, days_filter)
             VALUES ($1, $2, $3, $4, $5) RETURNING *"
        )
        .bind(website_research_id)
        .bind(query)
        .bind(search_depth)
        .bind(max_results)
        .bind(days_filter)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    pub async fn find_by_research_id(research_id: Uuid, pool: &PgPool) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM tavily_search_queries WHERE website_research_id = $1 ORDER BY executed_at ASC"
        )
        .bind(research_id)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }
}

#[derive(Debug, Clone, FromRow)]
pub struct TavilySearchResult {
    pub id: Uuid,
    pub query_id: Uuid,
    pub title: String,
    pub url: String,
    pub content: String,
    pub score: f64,
    pub published_date: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl TavilySearchResult {
    pub async fn create_batch(
        query_id: Uuid,
        results: Vec<(String, String, String, f64, Option<String>)>,
        pool: &PgPool,
    ) -> Result<Vec<Self>> {
        let mut created = Vec::new();
        for (title, url, content, score, published_date) in results {
            let result = sqlx::query_as::<_, Self>(
                "INSERT INTO tavily_search_results (query_id, title, url, content, score, published_date)
                 VALUES ($1, $2, $3, $4, $5, $6) RETURNING *"
            )
            .bind(query_id)
            .bind(title)
            .bind(url)
            .bind(content)
            .bind(score)
            .bind(published_date)
            .fetch_one(pool)
            .await?;
            created.push(result);
        }
        Ok(created)
    }

    pub async fn find_by_query_id(query_id: Uuid, pool: &PgPool) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM tavily_search_results WHERE query_id = $1 ORDER BY score DESC",
        )
        .bind(query_id)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }
}
