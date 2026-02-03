//! Scraper cascade handlers
//!
//! These handlers respond to fact events and are called from the composite effect.
//! Entry-point actions live in `actions/`, not here.
//!
//! # Deprecation Notice
//!
//! This module uses the deprecated `PageSnapshot` model. New code should use
//! the extraction library's `ExtractionService` for scraping and extraction.

#![allow(deprecated)]

use anyhow::Result;
use seesaw_core::EffectContext;

use crate::common::AppState;
use crate::common::JobId;
use crate::domains::crawling::models::PageSnapshot;
use crate::domains::posts::events::PostEvent;
use crate::kernel::ServerDeps;

/// Cascade handler: WebsiteCreatedFromLink → scrape resource link
pub async fn handle_scrape_resource_link(
    job_id: JobId,
    url: String,
    context: Option<String>,
    submitter_contact: Option<String>,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<()> {
    tracing::info!(
        job_id = %job_id,
        url = %url,
        context = ?context,
        "Starting resource link scrape (cascade)"
    );

    let raw_page = match ctx.deps().ingestor.fetch_one(&url).await {
        Ok(r) => {
            tracing::info!(
                job_id = %job_id,
                content_length = r.content.len(),
                "Resource link scrape completed"
            );
            r
        }
        Err(e) => {
            tracing::error!(job_id = %job_id, url = %url, error = %e, "Scraping failed");
            ctx.emit(PostEvent::ResourceLinkScrapeFailed {
                job_id,
                reason: format!("Web scraping failed: {}", e),
            });
            return Ok(());
        }
    };

    let (page_snapshot, is_new) = match PageSnapshot::upsert(
        &ctx.deps().db_pool,
        url.clone(),
        raw_page.content.clone(),
        Some(raw_page.content.clone()),
        "ingestor".to_string(),
    )
    .await
    {
        Ok(snapshot) => snapshot,
        Err(e) => {
            tracing::warn!(job_id = %job_id, error = %e, "Failed to store page snapshot");
            (
                PageSnapshot {
                    id: uuid::Uuid::new_v4(),
                    url: url.clone(),
                    content_hash: vec![],
                    html: raw_page.content.clone(),
                    markdown: Some(raw_page.content.clone()),
                    fetched_via: "ingestor".to_string(),
                    metadata: serde_json::json!({}),
                    crawled_at: chrono::Utc::now(),
                    listings_extracted_count: Some(0),
                    extraction_completed_at: None,
                    extraction_status: Some("pending".to_string()),
                },
                true,
            )
        }
    };

    if is_new {
        tracing::info!(job_id = %job_id, page_snapshot_id = %page_snapshot.id, "Created page snapshot");
    }

    ctx.emit(PostEvent::ResourceLinkScraped {
        job_id,
        url,
        content: raw_page.content,
        context,
        submitter_contact,
        page_snapshot_id: Some(page_snapshot.id),
    });
    Ok(())
}
