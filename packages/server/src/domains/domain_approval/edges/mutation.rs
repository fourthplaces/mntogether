use crate::common::{WebsiteId, JobId};
use crate::domains::domain_approval::events::DomainApprovalEvent;
use crate::server::graphql::context::GraphQLContext;
use juniper::{FieldError, FieldResult};
use seesaw_core::dispatch_request;
use tracing::info;
use uuid::Uuid;

/// Generate a comprehensive assessment report for a website
/// This creates a "background check" style markdown report to help with approval decisions
///
/// The assessment process:
/// 1. Fetches or creates research (scrapes homepage if needed)
/// 2. Conducts Tavily searches (background, problems, history)
/// 3. Generates AI assessment with recommendations
///
/// Returns the assessment ID on success
pub async fn generate_website_assessment(
    ctx: &GraphQLContext,
    website_id: String,
) -> FieldResult<String> {
    info!(website_id = %website_id, "Generating website assessment");

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

    // Parse website ID
    let website_uuid = Uuid::parse_str(&website_id)
        .map_err(|e| FieldError::new(format!("Invalid website ID: {}", e), juniper::Value::null()))?;
    let website_id_typed = WebsiteId::from_uuid(website_uuid);

    // Generate job ID
    let job_id = JobId::new();

    // Emit event to trigger assessment workflow and await completion
    let assessment_id = dispatch_request(
        DomainApprovalEvent::AssessWebsiteRequested {
            website_id: website_id_typed,
            job_id,
            requested_by: user.member_id,
        },
        &ctx.bus,
        |m| {
            m.try_match(|e: &DomainApprovalEvent| match e {
                // Success - assessment workflow complete
                DomainApprovalEvent::WebsiteAssessmentCompleted {
                    website_id: completed_id,
                    job_id: completed_job_id,
                    assessment_id,
                    ..
                } if *completed_id == website_id_typed && *completed_job_id == job_id => {
                    Some(Ok(*assessment_id))
                }
                // Failure - any step in the chain failed
                DomainApprovalEvent::WebsiteResearchFailed {
                    website_id: failed_id,
                    job_id: failed_job_id,
                    reason,
                } if *failed_id == website_id_typed && *failed_job_id == job_id => {
                    Some(Err(anyhow::anyhow!("Research failed: {}", reason)))
                }
                DomainApprovalEvent::ResearchSearchesFailed {
                    website_id: failed_id,
                    job_id: failed_job_id,
                    reason,
                    ..
                } if *failed_id == website_id_typed && *failed_job_id == job_id => {
                    Some(Err(anyhow::anyhow!("Search failed: {}", reason)))
                }
                DomainApprovalEvent::AssessmentGenerationFailed {
                    website_id: failed_id,
                    job_id: failed_job_id,
                    reason,
                } if *failed_id == website_id_typed && *failed_job_id == job_id => {
                    Some(Err(anyhow::anyhow!("Assessment generation failed: {}", reason)))
                }
                _ => None,
            })
            .result()
        },
    )
    .await
    .map_err(|e| {
        FieldError::new(
            format!("Assessment failed: {}", e),
            juniper::Value::null(),
        )
    })?;

    info!(
        website_id = %website_id,
        assessment_id = %assessment_id,
        "Assessment completed successfully"
    );

    // Return the assessment ID as a string
    Ok(assessment_id.to_string())
}
