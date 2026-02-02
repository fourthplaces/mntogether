//! Scraping actions - entry-point functions for scraping operations
//!
//! These are called directly from GraphQL mutations via `process()`.
//! Actions are self-contained: they take raw input, handle ID parsing,
//! auth checks, and return results directly.

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
/// Returns scrape job result directly.
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

    if let Err(auth_err) = Actor::new(requested_by, is_admin)
        .can(AdminCapability::TriggerScraping)
        .check(ctx.deps())
        .await
    {
        tracing::warn!(
            source_id = %source_id,
            requested_by = %requested_by,
            error = %auth_err,
            "Authorization denied"
        );
        ctx.emit(PostEvent::AuthorizationDenied {
            user_id: requested_by,
            action: "ScrapeSource".to_string(),
            reason: auth_err.to_string(),
        });
        anyhow::bail!("Authorization denied: {}", auth_err);
    }

    let source = match Website::find_by_id(source_id, &ctx.deps().db_pool).await {
        Ok(s) => {
            info!(source_id = %source_id, domain = %s.domain, "Source found");
            s
        }
        Err(e) => {
            tracing::error!(source_id = %source_id, error = %e, "Failed to find source");
            ctx.emit(PostEvent::ScrapeFailed {
                source_id,
                job_id,
                reason: format!("Failed to find source: {}", e),
            });
            return Ok(ScrapeJobResult {
                job_id: job_id.into_uuid(),
                source_id: source_id.into_uuid(),
                status: "failed".to_string(),
                message: Some(format!("Source not found: {}", e)),
            });
        }
    };

    let scrape_result = match ctx.deps().web_scraper.scrape(&source.domain).await {
        Ok(r) => {
            info!(source_id = %source_id, content_length = r.markdown.len(), "Scrape completed");
            r
        }
        Err(e) => {
            tracing::error!(source_id = %source_id, error = %e, "Scraping failed");
            ctx.emit(PostEvent::ScrapeFailed {
                source_id,
                job_id,
                reason: format!("Scraping failed: {}", e),
            });
            return Ok(ScrapeJobResult {
                job_id: job_id.into_uuid(),
                source_id: source_id.into_uuid(),
                status: "failed".to_string(),
                message: Some(format!("Scraping failed: {}", e)),
            });
        }
    };

    let (page_snapshot, is_new) = match PageSnapshot::upsert(
        &ctx.deps().db_pool,
        source.domain.clone(),
        scrape_result.markdown.clone(),
        Some(scrape_result.markdown.clone()),
        "simple_scraper".to_string(),
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
                    html: scrape_result.markdown.clone(),
                    markdown: Some(scrape_result.markdown.clone()),
                    fetched_via: "simple_scraper".to_string(),
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

    if let Ok(website_snapshot) = WebsiteSnapshot::upsert(
        &ctx.deps().db_pool,
        source_id,
        source.domain.clone(),
        None,
    )
    .await
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
        content: scrape_result.markdown,
        page_snapshot_id: Some(page_snapshot.id),
    });

    Ok(ScrapeJobResult {
        job_id: job_id.into_uuid(),
        source_id: source_id.into_uuid(),
        status: "completed".to_string(),
        message: Some("Scraping completed".to_string()),
    })
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
