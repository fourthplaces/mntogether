//! Scraping actions - entry-point functions for scraping operations
//!
//! These are called directly from GraphQL mutations via `process()`.
//! Actions are self-contained: they take raw input, handle ID parsing,
//! auth checks, and return results directly.

use anyhow::{Context, Result};
use seesaw_core::EffectContext;
use tracing::info;
use uuid::Uuid;

use crate::common::{AppState, JobId};
use crate::domains::posts::effects::post::extract_domain;
use crate::domains::posts::events::PostEvent;
use crate::domains::website::models::Website;
use crate::kernel::ServerDeps;

/// Result of submitting a resource link
#[derive(Debug, Clone)]
pub struct SubmitResourceLinkResult {
    pub job_id: Uuid,
    pub status: String,
    pub message: String,
}

/// Submit a resource link for processing (public - no auth required)
/// Returns submission result directly.
pub async fn submit_resource_link(
    url: String,
    submitter_contact: Option<String>,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<SubmitResourceLinkResult> {
    url::Url::parse(&url).context("Invalid URL format")?;

    let job_id = JobId::new();

    // Extract organization name from URL domain
    let organization_name = url
        .split("//")
        .nth(1)
        .and_then(|s| s.split('/').next())
        .unwrap_or("Unknown Organization")
        .to_string();

    info!(
        url = %url,
        organization_name = %organization_name,
        job_id = %job_id,
        "Processing submitted resource link"
    );

    let domain = extract_domain(&url)
        .ok_or_else(|| anyhow::anyhow!("Invalid URL: could not extract domain"))?;

    let source = Website::find_or_create(
        url.clone(),
        None,
        "public_user".to_string(),
        submitter_contact.clone(),
        3,
        &ctx.deps().db_pool,
    )
    .await?;

    info!(
        source_id = %source.id,
        domain = %source.domain,
        status = %source.status,
        "Found or created website"
    );

    if source.status == "pending_review" {
        ctx.emit(PostEvent::WebsitePendingApproval {
            website_id: source.id,
            url: domain,
            submitted_url: url,
            submitter_contact,
        });
    } else {
        ctx.emit(PostEvent::WebsiteCreatedFromLink {
            source_id: source.id,
            job_id,
            url,
            organization_name,
            submitter_contact,
        });
    }

    Ok(SubmitResourceLinkResult {
        job_id: job_id.into_uuid(),
        status: "pending".to_string(),
        message: "Resource submitted successfully! We'll process it shortly.".to_string(),
    })
}
