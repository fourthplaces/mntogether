use crate::domains::scraping::models::{WebsiteAssessment, WebsiteSearchResult};
use chrono::{DateTime, Utc};
use juniper::GraphQLObject;
use uuid::Uuid;

/// GraphQL representation of a website assessment
#[derive(Debug, Clone, GraphQLObject)]
#[graphql(description = "AI-generated assessment report for a website")]
pub struct WebsiteAssessmentData {
    pub id: Uuid,
    pub website_id: Uuid,
    pub assessment_markdown: String,
    pub recommendation: String,
    pub confidence_score: Option<f64>,
    pub organization_name: Option<String>,
    pub founded_year: Option<i32>,
    pub generated_at: DateTime<Utc>,
    pub model_used: String,
    pub reviewed_by_human: bool,
}

impl From<WebsiteAssessment> for WebsiteAssessmentData {
    fn from(assessment: WebsiteAssessment) -> Self {
        Self {
            id: assessment.id,
            website_id: assessment.website_id,
            assessment_markdown: assessment.assessment_markdown,
            recommendation: assessment.recommendation,
            confidence_score: assessment.confidence_score,
            organization_name: assessment.organization_name,
            founded_year: assessment.founded_year,
            generated_at: assessment.generated_at,
            model_used: assessment.model_used,
            reviewed_by_human: assessment.reviewed_by_human,
        }
    }
}

/// GraphQL representation of a website search result
#[derive(Debug, Clone, GraphQLObject)]
#[graphql(description = "Website found via semantic search")]
pub struct WebsiteSearchResultData {
    pub website_id: Uuid,
    pub assessment_id: Uuid,
    pub website_url: String,
    pub organization_name: Option<String>,
    pub recommendation: String,
    pub assessment_markdown: String,
    pub similarity: f64,
}

impl From<WebsiteSearchResult> for WebsiteSearchResultData {
    fn from(result: WebsiteSearchResult) -> Self {
        Self {
            website_id: result.website_id,
            assessment_id: result.assessment_id,
            website_url: result.website_url,
            organization_name: result.organization_name,
            recommendation: result.recommendation,
            assessment_markdown: result.assessment_markdown,
            similarity: result.similarity,
        }
    }
}
