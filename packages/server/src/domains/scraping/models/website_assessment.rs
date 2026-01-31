use sqlx::FromRow;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use anyhow::Result;
use sqlx::PgPool;

#[derive(Debug, Clone, FromRow)]
pub struct WebsiteAssessment {
    pub id: Uuid,
    pub website_id: Uuid,
    pub website_research_id: Option<Uuid>,
    pub assessment_markdown: String,
    pub recommendation: String,
    pub confidence_score: Option<f64>,
    pub organization_name: Option<String>,
    pub founded_year: Option<i32>,
    pub generated_by: Option<Uuid>,
    pub generated_at: DateTime<Utc>,
    pub model_used: String,
    pub reviewed_by_human: bool,
    pub human_notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl WebsiteAssessment {
    pub async fn find_latest_by_website_id(website_id: Uuid, pool: &PgPool) -> Result<Option<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM website_assessments WHERE website_id = $1 ORDER BY generated_at DESC LIMIT 1"
        )
        .bind(website_id)
        .fetch_optional(pool)
        .await
        .map_err(Into::into)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn create(
        website_id: Uuid,
        website_research_id: Option<Uuid>,
        assessment_markdown: String,
        recommendation: String,
        confidence_score: Option<f64>,
        organization_name: Option<String>,
        founded_year: Option<i32>,
        generated_by: Option<Uuid>,
        model_used: String,
        pool: &PgPool,
    ) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            "INSERT INTO website_assessments (
                website_id, website_research_id, assessment_markdown, recommendation,
                confidence_score, organization_name, founded_year, generated_by, model_used
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) RETURNING *"
        )
        .bind(website_id)
        .bind(website_research_id)
        .bind(assessment_markdown)
        .bind(recommendation)
        .bind(confidence_score)
        .bind(organization_name)
        .bind(founded_year)
        .bind(generated_by)
        .bind(model_used)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }
}
