//! Website domain actions - business logic functions
//!
//! Actions are async functions called directly from GraphQL mutations via `process()`.
//! They do the work, emit fact events, and return ReadResult<T> for deferred reads.

use anyhow::Result;
use seesaw_core::EffectContext;
use tracing::info;

use crate::common::{AppState, MemberId, ReadResult, WebsiteId};
use crate::domains::website::events::WebsiteEvent;
use crate::domains::website::models::Website;
use crate::kernel::ServerDeps;

/// Approve a website for crawling
pub async fn approve_website(
    website_id: WebsiteId,
    requested_by: MemberId,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<ReadResult<Website>> {
    info!(website_id = %website_id, requested_by = %requested_by, "Approving website");

    Website::approve(website_id, requested_by, &ctx.deps().db_pool).await?;

    ctx.emit(WebsiteEvent::WebsiteApproved {
        website_id,
        reviewed_by: requested_by,
    });

    Ok(ReadResult::new(website_id, ctx.deps().db_pool.clone()))
}

/// Reject a website submission
pub async fn reject_website(
    website_id: WebsiteId,
    reason: String,
    requested_by: MemberId,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<ReadResult<Website>> {
    info!(website_id = %website_id, reason = %reason, requested_by = %requested_by, "Rejecting website");

    Website::reject(website_id, requested_by, reason.clone(), &ctx.deps().db_pool).await?;

    ctx.emit(WebsiteEvent::WebsiteRejected {
        website_id,
        reason,
        reviewed_by: requested_by,
    });

    Ok(ReadResult::new(website_id, ctx.deps().db_pool.clone()))
}

/// Suspend an approved website
pub async fn suspend_website(
    website_id: WebsiteId,
    reason: String,
    requested_by: MemberId,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<ReadResult<Website>> {
    info!(website_id = %website_id, reason = %reason, requested_by = %requested_by, "Suspending website");

    Website::suspend(website_id, requested_by, reason.clone(), &ctx.deps().db_pool).await?;

    ctx.emit(WebsiteEvent::WebsiteSuspended {
        website_id,
        reason,
        reviewed_by: requested_by,
    });

    Ok(ReadResult::new(website_id, ctx.deps().db_pool.clone()))
}

/// Update website crawl settings
pub async fn update_crawl_settings(
    website_id: WebsiteId,
    max_pages_per_crawl: i32,
    requested_by: MemberId,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<ReadResult<Website>> {
    info!(
        website_id = %website_id,
        max_pages_per_crawl = max_pages_per_crawl,
        requested_by = %requested_by,
        "Updating website crawl settings"
    );

    Website::update_max_pages_per_crawl(website_id, max_pages_per_crawl, &ctx.deps().db_pool).await?;

    ctx.emit(WebsiteEvent::CrawlSettingsUpdated {
        website_id,
        max_pages_per_crawl,
    });

    Ok(ReadResult::new(website_id, ctx.deps().db_pool.clone()))
}
