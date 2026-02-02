//! Research effect - handles fetching or creating domain research
//!
//! This module contains the handler for the AssessWebsiteRequested event.

use crate::common::{JobId, MemberId, WebsiteId};
use crate::domains::chatrooms::ChatRequestState;
use crate::domains::domain_approval::events::DomainApprovalEvent;
use crate::kernel::ServerDeps;
use crate::domains::website::models::{Website, WebsiteResearch, WebsiteResearchHomepage};
use anyhow::{Context, Result};
use seesaw_core::EffectContext;
use tracing::info;

// ============================================================================
// Handler Functions (Business Logic)
// ============================================================================

/// Handle the AssessWebsiteRequested event by fetching or creating research.
pub async fn handle_assess_website(
    website_id: WebsiteId,
    job_id: JobId,
    requested_by: MemberId,
    ctx: &EffectContext<ServerDeps, ChatRequestState>,
) -> Result<DomainApprovalEvent> {
    info!(
        website_id = %website_id,
        job_id = %job_id,
        "Fetching or creating research"
    );

    // Step 1: Fetch website to ensure it exists
    let website = Website::find_by_id(website_id.into(), &ctx.deps().db_pool)
        .await
        .context(format!("Website not found: {}", website_id))?;

    info!(
        website_id = %website_id,
        website_domain = %website.domain,
        "Website found"
    );

    // Step 2: Check for existing research (<7 days old)
    let existing =
        WebsiteResearch::find_latest_by_website_id(website_id.into(), &ctx.deps().db_pool).await?;

    if let Some(research) = existing {
        let age_days = (chrono::Utc::now() - research.created_at).num_days();

        info!(
            research_id = %research.id,
            age_days = age_days,
            "Found existing research"
        );

        if age_days < 7 {
            return Ok(DomainApprovalEvent::WebsiteResearchFound {
                research_id: research.id,
                website_id,
                job_id,
                age_days,
                requested_by,
            });
        }

        info!(research_id = %research.id, "Research is stale, creating fresh research");
    }

    // Step 3: Create fresh research - scrape homepage (with graceful error handling)
    info!(website_domain = %website.domain, "Scraping homepage");

    let homepage_content = match ctx
        .deps()
        .web_scraper
        .scrape(&format!("https://{}", &website.domain))
        .await
    {
        Ok(result) => {
            info!(
                website_domain = %website.domain,
                markdown_length = result.markdown.len(),
                "Homepage scraped successfully"
            );
            Some(result.markdown)
        }
        Err(e) => {
            // Log warning but continue - homepage scraping is not critical
            tracing::warn!(
                website_domain = %website.domain,
                error = %e,
                "Failed to scrape homepage, continuing with search-based research"
            );
            None
        }
    };

    // Step 4: Create research record
    let research = WebsiteResearch::create(
        website_id.into(),
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
    } else {
        info!(research_id = %research.id, "Skipping homepage storage (scrape failed)");
    }

    // Step 6: Emit event - research created, needs searches
    Ok(DomainApprovalEvent::WebsiteResearchCreated {
        research_id: research.id,
        website_id,
        job_id,
        homepage_url: website.domain,
        requested_by,
    })
}
