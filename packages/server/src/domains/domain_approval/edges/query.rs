use crate::domains::domain_approval::data::WebsiteAssessmentData;
use crate::domains::scraping::models::WebsiteAssessment;
use crate::server::graphql::context::GraphQLContext;
use juniper::{FieldError, FieldResult};
use uuid::Uuid;

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

    let website_uuid = Uuid::parse_str(&website_id)
        .map_err(|e| FieldError::new(format!("Invalid website ID: {}", e), juniper::Value::null()))?;

    let assessment = WebsiteAssessment::find_latest_by_website_id(website_uuid, &ctx.db_pool)
        .await
        .map_err(|e| FieldError::new(format!("Database error: {}", e), juniper::Value::null()))?;

    Ok(assessment.map(Into::into))
}
