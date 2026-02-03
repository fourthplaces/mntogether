//! Website domain actions - business logic functions
//!
//! Actions are async functions called directly from GraphQL mutations via `process()`.
//! They do the work, emit fact events, and return ReadResult<T> for deferred reads.

use anyhow::Result;
use seesaw_core::EffectContext;
use tracing::info;

use crate::common::{
    build_page_info, AppState, Cursor, MemberId, ReadResult, ValidatedPaginationArgs, WebsiteId,
};
use crate::domains::website::data::{WebsiteConnection, WebsiteData, WebsiteEdge};
use crate::domains::website::events::WebsiteEvent;
use crate::domains::website::models::Website;
use crate::kernel::ServerDeps;

/// Get all websites pending review (admin only)
pub async fn get_pending_websites(
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<Vec<Website>> {
    // Admin authorization check
    ctx.next_state().require_admin()?;

    info!("Getting pending websites");

    Website::find_pending_review(&ctx.deps().db_pool).await
}

/// Approve a website for crawling
pub async fn approve_website(
    website_id: WebsiteId,
    requested_by: MemberId,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<ReadResult<Website>> {
    // Admin authorization check
    ctx.next_state().require_admin()?;

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
    // Admin authorization check
    ctx.next_state().require_admin()?;

    info!(website_id = %website_id, reason = %reason, requested_by = %requested_by, "Rejecting website");

    Website::reject(
        website_id,
        requested_by,
        reason.clone(),
        &ctx.deps().db_pool,
    )
    .await?;

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
    // Admin authorization check
    ctx.next_state().require_admin()?;

    info!(website_id = %website_id, reason = %reason, requested_by = %requested_by, "Suspending website");

    Website::suspend(
        website_id,
        requested_by,
        reason.clone(),
        &ctx.deps().db_pool,
    )
    .await?;

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
    // Admin authorization check
    ctx.next_state().require_admin()?;

    info!(
        website_id = %website_id,
        max_pages_per_crawl = max_pages_per_crawl,
        requested_by = %requested_by,
        "Updating website crawl settings"
    );

    Website::update_max_pages_per_crawl(website_id, max_pages_per_crawl, &ctx.deps().db_pool)
        .await?;

    ctx.emit(WebsiteEvent::CrawlSettingsUpdated {
        website_id,
        max_pages_per_crawl,
    });

    Ok(ReadResult::new(website_id, ctx.deps().db_pool.clone()))
}

// ============================================================================
// Query Actions (Relay pagination)
// ============================================================================

/// Get paginated websites with cursor-based pagination (Relay spec)
///
/// Admin only. Returns a WebsiteConnection with edges, pageInfo, and totalCount.
pub async fn get_websites_paginated(
    status: Option<&str>,
    args: &ValidatedPaginationArgs,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<WebsiteConnection> {
    // Admin authorization check
    ctx.next_state().require_admin()?;

    let pool = &ctx.deps().db_pool;

    // Fetch websites with cursor pagination
    let (websites, has_more) = Website::find_paginated(status, args, pool).await?;

    // Get total count for the filter
    let total_count = Website::count_with_filters(status, pool).await? as i32;

    // Build edges with cursors
    let edges: Vec<WebsiteEdge> = websites
        .into_iter()
        .map(|website| {
            let cursor = Cursor::encode_uuid(website.id.into_uuid());
            WebsiteEdge {
                node: WebsiteData::from(website),
                cursor,
            }
        })
        .collect();

    // Build page info
    let page_info = build_page_info(
        has_more,
        args,
        edges.first().map(|e| e.cursor.clone()),
        edges.last().map(|e| e.cursor.clone()),
    );

    Ok(WebsiteConnection {
        edges,
        page_info,
        total_count,
    })
}
