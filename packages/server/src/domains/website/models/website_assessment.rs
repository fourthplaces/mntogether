use anyhow::Result;
use chrono::{DateTime, Utc};
use pgvector::Vector;
use sqlx::FromRow;
use sqlx::PgPool;
use uuid::Uuid;

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

/// Search result with similarity score
#[derive(Debug, Clone, FromRow)]
pub struct WebsiteSearchResult {
    pub website_id: Uuid,
    pub assessment_id: Uuid,
    pub website_domain: String,
    pub organization_name: Option<String>,
    pub recommendation: String,
    pub assessment_markdown: String,
    pub similarity: f64,
}

impl WebsiteAssessment {
    pub async fn find_latest_by_website_id(
        website_id: Uuid,
        pool: &PgPool,
    ) -> Result<Option<Self>> {
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
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) RETURNING *",
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

    /// Update the embedding for this assessment
    pub async fn update_embedding(id: Uuid, embedding: &[f32], pool: &PgPool) -> Result<()> {
        let vector = Vector::from(embedding.to_vec());

        sqlx::query("UPDATE website_assessments SET embedding = $2 WHERE id = $1")
            .bind(id)
            .bind(vector)
            .execute(pool)
            .await?;

        Ok(())
    }

    /// Search websites by semantic similarity to a query embedding
    pub async fn search_by_similarity(
        query_embedding: &[f32],
        match_threshold: f32,
        limit: i32,
        pool: &PgPool,
    ) -> Result<Vec<WebsiteSearchResult>> {
        let vector = Vector::from(query_embedding.to_vec());

        let results = sqlx::query_as::<_, WebsiteSearchResult>(
            r#"
            SELECT
                w.id as website_id,
                wa.id as assessment_id,
                w.domain as website_domain,
                wa.organization_name,
                wa.recommendation,
                wa.assessment_markdown,
                (1 - (wa.embedding <=> $1))::float8 as similarity
            FROM website_assessments wa
            JOIN websites w ON w.id = wa.website_id
            WHERE wa.embedding IS NOT NULL
                AND (1 - (wa.embedding <=> $1)) > $2
            ORDER BY wa.embedding <=> $1
            LIMIT $3
            "#,
        )
        .bind(vector)
        .bind(match_threshold)
        .bind(limit)
        .fetch_all(pool)
        .await?;

        Ok(results)
    }

    /// Find assessments that don't have embeddings yet
    pub async fn find_without_embeddings(pool: &PgPool) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM website_assessments WHERE embedding IS NULL ORDER BY created_at DESC",
        )
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }
}
