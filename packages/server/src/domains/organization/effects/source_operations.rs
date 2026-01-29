// Domain functions for organization source operations
//
// These functions contain the business logic for source CRUD operations,
// separated from the thin Effect orchestrator.

use anyhow::{Context, Result};
use sqlx::PgPool;
use url::Url;

use crate::common::SourceId;
use crate::domains::organization::models::source::OrganizationSource;

/// Validate URL format
pub fn validate_url(url: &str) -> Result<()> {
    Url::parse(url)
        .context("Invalid URL format")?;
    Ok(())
}

/// Add a scrape URL to an organization source
pub async fn add_scrape_url(
    source_id: SourceId,
    url: String,
    pool: &PgPool,
) -> Result<()> {
    // Validate URL format
    validate_url(&url)?;

    OrganizationSource::add_scrape_url(source_id, url, pool)
        .await
        .context("Failed to add scrape URL")
}

/// Remove a scrape URL from an organization source
pub async fn remove_scrape_url(
    source_id: SourceId,
    url: String,
    pool: &PgPool,
) -> Result<()> {
    OrganizationSource::remove_scrape_url(source_id, url, pool)
        .await
        .context("Failed to remove scrape URL")
}
