//! Pass 1: Summarize individual pages with caching (DEPRECATED)
//!
//! Each page is summarized to extract structured content as JSON.
//! Results are cached by content hash - if page content hasn't changed,
//! we reuse the cached summary.
//!
//! # Deprecation Notice
//!
//! This module is deprecated in favor of the extraction library's summarization.
//!
//! **Migration path:**
//! - For **textual summaries** (search/retrieval): Use `extraction::AI::summarize()`
//! - For **structured extraction** (contact, hours, programs): Use `extraction::Index::extract()`
//!
//! The extraction library stores summaries in `extraction_summaries` table with:
//! - Content hash-based caching
//! - Prompt version tracking (auto-invalidates stale summaries)
//! - Embedding generation for semantic search
//!
//! **Current vs New:**
//! | This Module | Extraction Library |
//! |-------------|-------------------|
//! | `summarize_pages()` | `extraction::pipeline::ingest::ingest_with_ingestor()` |
//! | `hash_content()` | `extraction::CachedPage::hash_content()` |
//! | `PageSummary` (server) | `extraction::Summary` (library) |
//!
//! This code remains for backward compatibility with existing pipelines.

#![allow(deprecated)]

use anyhow::Result;
use futures::future::join_all;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use tracing::info;

use crate::domains::crawling::models::PageSummary;
use crate::kernel::{BaseAI, LlmRequestExt};

use super::types::{PageSummaryContent, PageToSummarize, SummarizedPage};

/// Summarize a page, using cache if available
///
/// # Deprecated
/// Use extraction library's `ingest_with_ingestor()` or `AI::summarize()` instead.
#[deprecated(
    since = "0.1.0",
    note = "Use extraction library's ingest pipeline for summarization"
)]
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
///
/// # Deprecated
/// Use extraction library's `ingest_with_ingestor()` which handles
/// concurrent summarization with configurable concurrency.
const CONCURRENT_SUMMARIZATIONS: usize = 5;

#[deprecated(
    since = "0.1.0",
    note = "Use extraction library's ingest_with_ingestor() instead"
)]
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
///
/// # Deprecated
/// Use `extraction::CachedPage::hash_content()` instead.
#[deprecated(since = "0.1.0", note = "Use extraction::CachedPage::hash_content() instead")]
pub fn hash_content(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    hex::encode(hasher.finalize())
}

/// AI call to extract structured content from a page
///
/// This extracts ALL meaningful information from a webpage as structured JSON.
/// The goal is thorough extraction with explicit fields for contact, hours, location,
/// so synthesis can reliably combine information across pages.
async fn generate_page_content(ai: &dyn BaseAI, url: &str, raw_content: &str) -> Result<String> {
    let system = PAGE_SUMMARY_SYSTEM_PROMPT;
    let user = format!(
        "URL: {}\n\nPage Content:\n{}",
        url,
        truncate_content(raw_content, 30000)
    );

    let content: PageSummaryContent = ai
        .request()
        .system(system)
        .user(user)
        .schema_hint(PAGE_SUMMARY_SCHEMA)
        .max_retries(2)
        .output()
        .await?;

    // Serialize to JSON for storage
    Ok(serde_json::to_string(&content)?)
}

const PAGE_SUMMARY_SYSTEM_PROMPT: &str = r#"Extract ALL meaningful information from this webpage.

Your goal is THOROUGH extraction. Every piece of contact info, every program detail, every hour of operation matters. This data will be combined with other pages to create complete service listings.

EXTRACT:

1. ORGANIZATION INFO - If this page mentions the organization:
   - Name of the organization
   - Mission statement or description (in plain, readable English)
   - Languages served

2. PROGRAMS/SERVICES - Each distinct program or service should be a separate entry:
   - Program name (required)
   - **description**: A READABLE, user-friendly explanation of what this program offers. Write it as if explaining to someone who needs help. Example: "Provides free groceries to families in the St. Cloud area. You can visit once per month and receive enough food for 5-7 days worth of meals."
   - **serves**: Who this program helps (e.g., "Families with children under 18", "Anyone in need")
   - **how_to_access**: Step-by-step how someone would get this service (e.g., "Walk in during open hours - no appointment needed", "Call ahead to schedule")
   - **eligibility**: What's required or what to bring (e.g., "Bring photo ID and proof of address", "No requirements - open to all")
   - Program-specific contact/hours/location if different from org-wide

3. CONTACT INFO - Extract ALL contact methods found:
   - Phone numbers (include what they're for, e.g., "main line", "crisis hotline")
   - Email addresses
   - Website URLs
   - Other methods (fax, TTY, text lines)

4. LOCATION - Physical presence:
   - Street address
   - City, state, zip
   - Service area (what geographic area they cover)

5. HOURS - When they operate:
   - General hours description
   - Day-by-day hours if listed
   - Notes about closures, holidays, seasonal changes

6. EVENTS - Time-sensitive items:
   - Event/class names
   - Dates and times
   - Registration information

IMPORTANT:
- Program descriptions should be READABLE and USER-FRIENDLY, not just keywords
- Write descriptions as if explaining to someone seeking help
- Extract EVERYTHING - don't compress details
- If contact info appears anywhere on the page, capture it
- If hours are mentioned anywhere, capture them
- Each distinct program should be its own entry in the programs array
- Use additional_context for important info that doesn't fit the structured fields

IGNORE:
- Navigation menus, headers, footers
- Social media links, share buttons
- Cookie notices, legal boilerplate
- Advertising content"#;

const PAGE_SUMMARY_SCHEMA: &str = r#"Expected structure:
{
  "organization": {
    "name": "string or null",
    "mission": "string or null - readable mission statement",
    "description": "string or null - plain English description of what they do",
    "languages_served": ["string"]
  },
  "programs": [{
    "name": "string - program/service name (required)",
    "description": "string or null - READABLE explanation of what this offers, written for someone seeking help. Example: 'Provides free groceries to families. Visit once per month for 5-7 days of food.'",
    "serves": "string or null - who it helps (e.g., 'Families in Washington County')",
    "how_to_access": "string or null - exactly how to get help (e.g., 'Walk in Mon-Fri 9am-3pm, no appointment needed')",
    "eligibility": "string or null - requirements or what to bring (e.g., 'Bring photo ID and utility bill')",
    "contact": { "phone": "string or null", "email": "string or null", "website": "string or null", "other": ["string"] },
    "hours": "string or null - program-specific hours",
    "location": "string or null - program-specific location"
  }],
  "contact": {
    "phone": "string or null - main phone",
    "email": "string or null - main email",
    "website": "string or null",
    "other": ["string - other contact methods like fax, TTY"]
  },
  "location": {
    "address": "string or null - street address",
    "city": "string or null",
    "state": "string or null",
    "zip": "string or null",
    "service_area": "string or null - geographic coverage (e.g., 'Washington County residents')"
  },
  "hours": {
    "general": "string or null - general hours description",
    "by_day": [{ "day": "string", "hours": "string" }],
    "notes": "string or null - holiday/seasonal notes"
  },
  "events": [{
    "name": "string",
    "date": "string or null",
    "time": "string or null",
    "description": "string or null",
    "registration_info": "string or null"
  }],
  "additional_context": "string or null - important info that doesn't fit above"
}"#;

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
