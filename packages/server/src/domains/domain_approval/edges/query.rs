use crate::domains::domain_approval::data::{WebsiteAssessmentData, WebsiteSearchResultData};
use crate::domains::website::models::WebsiteAssessment;
use crate::server::graphql::context::GraphQLContext;
use juniper::{FieldError, FieldResult};
use uuid::Uuid;

/// Search websites semantically using natural language queries
///
/// Example queries:
/// - "find me a law firm helping immigrants"
/// - "food shelves in Minneapolis"
/// - "mental health services for teenagers"
pub async fn search_websites_semantic(
    ctx: &GraphQLContext,
    query: String,
    limit: Option<i32>,
    threshold: Option<f64>,
) -> FieldResult<Vec<WebsiteSearchResultData>> {
    // Generate embedding for the search query
    let embedding_response = ctx
        .openai_client
        .create_embedding(&query)
        .await
        .map_err(|e| {
            FieldError::new(
                format!("Failed to generate embedding: {}", e),
                juniper::Value::null(),
            )
        })?;

    let query_embedding = embedding_response
        .data
        .first()
        .map(|d| d.embedding.clone())
        .ok_or_else(|| FieldError::new("No embedding returned", juniper::Value::null()))?;

    // Search by similarity
    let match_threshold = threshold.unwrap_or(0.5) as f32;
    let match_limit = limit.unwrap_or(20);

    let results = WebsiteAssessment::search_by_similarity(
        &query_embedding,
        match_threshold,
        match_limit,
        &ctx.db_pool,
    )
    .await
    .map_err(|e| FieldError::new(format!("Search failed: {}", e), juniper::Value::null()))?;

    Ok(results.into_iter().map(Into::into).collect())
}

/// Get the latest assessment for a website
pub async fn website_assessment(
    ctx: &GraphQLContext,
    website_id: String,
) -> FieldResult<Option<WebsiteAssessmentData>> {
    // Admin auth check
    let user = ctx
        .auth_user
        .as_ref()
        .ok_or_else(|| FieldError::new("Authentication required", juniper::Value::null()))?;

    if !user.is_admin {
        return Err(FieldError::new(
            "Admin access required",
            juniper::Value::null(),
        ));
    }

    let website_uuid = Uuid::parse_str(&website_id).map_err(|e| {
        FieldError::new(format!("Invalid website ID: {}", e), juniper::Value::null())
    })?;

    let assessment = WebsiteAssessment::find_latest_by_website_id(website_uuid, &ctx.db_pool)
        .await
        .map_err(|e| FieldError::new(format!("Database error: {}", e), juniper::Value::null()))?;

    Ok(assessment.map(Into::into))
}
