//! Scraping actions - entry-point functions for scraping operations

use anyhow::{Context, Result};
use tracing::info;

use crate::common::JobId;
use crate::domains::website::models::{CreateWebsite, Website};
use crate::kernel::ServerDeps;

/// Extract domain from URL (e.g., "https://example.org/path" -> "example.org")
pub fn extract_domain(url: &str) -> Option<String> {
    url::Url::parse(url).ok().and_then(|parsed| {
        parsed
            .host_str()
            .map(|host| host.strip_prefix("www.").unwrap_or(host).to_lowercase())
    })
}

/// Result of submitting a resource link
#[derive(Debug, Clone)]
pub enum ResourceLinkSubmission {
    /// Website is pending admin approval (new or unapproved)
    PendingApproval { url: String },
    /// Website is approved, processing can begin
    Processing {
        job_id: JobId,
        url: String,
        submitter_contact: Option<String>,
    },
}

/// Submit a resource link for processing (public - no auth required)
/// Returns the submission status.
pub async fn submit_resource_link(
    url: String,
    submitter_contact: Option<String>,
    deps: &ServerDeps,
) -> Result<ResourceLinkSubmission> {
    url::Url::parse(&url).context("Invalid URL format")?;

    let job_id = JobId::new();

    info!(
        url = %url,
        job_id = %job_id,
        "Processing submitted resource link"
    );

    let _domain = extract_domain(&url)
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
        Ok(ResourceLinkSubmission::PendingApproval { url })
    } else {
        Ok(ResourceLinkSubmission::Processing {
            job_id,
            url,
            submitter_contact,
        })
    }
}
