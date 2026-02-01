//! Pass 1: Summarize individual pages with caching
//!
//! Each page is summarized to extract meaningful content.
//! Results are cached by content hash - if page content hasn't changed,
//! we reuse the cached summary.

use anyhow::Result;
use futures::future::join_all;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use tracing::info;

use crate::domains::scraping::models::PageSummary;
use crate::kernel::BaseAI;

use super::types::{PageToSummarize, SummarizedPage};

/// Summarize a page, using cache if available
pub async fn summarize_page(
    page: &PageToSummarize,
    ai: &dyn BaseAI,
    pool: &PgPool,
) -> Result<SummarizedPage> {
    // Check cache first
    if let Some(cached) = PageSummary::find_by_hash(&page.content_hash, pool).await? {
        info!(url = %page.url, "Using cached page summary");
        return Ok(SummarizedPage {
            snapshot_id: page.snapshot_id,
            url: page.url.clone(),
            content: cached.content,
        });
    }

    // Cache miss - generate summary
    info!(url = %page.url, content_len = page.raw_content.len(), "Generating page summary");
    let content = generate_page_content(ai, &page.url, &page.raw_content).await?;

    // Store in cache
    PageSummary::create(page.snapshot_id, &page.content_hash, &content, pool).await?;

    Ok(SummarizedPage {
        snapshot_id: page.snapshot_id,
        url: page.url.clone(),
        content,
    })
}

/// Summarize multiple pages in parallel chunks
const CONCURRENT_SUMMARIZATIONS: usize = 5;

pub async fn summarize_pages(
    pages: Vec<PageToSummarize>,
    ai: &dyn BaseAI,
    pool: &PgPool,
) -> Result<Vec<SummarizedPage>> {
    let mut results = Vec::with_capacity(pages.len());

    // Process in chunks of N concurrent requests
    for chunk in pages.chunks(CONCURRENT_SUMMARIZATIONS) {
        let futures: Vec<_> = chunk
            .iter()
            .map(|page| summarize_page(page, ai, pool))
            .collect();

        let chunk_results = join_all(futures).await;

        for result in chunk_results {
            match result {
                Ok(summary) => results.push(summary),
                Err(e) => {
                    tracing::warn!(error = %e, "Failed to summarize page, skipping");
                }
            }
        }
    }

    info!(
        total = results.len(),
        chunks = (pages.len() + CONCURRENT_SUMMARIZATIONS - 1) / CONCURRENT_SUMMARIZATIONS,
        "Page summarization complete"
    );

    Ok(results)
}

/// Generate content hash for cache key
pub fn hash_content(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    hex::encode(hasher.finalize())
}

/// AI call to extract meaningful content from a page
///
/// This extracts ALL meaningful information from a webpage, identifying each
/// distinct program or service as a potential post. The goal is thorough extraction,
/// not compression - we want the full details that would help someone find or use services.
async fn generate_page_content(ai: &dyn BaseAI, url: &str, raw_content: &str) -> Result<String> {
    let prompt = format!(
        r#"Extract ALL meaningful information from this webpage.

For each distinct program, service, or offering you find, create a clearly labeled section with:
- Program/service name
- What it offers and who it helps
- Contact information (phone, email, address, hours of operation)
- Eligibility requirements or how to access
- Any time-sensitive information (events, deadlines, registration periods)
- Geographic service area or coverage

Also extract any general organization information:
- Organization name and mission
- Overall contact information
- Physical location and service hours
- Languages served

BE THOROUGH. Include all details that would help someone find or use these services.
Do NOT compress or summarize - we want the full details extracted.
Each distinct program/service should be its own section.

IGNORE:
- Navigation menus, headers, footers
- Ads and promotional banners
- Social media links and share buttons
- Cookie notices and legal boilerplate
- Duplicate content

Write in plain text, no markdown formatting.
Use clear section headers like "PROGRAM:" or "SERVICE:" to separate distinct offerings.

URL: {}

Page Content:
{}"#,
        url,
        truncate_content(raw_content, 30000)
    );

    ai.complete(&prompt).await
}

/// Truncate content to fit in context window
///
/// Default max is 30K chars to allow thorough extraction while staying within
/// model context limits. Can be overridden for specific use cases.
fn truncate_content(content: &str, max_chars: usize) -> &str {
    if content.len() <= max_chars {
        content
    } else {
        // Try to break at a word boundary
        match content[..max_chars].rfind(char::is_whitespace) {
            Some(pos) => &content[..pos],
            None => &content[..max_chars],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_content() {
        let hash1 = hash_content("hello world");
        let hash2 = hash_content("hello world");
        let hash3 = hash_content("different content");

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
        assert_eq!(hash1.len(), 64); // SHA256 hex = 64 chars
    }

    #[test]
    fn test_truncate_content() {
        let short = "hello";
        assert_eq!(truncate_content(short, 100), "hello");

        let long = "hello world this is a test";
        assert_eq!(truncate_content(long, 15), "hello world");
    }
}
