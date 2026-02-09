//! Website domain actions - business logic functions
//!
//! Actions return events directly. GraphQL mutations call actions via `process()`
//! and the returned event is dispatched through the engine.

pub mod approval;
pub mod discover;

use anyhow::Result;
use tracing::info;

use crate::common::{build_page_info, Cursor, MemberId, ValidatedPaginationArgs, WebsiteId};
use crate::domains::website::data::{WebsiteConnection, WebsiteData, WebsiteEdge};
use crate::domains::website::models::Website;
use crate::kernel::ServerDeps;

/// Get all websites pending review
/// Note: Admin auth is checked at the GraphQL layer
pub async fn get_pending_websites(deps: &ServerDeps) -> Result<Vec<Website>> {
    info!("Getting pending websites");

    Website::find_pending_review(&deps.db_pool).await
}

/// Approve a website for crawling
/// Returns the approved WebsiteId.
pub async fn approve_website(
    website_id: WebsiteId,
    requested_by: MemberId,
    deps: &ServerDeps,
) -> Result<WebsiteId> {
    info!(website_id = %website_id, requested_by = %requested_by, "Approving website");

    Website::approve(website_id, requested_by, &deps.db_pool).await?;

    Ok(website_id)
}

/// Reject a website submission
/// Returns the rejected WebsiteId.
pub async fn reject_website(
    website_id: WebsiteId,
    reason: String,
    requested_by: MemberId,
    deps: &ServerDeps,
) -> Result<WebsiteId> {
    info!(website_id = %website_id, reason = %reason, requested_by = %requested_by, "Rejecting website");

    Website::reject(website_id, requested_by, reason, &deps.db_pool).await?;

    Ok(website_id)
}

/// Suspend an approved website
/// Returns the suspended WebsiteId.
pub async fn suspend_website(
    website_id: WebsiteId,
    reason: String,
    requested_by: MemberId,
    deps: &ServerDeps,
) -> Result<WebsiteId> {
    info!(website_id = %website_id, reason = %reason, requested_by = %requested_by, "Suspending website");

    Website::suspend(website_id, requested_by, reason, &deps.db_pool).await?;

    Ok(website_id)
}

/// Update website crawl settings
/// Returns the updated WebsiteId.
pub async fn update_crawl_settings(
    website_id: WebsiteId,
    max_pages_per_crawl: i32,
    deps: &ServerDeps,
) -> Result<WebsiteId> {
    info!(
        website_id = %website_id,
        max_pages_per_crawl = max_pages_per_crawl,
        "Updating website crawl settings"
    );

    Website::update_max_pages_per_crawl(website_id, max_pages_per_crawl, &deps.db_pool).await?;

    Ok(website_id)
}

// ============================================================================
// Semantic Search
// ============================================================================

/// Search websites semantically using natural language queries.
///
/// Generates an embedding for the query then searches assessments by cosine similarity.
pub async fn search_websites_semantic(
    query: &str,
    threshold: f32,
    limit: i32,
    deps: &ServerDeps,
) -> Result<Vec<crate::domains::website::models::WebsiteSearchResult>> {
    let query_embedding = deps.ai.create_embedding(query, "text-embedding-3-small").await?;

    crate::domains::website::models::WebsiteAssessment::search_by_similarity(
        &query_embedding,
        threshold,
        limit,
        &deps.db_pool,
    )
    .await
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
