use crate::domains::scraping::models::WebsiteAssessment;
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
