//! Scraping actions - entry-point functions for scraping operations
//!
//! These are called directly from GraphQL mutations via `process()`.
//! Actions are self-contained: they take raw input, handle ID parsing,
//! auth checks, and return events directly.

use anyhow::{Context, Result};
use tracing::info;

use crate::common::JobId;
use crate::domains::posts::effects::post::extract_domain;
use crate::domains::posts::events::PostEvent;
use crate::domains::website::models::{CreateWebsite, Website};
use crate::kernel::ServerDeps;

/// Submit a resource link for processing (public - no auth required)
/// Returns the appropriate event (WebsitePendingApproval or WebsiteCreatedFromLink).
pub async fn submit_resource_link(
    url: String,
    submitter_contact: Option<String>,
    deps: &ServerDeps,
) -> Result<PostEvent> {
    url::Url::parse(&url).context("Invalid URL format")?;

    let job_id = JobId::new();

    info!(
        url = %url,
        job_id = %job_id,
        "Processing submitted resource link"
    );

    let domain = extract_domain(&url)
        .ok_or_else(|| anyhow::anyhow!("Invalid URL: could not extract domain"))?;

    let source = Website::find_or_create(
        CreateWebsite::builder()
            .url_or_domain(url.clone())
            .submitter_type("public_user")
            .submission_context(submitter_contact.clone())
            .max_crawl_depth(3)
            .build(),
        &deps.db_pool,
    )
    .await?;

    info!(
        source_id = %source.id,
        domain = %source.domain,
        status = %source.status,
        "Found or created website"
    );

    if source.status == "pending_review" {
        Ok(PostEvent::WebsitePendingApproval {
            website_id: source.id,
            url: domain,
            submitted_url: url,
            submitter_contact,
        })
    } else {
        Ok(PostEvent::WebsiteCreatedFromLink {
            source_id: source.id,
            job_id,
            url,
            submitter_contact,
        })
    }
}
