use crate::common::{JobId, WebsiteId};
use crate::domains::posts::data::ScrapeJobResult;
use crate::domains::posts::events::PostEvent;
use crate::domains::website::data::WebsiteData;
use crate::domains::website::models::{Website, WebsiteSnapshot};
use crate::server::graphql::context::GraphQLContext;
use juniper::{FieldError, FieldResult};
use seesaw_core::dispatch_request;
use tracing::info;
use uuid::Uuid;

/// Approve a website for crawling (admin only)
/// Direct database operation - no event bus needed for approval workflow
pub async fn approve_website(
    ctx: &GraphQLContext,
    website_id: String,
) -> FieldResult<WebsiteData> {
    info!(website_id = %website_id, "Approving website");

    // Get user info - must be admin
    let user = ctx
        .auth_user
        .as_ref()
        .ok_or_else(|| FieldError::new("Authentication required", juniper::Value::null()))?;

    if !user.is_admin {
        return Err(FieldError::new(
            "Admin authorization required",
            juniper::Value::null(),
        ));
    }

    // Parse website ID
    let uuid = Uuid::parse_str(&website_id)
        .map_err(|_| FieldError::new("Invalid website ID", juniper::Value::null()))?;
    let id = WebsiteId::from_uuid(uuid);

    // Approve using model method
    let website = Website::approve(id, user.member_id, &ctx.db_pool)
        .await
        .map_err(|e| {
            FieldError::new(
                format!("Failed to approve website: {}", e),
                juniper::Value::null(),
            )
        })?;

    Ok(WebsiteData::from(website))
}

/// Reject a website submission (admin only)
/// Direct database operation - no event bus needed for approval workflow
pub async fn reject_website(
    ctx: &GraphQLContext,
    website_id: String,
    reason: String,
) -> FieldResult<WebsiteData> {
    info!(website_id = %website_id, reason = %reason, "Rejecting website");

    // Get user info - must be admin
    let user = ctx
        .auth_user
        .as_ref()
        .ok_or_else(|| FieldError::new("Authentication required", juniper::Value::null()))?;

    if !user.is_admin {
        return Err(FieldError::new(
            "Admin authorization required",
            juniper::Value::null(),
        ));
    }

    // Parse website ID
    let uuid = Uuid::parse_str(&website_id)
        .map_err(|_| FieldError::new("Invalid website ID", juniper::Value::null()))?;
    let id = WebsiteId::from_uuid(uuid);

    // Reject using model method
    let website = Website::reject(id, user.member_id, reason, &ctx.db_pool)
        .await
        .map_err(|e| {
            FieldError::new(
                format!("Failed to reject website: {}", e),
                juniper::Value::null(),
            )
        })?;

    Ok(WebsiteData::from(website))
}

/// Suspend a website (admin only)
/// Direct database operation - no event bus needed for approval workflow
pub async fn suspend_website(
    ctx: &GraphQLContext,
    website_id: String,
    reason: String,
) -> FieldResult<WebsiteData> {
    info!(website_id = %website_id, reason = %reason, "Suspending website");

    // Get user info - must be admin
    let user = ctx
        .auth_user
        .as_ref()
        .ok_or_else(|| FieldError::new("Authentication required", juniper::Value::null()))?;

    if !user.is_admin {
        return Err(FieldError::new(
            "Admin authorization required",
            juniper::Value::null(),
        ));
    }

    // Parse website ID
    let uuid = Uuid::parse_str(&website_id)
        .map_err(|_| FieldError::new("Invalid website ID", juniper::Value::null()))?;
    let id = WebsiteId::from_uuid(uuid);

    // Suspend using model method
    let website = Website::suspend(id, user.member_id, reason, &ctx.db_pool)
        .await
        .map_err(|e| {
            FieldError::new(
                format!("Failed to suspend website: {}", e),
                juniper::Value::null(),
            )
        })?;

    Ok(WebsiteData::from(website))
}

/// Crawl a website (multi-page) to discover and extract listings
/// This performs a full crawl of the website, extracting listings from all pages found
pub async fn crawl_website(ctx: &GraphQLContext, website_id: Uuid) -> FieldResult<ScrapeJobResult> {
    info!(website_id = %website_id, "Crawling website (multi-page)");

    // Get user info (authorization will be checked in effect)
    let user = ctx
        .auth_user
        .as_ref()
        .ok_or_else(|| FieldError::new("Authentication required", juniper::Value::null()))?;

    // Convert to typed IDs
    let website_id = WebsiteId::from_uuid(website_id);
    let job_id = JobId::new();

    // Dispatch crawl request and await completion
    let result = dispatch_request(
        PostEvent::CrawlWebsiteRequested {
            website_id,
            job_id,
            requested_by: user.member_id,
            is_admin: user.is_admin,
        },
        &ctx.bus,
        |m| {
            m.try_match(|e: &PostEvent| match e {
                // Success - crawl workflow complete (listings synced)
                PostEvent::PostsSynced {
                    source_id: synced_source_id,
                    job_id: synced_job_id,
                    new_count,
                    updated_count,
                    unchanged_count,
                } if *synced_source_id == website_id && *synced_job_id == job_id => Some(Ok((
                    "completed".to_string(),
                    format!(
                        "Crawl complete! Found {} new, {} updated, {} unchanged listings",
                        new_count, updated_count, unchanged_count
                    ),
                ))),
                // No listings found but may retry
                PostEvent::WebsiteMarkedNoListings {
                    website_id: marked_id,
                    job_id: marked_job_id,
                    total_attempts,
                } if *marked_id == website_id && *marked_job_id == job_id => Some(Ok((
                    "no_posts".to_string(),
                    format!(
                        "No listings found after {} attempts. Website marked as no_posts_found.",
                        total_attempts
                    ),
                ))),
                // Failure events
                PostEvent::WebsiteCrawlFailed {
                    website_id: failed_id,
                    job_id: failed_job_id,
                    reason,
                } if *failed_id == website_id && *failed_job_id == job_id => {
                    Some(Err(anyhow::anyhow!("Crawl failed: {}", reason)))
                }
                PostEvent::ExtractFailed {
                    source_id: failed_source_id,
                    job_id: failed_job_id,
                    reason,
                } if *failed_source_id == website_id && *failed_job_id == job_id => {
                    Some(Err(anyhow::anyhow!("Extraction failed: {}", reason)))
                }
                PostEvent::SyncFailed {
                    source_id: failed_source_id,
                    job_id: failed_job_id,
                    reason,
                } if *failed_source_id == website_id && *failed_job_id == job_id => {
                    Some(Err(anyhow::anyhow!("Sync failed: {}", reason)))
                }
                PostEvent::AuthorizationDenied {
                    user_id,
                    action,
                    reason,
                } if *user_id == user.member_id && action == "CrawlWebsite" => {
                    Some(Err(anyhow::anyhow!("Authorization denied: {}", reason)))
                }
                _ => None,
            })
            .result()
        },
    )
    .await
    .map_err(|e| FieldError::new(format!("Crawl failed: {}", e), juniper::Value::null()))?;

    let (status, message) = result;

    Ok(ScrapeJobResult {
        job_id: job_id.into_uuid(),
        source_id: website_id.into_uuid(),
        status,
        message: Some(message),
    })
}

/// Update max pages per crawl setting (admin only)
pub async fn update_website_crawl_settings(
    ctx: &GraphQLContext,
    website_id: String,
    max_pages_per_crawl: i32,
) -> FieldResult<WebsiteData> {
    info!(website_id = %website_id, max_pages = %max_pages_per_crawl, "Updating website crawl settings");

    // Get user info - must be admin
    let user = ctx
        .auth_user
        .as_ref()
        .ok_or_else(|| FieldError::new("Authentication required", juniper::Value::null()))?;

    if !user.is_admin {
        return Err(FieldError::new(
            "Admin authorization required",
            juniper::Value::null(),
        ));
    }

    // Validate max_pages
    if max_pages_per_crawl < 1 || max_pages_per_crawl > 100 {
        return Err(FieldError::new(
            "Max pages must be between 1 and 100",
            juniper::Value::null(),
        ));
    }

    // Parse website ID
    let uuid = Uuid::parse_str(&website_id)
        .map_err(|_| FieldError::new("Invalid website ID", juniper::Value::null()))?;
    let id = WebsiteId::from_uuid(uuid);

    // Update using model method
    let website = Website::update_max_pages_per_crawl(id, max_pages_per_crawl, &ctx.db_pool)
        .await
        .map_err(|e| {
            FieldError::new(
                format!("Failed to update website settings: {}", e),
                juniper::Value::null(),
            )
        })?;

    Ok(WebsiteData::from(website))
}

/// Regenerate posts from existing page snapshots (admin only)
/// Re-runs the AI extraction and sync workflow without re-crawling
pub async fn regenerate_posts(ctx: &GraphQLContext, website_id: Uuid) -> FieldResult<ScrapeJobResult> {
    info!(website_id = %website_id, "Regenerating posts from existing snapshots");

    // Get user info (authorization will be checked in effect)
    let user = ctx
        .auth_user
        .as_ref()
        .ok_or_else(|| FieldError::new("Authentication required", juniper::Value::null()))?;

    // Convert to typed IDs
    let website_id = WebsiteId::from_uuid(website_id);
    let job_id = JobId::new();

    // Dispatch regenerate posts request and await completion
    let result = dispatch_request(
        PostEvent::RegeneratePostsRequested {
            website_id,
            job_id,
            requested_by: user.member_id,
            is_admin: user.is_admin,
        },
        &ctx.bus,
        |m| {
            m.try_match(|e: &PostEvent| match e {
                // Success - extraction and sync workflow complete
                PostEvent::PostsSynced {
                    source_id: synced_source_id,
                    job_id: synced_job_id,
                    new_count,
                    updated_count,
                    unchanged_count,
                } if *synced_source_id == website_id && *synced_job_id == job_id => Some(Ok((
                    "completed".to_string(),
                    format!(
                        "Regeneration complete! Found {} new, {} updated, {} unchanged posts",
                        new_count, updated_count, unchanged_count
                    ),
                ))),
                // No listings found
                PostEvent::WebsiteMarkedNoListings {
                    website_id: marked_id,
                    job_id: marked_job_id,
                    total_attempts,
                } if *marked_id == website_id && *marked_job_id == job_id => Some(Ok((
                    "no_posts".to_string(),
                    format!(
                        "No listings found after {} attempts.",
                        total_attempts
                    ),
                ))),
                // Failure events
                PostEvent::WebsiteCrawlFailed {
                    website_id: failed_id,
                    job_id: failed_job_id,
                    reason,
                } if *failed_id == website_id && *failed_job_id == job_id => {
                    Some(Err(anyhow::anyhow!("Regeneration failed: {}", reason)))
                }
                PostEvent::ExtractFailed {
                    source_id: failed_source_id,
                    job_id: failed_job_id,
                    reason,
                } if *failed_source_id == website_id && *failed_job_id == job_id => {
                    Some(Err(anyhow::anyhow!("Extraction failed: {}", reason)))
                }
                PostEvent::SyncFailed {
                    source_id: failed_source_id,
                    job_id: failed_job_id,
                    reason,
                } if *failed_source_id == website_id && *failed_job_id == job_id => {
                    Some(Err(anyhow::anyhow!("Sync failed: {}", reason)))
                }
                PostEvent::AuthorizationDenied {
                    user_id,
                    action,
                    reason,
                } if *user_id == user.member_id && action == "RegeneratePosts" => {
                    Some(Err(anyhow::anyhow!("Authorization denied: {}", reason)))
                }
                _ => None,
            })
            .result()
        },
    )
    .await
    .map_err(|e| FieldError::new(format!("Regeneration failed: {}", e), juniper::Value::null()))?;

    let (status, message) = result;

    Ok(ScrapeJobResult {
        job_id: job_id.into_uuid(),
        source_id: website_id.into_uuid(),
        status,
        message: Some(message),
    })
}

/// Regenerate page summaries for existing snapshots (admin only)
/// Clears cached summaries and re-runs AI summarization
pub async fn regenerate_page_summaries(
    ctx: &GraphQLContext,
    website_id: Uuid,
) -> FieldResult<ScrapeJobResult> {
    info!(website_id = %website_id, "Regenerating page summaries");

    // Get user info (authorization will be checked in effect)
    let user = ctx
        .auth_user
        .as_ref()
        .ok_or_else(|| FieldError::new("Authentication required", juniper::Value::null()))?;

    // Convert to typed IDs
    let website_id = WebsiteId::from_uuid(website_id);
    let job_id = JobId::new();

    // Dispatch regenerate page summaries request and await completion
    let result = dispatch_request(
        PostEvent::RegeneratePageSummariesRequested {
            website_id,
            job_id,
            requested_by: user.member_id,
            is_admin: user.is_admin,
        },
        &ctx.bus,
        |m| {
            m.try_match(|e: &PostEvent| match e {
                // Success - page summaries regenerated
                PostEvent::PageSummariesRegenerated {
                    website_id: regen_id,
                    job_id: regen_job_id,
                    pages_processed,
                } if *regen_id == website_id && *regen_job_id == job_id => Some(Ok((
                    "completed".to_string(),
                    format!("Successfully regenerated {} page summaries", pages_processed),
                ))),
                // Failure events
                PostEvent::WebsiteCrawlFailed {
                    website_id: failed_id,
                    job_id: failed_job_id,
                    reason,
                } if *failed_id == website_id && *failed_job_id == job_id => {
                    Some(Err(anyhow::anyhow!("Regeneration failed: {}", reason)))
                }
                PostEvent::AuthorizationDenied {
                    user_id,
                    action,
                    reason,
                } if *user_id == user.member_id && action == "RegeneratePageSummaries" => {
                    Some(Err(anyhow::anyhow!("Authorization denied: {}", reason)))
                }
                _ => None,
            })
            .result()
        },
    )
    .await
    .map_err(|e| {
        FieldError::new(
            format!("Page summary regeneration failed: {}", e),
            juniper::Value::null(),
        )
    })?;

    let (status, message) = result;

    Ok(ScrapeJobResult {
        job_id: job_id.into_uuid(),
        source_id: website_id.into_uuid(),
        status,
        message: Some(message),
    })
}

/// Regenerate AI summary for a single page snapshot (admin only)
pub async fn regenerate_page_summary(
    ctx: &GraphQLContext,
    page_snapshot_id: Uuid,
) -> FieldResult<ScrapeJobResult> {
    info!(page_snapshot_id = %page_snapshot_id, "Regenerating AI summary for single page");

    // Get user info (authorization will be checked in effect)
    let user = ctx
        .auth_user
        .as_ref()
        .ok_or_else(|| FieldError::new("Authentication required", juniper::Value::null()))?;

    let job_id = JobId::new();

    // Dispatch regenerate page summary request and await completion
    let result = dispatch_request(
        PostEvent::RegeneratePageSummaryRequested {
            page_snapshot_id,
            job_id,
            requested_by: user.member_id,
            is_admin: user.is_admin,
        },
        &ctx.bus,
        |m| {
            m.try_match(|e: &PostEvent| match e {
                // Success
                PostEvent::PageSummaryRegenerated {
                    page_snapshot_id: regen_id,
                    job_id: regen_job_id,
                } if *regen_id == page_snapshot_id && *regen_job_id == job_id => Some(Ok((
                    "completed".to_string(),
                    "AI summary regenerated successfully".to_string(),
                ))),
                // Failure events
                PostEvent::AuthorizationDenied {
                    user_id,
                    action,
                    reason,
                } if *user_id == user.member_id && action == "RegeneratePageSummary" => {
                    Some(Err(anyhow::anyhow!("Authorization denied: {}", reason)))
                }
                _ => None,
            })
            .result()
        },
    )
    .await
    .map_err(|e| {
        FieldError::new(
            format!("Failed to regenerate page summary: {}", e),
            juniper::Value::null(),
        )
    })?;

    let (status, message) = result;

    Ok(ScrapeJobResult {
        job_id: job_id.into_uuid(),
        source_id: page_snapshot_id,
        status,
        message: Some(message),
    })
}

/// Regenerate posts for a single page snapshot (admin only)
pub async fn regenerate_page_posts(
    ctx: &GraphQLContext,
    page_snapshot_id: Uuid,
) -> FieldResult<ScrapeJobResult> {
    info!(page_snapshot_id = %page_snapshot_id, "Regenerating posts for single page");

    // Get user info (authorization will be checked in effect)
    let user = ctx
        .auth_user
        .as_ref()
        .ok_or_else(|| FieldError::new("Authentication required", juniper::Value::null()))?;

    let job_id = JobId::new();

    // Dispatch regenerate page posts request and await completion
    let result = dispatch_request(
        PostEvent::RegeneratePagePostsRequested {
            page_snapshot_id,
            job_id,
            requested_by: user.member_id,
            is_admin: user.is_admin,
        },
        &ctx.bus,
        |m| {
            m.try_match(|e: &PostEvent| match e {
                // Success
                PostEvent::PagePostsRegenerated {
                    page_snapshot_id: regen_id,
                    job_id: regen_job_id,
                    posts_count,
                } if *regen_id == page_snapshot_id && *regen_job_id == job_id => Some(Ok((
                    "completed".to_string(),
                    format!("Extracted {} posts from page", posts_count),
                ))),
                // Failure events
                PostEvent::AuthorizationDenied {
                    user_id,
                    action,
                    reason,
                } if *user_id == user.member_id && action == "RegeneratePagePosts" => {
                    Some(Err(anyhow::anyhow!("Authorization denied: {}", reason)))
                }
                _ => None,
            })
            .result()
        },
    )
    .await
    .map_err(|e| {
        FieldError::new(
            format!("Failed to regenerate page posts: {}", e),
            juniper::Value::null(),
        )
    })?;

    let (status, message) = result;

    Ok(ScrapeJobResult {
        job_id: job_id.into_uuid(),
        source_id: page_snapshot_id,
        status,
        message: Some(message),
    })
}

/// Refresh a page snapshot by re-scraping (admin only)
/// Re-scrapes a specific domain snapshot to update listings when page content changes
pub async fn refresh_page_snapshot(
    ctx: &GraphQLContext,
    snapshot_id: String,
) -> FieldResult<ScrapeJobResult> {
    info!(snapshot_id = %snapshot_id, "Refreshing page snapshot");

    // Get user info - must be admin
    let user = ctx
        .auth_user
        .as_ref()
        .ok_or_else(|| FieldError::new("Authentication required", juniper::Value::null()))?;

    if !user.is_admin {
        return Err(FieldError::new(
            "Admin authorization required",
            juniper::Value::null(),
        ));
    }

    // Parse snapshot ID
    let snapshot_uuid = Uuid::parse_str(&snapshot_id)
        .map_err(|_| FieldError::new("Invalid snapshot ID", juniper::Value::null()))?;

    // Get the domain snapshot
    let snapshot = WebsiteSnapshot::find_by_id(&ctx.db_pool, snapshot_uuid)
        .await
        .map_err(|e| {
            FieldError::new(
                format!("Failed to find snapshot: {}", e),
                juniper::Value::null(),
            )
        })?;

    // Get the website to verify it's approved
    let website = Website::find_by_id(snapshot.get_website_id(), &ctx.db_pool)
        .await
        .map_err(|e| {
            FieldError::new(
                format!("Failed to find website: {}", e),
                juniper::Value::null(),
            )
        })?;

    if website.status != "approved" {
        return Err(FieldError::new(
            "Website must be approved before refreshing",
            juniper::Value::null(),
        ));
    }

    // Trigger re-scrape by dispatching event (same as scrapeOrganization)
    let source_id = snapshot.get_website_id();
    let job_id = JobId::new();

    // Dispatch request event and await completion
    let result = dispatch_request(
        PostEvent::ScrapeSourceRequested {
            source_id,
            job_id,
            requested_by: user.member_id,
            is_admin: user.is_admin,
        },
        &ctx.bus,
        |m| {
            m.try_match(|e: &PostEvent| match e {
                // Success - scraping workflow complete
                PostEvent::PostsSynced {
                    source_id: synced_source_id,
                    job_id: synced_job_id,
                    new_count,
                    updated_count,
                    unchanged_count,
                } if *synced_source_id == source_id && *synced_job_id == job_id => Some(Ok((
                    "completed".to_string(),
                    format!(
                        "Refresh complete! Found {} new, {} updated, {} unchanged",
                        new_count, updated_count, unchanged_count
                    ),
                ))),
                // Failure events
                PostEvent::ScrapeFailed {
                    source_id: failed_source_id,
                    job_id: failed_job_id,
                    reason,
                } if *failed_source_id == source_id && *failed_job_id == job_id => {
                    Some(Err(anyhow::anyhow!("Scrape failed: {}", reason)))
                }
                PostEvent::ExtractFailed {
                    source_id: failed_source_id,
                    job_id: failed_job_id,
                    reason,
                } if *failed_source_id == source_id && *failed_job_id == job_id => {
                    Some(Err(anyhow::anyhow!("Extraction failed: {}", reason)))
                }
                PostEvent::SyncFailed {
                    source_id: failed_source_id,
                    job_id: failed_job_id,
                    reason,
                } if *failed_source_id == source_id && *failed_job_id == job_id => {
                    Some(Err(anyhow::anyhow!("Sync failed: {}", reason)))
                }
                PostEvent::AuthorizationDenied {
                    user_id,
                    action,
                    reason,
                } if *user_id == user.member_id && action == "ScrapeSource" => {
                    Some(Err(anyhow::anyhow!("Authorization denied: {}", reason)))
                }
                _ => None,
            })
            .result()
        },
    )
    .await
    .map_err(|e| FieldError::new(format!("Refresh failed: {}", e), juniper::Value::null()))?;

    let (status, message) = result;

    Ok(ScrapeJobResult {
        job_id: job_id.into_uuid(),
        source_id: source_id.into_uuid(),
        status,
        message: Some(message),
    })
}
