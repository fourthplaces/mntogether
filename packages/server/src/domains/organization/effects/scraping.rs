// Scraping functions - domain logic for web scraping operations
//
// These are pure functions that use infrastructure (BaseWebScraper) from the kernel.

use anyhow::{Context, Result};
use sqlx::PgPool;

use crate::common::WebsiteId;
use crate::domains::scraping::models::Website;
use crate::kernel::{BaseWebScraper, ScrapeResult};

/// Scrape a website source and return the content
///
/// This function:
/// 1. Fetches the source from the database
/// 2. Scrapes the URL using the web scraper
/// 3. Updates the last_scraped_at timestamp
/// 4. Returns the scrape result
pub async fn scrape_source(
    source_id: WebsiteId,
    web_scraper: &dyn BaseWebScraper,
    db_pool: &PgPool,
) -> Result<(Website, ScrapeResult)> {
    // Get source from database using model layer
    let source = Website::find_by_id(source_id, db_pool)
        .await
        .context("Failed to find source")?;

    // Scrape website using web scraper
    let scrape_result = web_scraper
        .scrape(&source.url)
        .await
        .context("Web scraping failed")?;

    // Update last_scraped_at timestamp
    Website::update_last_scraped(source_id, db_pool)
        .await
        .context("Failed to update last_scraped_at")?;

    Ok((source, scrape_result))
}
