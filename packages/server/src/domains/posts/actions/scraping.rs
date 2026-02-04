//! Scraping actions - entry-point functions for scraping operations
//!
//! These are called directly from GraphQL mutations via `process()`.
//! Actions are self-contained: they take raw input, handle ID parsing,
//! auth checks, and return results directly.
//!
//! # Deprecation Notice
//!
//! This module uses deprecated `PageSnapshot` and `WebsiteSnapshot` models.
//! New code should use `ExtractionService::ingest()` for scraping which stores
//! pages in `extraction_pages`.

#![allow(deprecated)]

use anyhow::{Context, Result};
use seesaw_core::EffectContext;
use tracing::info;
use uuid::Uuid;

use crate::common::auth::{Actor, AdminCapability};
use crate::common::{AppState, JobId, MemberId, WebsiteId};
use crate::domains::crawling::models::{PageSnapshot, WebsiteSnapshot};
use crate::domains::posts::effects::post::extract_domain;
use crate::domains::posts::events::PostEvent;
use crate::domains::website::models::Website;
use crate::kernel::ServerDeps;

/// Result of starting a scrape job
#[derive(Debug, Clone)]
pub struct ScrapeJobResult {
    pub job_id: Uuid,
    pub source_id: Uuid,
    pub status: String,
    pub message: Option<String>,
}

/// Result of submitting a resource link
#[derive(Debug, Clone)]
pub struct SubmitResourceLinkResult {
    pub job_id: Uuid,
    pub status: String,
    pub message: String,
}

/// Scrape an organization source (admin only)
///
/// # Deprecated
/// Use `crawling::actions::ingest_website()` instead. This function uses
/// the old BaseWebScraper and writes to deprecated page_snapshots table.
#[deprecated(note = "Use crawling::actions::ingest_website() instead")]
pub async fn scrape_source(
    source_id: Uuid,
    member_id: Uuid,
    is_admin: bool,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<ScrapeJobResult> {
    let source_id = WebsiteId::from_uuid(source_id);
    let requested_by = MemberId::from_uuid(member_id);
    let job_id = JobId::new();

    info!(
        source_id = %source_id,
        job_id = %job_id,
        requested_by = %requested_by,
        "Starting scrape source action"
    );

    Actor::new(requested_by, is_admin)
        .can(AdminCapability::TriggerScraping)
        .check(ctx.deps())
        .await
        .map_err(|auth_err| {
            tracing::warn!(
                source_id = %source_id,
                requested_by = %requested_by,
                error = %auth_err,
                "Authorization denied"
            );
            anyhow::anyhow!("Authorization denied: {}", auth_err)
        })?;

    let source = Website::find_by_id(source_id, &ctx.deps().db_pool)
        .await
        .map_err(|e| {
            tracing::error!(source_id = %source_id, error = %e, "Failed to find source");
            anyhow::anyhow!("Failed to find source: {}", e)
        })?;
    info!(source_id = %source_id, domain = %source.domain, "Source found");

    let raw_page = ctx
        .deps()
        .ingestor
        .fetch_one(&source.domain)
        .await
        .map_err(|e| {
            tracing::error!(source_id = %source_id, error = %e, "Scraping failed");
            anyhow::anyhow!("Scraping failed: {}", e)
        })?;
    info!(source_id = %source_id, content_length = raw_page.content.len(), "Scrape completed");

    let (page_snapshot, is_new) = match PageSnapshot::upsert(
        &ctx.deps().db_pool,
        source.domain.clone(),
        raw_page.content.clone(),
        Some(raw_page.content.clone()),
        "ingestor".to_string(),
    )
    .await
    {
        Ok(snapshot) => snapshot,
        Err(e) => {
            tracing::error!(source_id = %source_id, error = %e, "Failed to store page snapshot");
            (
                PageSnapshot {
                    id: uuid::Uuid::new_v4(),
                    url: source.domain.clone(),
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
        info!(page_snapshot_id = %page_snapshot.id, "Created new page snapshot");
    }

    if let Ok(website_snapshot) =
        WebsiteSnapshot::upsert(&ctx.deps().db_pool, source_id, source.domain.clone(), None).await
    {
        let _ = website_snapshot
            .link_snapshot(&ctx.deps().db_pool, page_snapshot.id)
            .await;
    }

    let _ = Website::update_last_scraped(source_id, &ctx.deps().db_pool).await;

    ctx.emit(PostEvent::SourceScraped {
        source_id,
        job_id,
        organization_name: extract_domain(&source.domain).unwrap_or_else(|| source.domain.clone()),
        content: raw_page.content,
        page_snapshot_id: Some(page_snapshot.id),
    });

    Ok(ScrapeJobResult {
        job_id: job_id.into_uuid(),
        source_id: source_id.into_uuid(),
        status: "completed".to_string(),
        message: Some("Scraping completed".to_string()),
    })
}

/// Result of refreshing a page snapshot
#[derive(Debug, Clone)]
pub struct RefreshPageSnapshotResult {
    pub job_id: Uuid,
    pub page_snapshot_id: Uuid,
    pub status: String,
    pub message: Option<String>,
}

/// Refresh a specific page by re-ingesting its URL through the extraction pipeline (admin only).
///
/// This uses the extraction library to:
/// 1. Fetch the URL content
/// 2. Summarize and embed
/// 3. Store in extraction_pages
///
/// The old page_snapshots table is also updated for backward compatibility.
pub async fn refresh_page_snapshot(
    page_snapshot_id: Uuid,
    member_id: Uuid,
    is_admin: bool,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<RefreshPageSnapshotResult> {
    use crate::kernel::{FirecrawlIngestor, HttpIngestor, ValidatedIngestor};

    let requested_by = MemberId::from_uuid(member_id);
    let job_id = JobId::new();

    info!(
        page_snapshot_id = %page_snapshot_id,
        job_id = %job_id,
        requested_by = %requested_by,
        "Starting refresh page snapshot action"
    );

    // Auth check
    Actor::new(requested_by, is_admin)
        .can(AdminCapability::TriggerScraping)
        .check(ctx.deps())
        .await
        .map_err(|auth_err| {
            tracing::warn!(
                page_snapshot_id = %page_snapshot_id,
                requested_by = %requested_by,
                error = %auth_err,
                "Authorization denied"
            );
            anyhow::anyhow!("Authorization denied: {}", auth_err)
        })?;

    // Find the existing page snapshot to get the URL
    let page_snapshot = match PageSnapshot::find_by_id(&ctx.deps().db_pool, page_snapshot_id).await
    {
        Ok(ps) => {
            info!(
                page_snapshot_id = %page_snapshot_id,
                url = %ps.url,
                "Found page snapshot to refresh"
            );
            ps
        }
        Err(e) => {
            tracing::error!(page_snapshot_id = %page_snapshot_id, error = %e, "Page snapshot not found");
            return Ok(RefreshPageSnapshotResult {
                job_id: job_id.into_uuid(),
                page_snapshot_id,
                status: "failed".to_string(),
                message: Some(format!("Page snapshot not found: {}", e)),
            });
        }
    };

    // Use extraction library to ingest the single URL
    let urls = vec![page_snapshot.url.clone()];
    let result = match FirecrawlIngestor::from_env() {
        Ok(firecrawl) => {
            let ingestor = ValidatedIngestor::new(firecrawl);
            ctx.deps().extraction.ingest_urls(&urls, &ingestor).await
        }
        Err(_) => {
            let http = HttpIngestor::new();
            let ingestor = ValidatedIngestor::new(http);
            ctx.deps().extraction.ingest_urls(&urls, &ingestor).await
        }
    };

    match result {
        Ok(ingest_result) => {
            info!(
                page_snapshot_id = %page_snapshot_id,
                url = %page_snapshot.url,
                pages_summarized = ingest_result.pages_summarized,
                "Page refreshed via extraction library"
            );

            // Emit event to trigger post regeneration
            ctx.emit(PostEvent::PageSnapshotRefreshed {
                page_snapshot_id,
                job_id,
                url: page_snapshot.url.clone(),
                content: String::new(), // Content is now in extraction_pages
            });

            Ok(RefreshPageSnapshotResult {
                job_id: job_id.into_uuid(),
                page_snapshot_id,
                status: "completed".to_string(),
                message: Some(format!("Page refreshed: {}", page_snapshot.url)),
            })
        }
        Err(e) => {
            tracing::error!(
                page_snapshot_id = %page_snapshot_id,
                url = %page_snapshot.url,
                error = %e,
                "Page refresh failed"
            );
            Ok(RefreshPageSnapshotResult {
                job_id: job_id.into_uuid(),
                page_snapshot_id,
                status: "failed".to_string(),
                message: Some(format!("Refresh failed: {}", e)),
            })
        }
    }
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
