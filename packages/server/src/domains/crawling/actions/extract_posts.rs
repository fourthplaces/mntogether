//! Extract posts from pages action
//!
//! Two-pass extraction: summarize pages then synthesize posts.

use std::collections::HashSet;

use anyhow::Result;
use tracing::{info, warn};

use crate::common::{ContactInfo, ExtractedPost, JobId};
use crate::domains::crawling::effects::extraction::{
    summarize_pages, synthesize_posts, PageToSummarize, SynthesisInput,
};
use crate::domains::crawling::events::{CrawlEvent, PageExtractionResult};
use crate::domains::crawling::models::PageSnapshot;
use crate::domains::website::models::Website;
use crate::kernel::{BaseAI, ServerDeps};

/// Result of extraction containing posts and page results.
pub struct ExtractionResult {
    pub posts: Vec<ExtractedPost>,
    pub page_results: Vec<PageExtractionResult>,
    pub pages_with_posts: HashSet<String>,
}

/// Run two-pass extraction (summarize + synthesize) on pages.
///
/// Returns extracted posts and page results, or a failure/no-listings event.
pub async fn extract_posts_from_pages(
    website: &Website,
    pages_to_summarize: Vec<PageToSummarize>,
    job_id: JobId,
    ai: &dyn BaseAI,
    deps: &ServerDeps,
) -> Result<ExtractionResult, CrawlEvent> {
    let website_id = website.id;

    // Pass 1: Summarize each page (with caching)
    info!(
        website_id = %website_id,
        pages = pages_to_summarize.len(),
        "Pass 1: Summarizing pages"
    );

    let summaries = match summarize_pages(pages_to_summarize.clone(), ai, &deps.db_pool).await {
        Ok(s) => s,
        Err(e) => {
            warn!(website_id = %website_id, error = %e, "Pass 1 failed");
            return Err(CrawlEvent::WebsiteCrawlFailed {
                website_id,
                job_id,
                reason: format!("Summarization failed: {}", e),
            });
        }
    };

    if summaries.is_empty() {
        return Err(CrawlEvent::WebsiteCrawlNoListings {
            website_id,
            job_id,
            attempt_number: 1,
            pages_crawled: pages_to_summarize.len(),
            should_retry: false,
        });
    }

    // Pass 2: Synthesize posts from all summaries
    info!(
        website_id = %website_id,
        summaries = summaries.len(),
        "Pass 2: Synthesizing posts"
    );

    let extracted_posts = match synthesize_posts(
        SynthesisInput {
            website_domain: website.domain.clone(),
            pages: summaries,
        },
        ai,
    )
    .await
    {
        Ok(l) => l,
        Err(e) => {
            warn!(website_id = %website_id, error = %e, "Pass 2 failed");
            return Err(CrawlEvent::WebsiteCrawlFailed {
                website_id,
                job_id,
                reason: format!("Synthesis failed: {}", e),
            });
        }
    };

    // Convert to common format and build page results
    let (posts, page_results, pages_with_posts) =
        convert_extraction_results(&extracted_posts, &pages_to_summarize);

    Ok(ExtractionResult {
        posts,
        page_results,
        pages_with_posts,
    })
}

/// Convert extracted posts to common format and build page results.
fn convert_extraction_results(
    extracted_posts: &[crate::domains::crawling::effects::extraction::ExtractedPost],
    pages: &[PageToSummarize],
) -> (Vec<ExtractedPost>, Vec<PageExtractionResult>, HashSet<String>) {
    let mut all_posts: Vec<ExtractedPost> = Vec::new();
    let mut pages_with_posts: HashSet<String> = HashSet::new();

    for extracted in extracted_posts {
        for url in &extracted.source_urls {
            pages_with_posts.insert(url.clone());
        }

        all_posts.push(ExtractedPost {
            title: extracted.title.clone(),
            tldr: extracted.tldr.clone(),
            description: extracted.description.clone(),
            contact: extracted.contact.as_ref().map(|c| ContactInfo {
                phone: c.phone.clone(),
                email: c.email.clone(),
                website: c.website.clone(),
            }),
            location: extracted.location.clone(),
            urgency: Some("normal".to_string()),
            confidence: Some("high".to_string()),
            audience_roles: extracted
                .tags
                .iter()
                .filter(|t| t.kind == "audience_role")
                .map(|t| t.value.clone())
                .collect(),
        });
    }

    // Build page results
    let mut page_results: Vec<PageExtractionResult> = Vec::new();
    for page in pages {
        let has_posts = pages_with_posts.contains(&page.url);
        page_results.push(PageExtractionResult {
            url: page.url.clone(),
            snapshot_id: Some(page.snapshot_id),
            listings_count: if has_posts { 1 } else { 0 },
            has_posts,
        });
    }

    (all_posts, page_results, pages_with_posts)
}

/// Update page snapshot extraction status after processing.
pub async fn update_page_extraction_status(
    page_results: &[PageExtractionResult],
    pool: &sqlx::PgPool,
) {
    for page in page_results {
        if let Some(sid) = page.snapshot_id {
            let _ = PageSnapshot::update_extraction_status(
                pool,
                sid,
                page.listings_count as i32,
                "completed",
            )
            .await;
        }
    }
}
