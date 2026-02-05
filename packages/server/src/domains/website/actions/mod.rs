//! Website domain actions - business logic functions
//!
//! Actions return events directly. GraphQL mutations call actions via `process()`
//! and the returned event is dispatched through the engine.

use anyhow::Result;
use seesaw_core::EffectContext;
use tracing::info;

use crate::common::{
    build_page_info, AppState, Cursor, MemberId, ValidatedPaginationArgs, WebsiteId,
};
use crate::domains::website::data::{WebsiteConnection, WebsiteData, WebsiteEdge};
use crate::domains::website::events::WebsiteEvent;
use crate::domains::website::models::Website;
use crate::kernel::ServerDeps;

/// Get all websites pending review
/// Note: Admin auth is checked at the GraphQL layer
pub async fn get_pending_websites(
    deps: &ServerDeps,
) -> Result<Vec<Website>> {
    info!("Getting pending websites");

    Website::find_pending_review(&deps.db_pool).await
}

/// Approve a website for crawling
/// Returns WebsiteApproved event.
pub async fn approve_website(
    website_id: WebsiteId,
    requested_by: MemberId,
    deps: &ServerDeps,
) -> Result<WebsiteEvent> {
    info!(website_id = %website_id, requested_by = %requested_by, "Approving website");

    Website::approve(website_id, requested_by, &deps.db_pool).await?;

    Ok(WebsiteEvent::WebsiteApproved {
        website_id,
        reviewed_by: requested_by,
    })
}

/// Reject a website submission
/// Returns WebsiteRejected event.
pub async fn reject_website(
    website_id: WebsiteId,
    reason: String,
    requested_by: MemberId,
    deps: &ServerDeps,
) -> Result<WebsiteEvent> {
    info!(website_id = %website_id, reason = %reason, requested_by = %requested_by, "Rejecting website");

    Website::reject(website_id, requested_by, reason.clone(), &deps.db_pool).await?;

    Ok(WebsiteEvent::WebsiteRejected {
        website_id,
        reason,
        reviewed_by: requested_by,
    })
}

/// Suspend an approved website
/// Returns WebsiteSuspended event.
pub async fn suspend_website(
    website_id: WebsiteId,
    reason: String,
    requested_by: MemberId,
    deps: &ServerDeps,
) -> Result<WebsiteEvent> {
    info!(website_id = %website_id, reason = %reason, requested_by = %requested_by, "Suspending website");

    Website::suspend(website_id, requested_by, reason.clone(), &deps.db_pool).await?;

    Ok(WebsiteEvent::WebsiteSuspended {
        website_id,
        reason,
        reviewed_by: requested_by,
    })
}

/// Update website crawl settings
/// Returns CrawlSettingsUpdated event.
pub async fn update_crawl_settings(
    website_id: WebsiteId,
    max_pages_per_crawl: i32,
    deps: &ServerDeps,
) -> Result<WebsiteEvent> {
    info!(
        website_id = %website_id,
        max_pages_per_crawl = max_pages_per_crawl,
        "Updating website crawl settings"
    );

    Website::update_max_pages_per_crawl(website_id, max_pages_per_crawl, &deps.db_pool).await?;

    Ok(WebsiteEvent::CrawlSettingsUpdated {
        website_id,
        max_pages_per_crawl,
    })
}

// ============================================================================
// Query Actions (Relay pagination)
// ============================================================================

/// Get paginated websites with cursor-based pagination (Relay spec)
///
/// Admin only. Returns a WebsiteConnection with edges, pageInfo, and totalCount.
/// Note: Admin auth is checked at the GraphQL layer
pub async fn get_websites_paginated(
    status: Option<&str>,
    args: &ValidatedPaginationArgs,
    deps: &ServerDeps,
) -> Result<WebsiteConnection> {
    let pool = &deps.db_pool;

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
