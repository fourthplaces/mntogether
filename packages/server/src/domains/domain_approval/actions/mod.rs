//! Domain Approval actions
//!
//! Entry-point actions are called directly from GraphQL mutations via `process()`.
//! They do the work synchronously and return values.
//!
//! Actions are self-contained: they take raw Uuid types, handle conversions,
//! and return simple values.
//!
//! Flow:
//! - If fresh research exists → generate assessment directly (synchronous)
//! - If research is stale/missing → create research, trigger search cascade (async)

use crate::common::{AppState, JobId, MemberId, WebsiteId};
use crate::domains::domain_approval::effects::assessment::generate_assessment;
use crate::domains::domain_approval::events::DomainApprovalEvent;
use crate::domains::website::models::{Website, WebsiteResearch, WebsiteResearchHomepage};
use crate::kernel::ServerDeps;
use anyhow::{Context, Result};
use seesaw_core::EffectContext;
use tracing::info;
use uuid::Uuid;

/// Result of a website assessment operation
#[derive(Debug, Clone)]
pub struct AssessmentResult {
    pub job_id: Uuid,
    pub website_id: Uuid,
    pub assessment_id: Option<Uuid>,
    pub status: String,
    pub message: Option<String>,
}

// ============================================================================
// Entry Point: Assess Website
// ============================================================================

/// Assess a website by fetching/creating research and generating an assessment.
///
/// If fresh research exists (< 7 days old), generates assessment synchronously.
/// If research needs to be created, starts async workflow (searches then assessment).
pub async fn assess_website(
    website_id: Uuid,
    member_id: Uuid,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<AssessmentResult> {
    let website_id_typed = WebsiteId::from_uuid(website_id);
    let requested_by = MemberId::from_uuid(member_id);
    let job_id = JobId::new();

    info!(
        website_id = %website_id,
        job_id = %job_id,
        "Starting website assessment"
    );

    // Step 1: Fetch website to ensure it exists
    let website = Website::find_by_id(website_id_typed.into(), &ctx.deps().db_pool)
        .await
        .context(format!("Website not found: {}", website_id))?;

    info!(
        website_id = %website_id,
        website_domain = %website.domain,
        "Website found"
    );

    // Step 2: Check for existing fresh research (<7 days old)
    let existing =
        WebsiteResearch::find_latest_by_website_id(website_id_typed.into(), &ctx.deps().db_pool)
            .await?;

    if let Some(research) = existing {
        let age_days = (chrono::Utc::now() - research.created_at).num_days();

        info!(
            research_id = %research.id,
            age_days = age_days,
            "Found existing research"
        );

        if age_days < 7 {
            // Fresh research exists - generate assessment synchronously
            info!(
                research_id = %research.id,
                "Research is fresh, generating assessment directly"
            );

            let assessment =
                generate_assessment(research.id, website_id_typed, job_id, requested_by, ctx)
                    .await?;

            return Ok(AssessmentResult {
                job_id: job_id.into_uuid(),
                website_id,
                assessment_id: Some(assessment.id),
                status: "completed".to_string(),
                message: Some(format!(
                    "Assessment generated using existing research ({} days old)",
                    age_days
                )),
            });
        }

        info!(research_id = %research.id, "Research is stale, creating fresh research");
    }

    // Step 3: Create fresh research - fetch homepage using ingestor
    info!(website_domain = %website.domain, "Fetching homepage");

    let homepage_url = format!("https://{}", &website.domain);
    let homepage_content = match ctx
        .deps()
        .ingestor
        .fetch_one(&homepage_url)
        .await
    {
        Ok(page) => {
            info!(
                website_domain = %website.domain,
                content_length = page.content.len(),
                "Homepage fetched successfully"
            );
            Some(page.content)
        }
        Err(e) => {
            tracing::warn!(
                website_domain = %website.domain,
                error = %e,
                "Failed to fetch homepage, continuing with search-based research"
            );
            None
        }
    };

    // Step 4: Create research record
    let research = WebsiteResearch::create(
        website_id_typed.into(),
        website.domain.clone(),
        Some(requested_by.into()),
        &ctx.deps().db_pool,
    )
    .await
    .context("Failed to create research record")?;

    info!(research_id = %research.id, "Research record created");

    // Step 5: Store homepage content (if available)
    if let Some(content) = homepage_content {
        WebsiteResearchHomepage::create(
            research.id,
            Some(content.clone()),
            Some(content),
            &ctx.deps().db_pool,
        )
        .await
        .context("Failed to store homepage content")?;

        info!(research_id = %research.id, "Homepage content stored");
    }

    // Step 6: Emit event to trigger async search cascade
    // Flow: WebsiteResearchCreated → conduct_searches → ResearchSearchesCompleted → generate_assessment
    ctx.emit(DomainApprovalEvent::WebsiteResearchCreated {
        research_id: research.id,
        website_id: website_id_typed,
        job_id,
        homepage_url: website.domain,
        requested_by,
    });

    Ok(AssessmentResult {
        job_id: job_id.into_uuid(),
        website_id,
        assessment_id: None, // Will be created by search cascade
        status: "processing".to_string(),
        message: Some("Research created, running web searches...".to_string()),
    })
}
