use crate::common::ListingId;
use crate::domains::listings::data::listing_report::{ListingReport, ListingReportDetail};
use crate::domains::listings::data::{ListingConnection, ListingStatusData, ListingType};
use crate::domains::listings::models::listing_report::ListingReportRecord;
use crate::domains::listings::models::Listing;
use crate::server::graphql::context::GraphQLContext;
use juniper::{FieldError, FieldResult};
use sqlx::PgPool;
use uuid::Uuid;

/// Query listings with filters and pagination
pub async fn query_listings(
    pool: &PgPool,
    status: Option<ListingStatusData>,
    limit: Option<i32>,
    offset: Option<i32>,
) -> FieldResult<ListingConnection> {
    let limit = limit.unwrap_or(50).min(100); // Cap at 100
    let offset = offset.unwrap_or(0);

    // Default to active status if not specified
    let status_filter = match status {
        Some(ListingStatusData::Active) | None => "active",
        Some(ListingStatusData::PendingApproval) => "pending_approval",
        Some(ListingStatusData::Rejected) => "rejected",
        Some(ListingStatusData::Expired) => "expired",
        Some(ListingStatusData::Filled) => "filled",
    };

    // Fetch listings using model method
    let listings = Listing::find_by_status(status_filter, limit as i64, offset as i64, pool)
        .await
        .map_err(|_| FieldError::new("Database error", juniper::Value::null()))?;

    // Count total using model method
    let total_count = Listing::count_by_status(status_filter, pool)
        .await
        .map_err(|_| FieldError::new("Database error", juniper::Value::null()))?;

    let has_next_page = (offset + limit) < total_count as i32;

    Ok(ListingConnection {
        nodes: listings.into_iter().map(ListingType::from).collect(),
        total_count: total_count as i32,
        has_next_page,
    })
}

/// Get a single listing by ID
pub async fn query_listing(pool: &PgPool, id: Uuid) -> FieldResult<Option<ListingType>> {
    // Convert to typed ID
    let listing_id = ListingId::from_uuid(id);

    // Use model method - converts Result to Option for non-existent records
    let listing = Listing::find_by_id(listing_id, pool).await.ok(); // Convert Result<Listing> to Option<Listing>

    Ok(listing.map(ListingType::from))
}

/// Get a single post by ID
pub async fn query_post(
    ctx: &GraphQLContext,
    id: Uuid,
) -> FieldResult<Option<crate::domains::organization::data::PostData>> {
    use crate::common::PostId;
    use crate::domains::organization::data::PostData;
    use crate::domains::organization::models::Post;

    let post_id = PostId::from_uuid(id);

    match Post::find_by_id(post_id, &ctx.db_pool).await {
        Ok(Some(post)) => Ok(Some(PostData::from(post))),
        Ok(None) => Ok(None),
        Err(_) => Ok(None),
    }
}

/// Get posts for a specific listing
pub async fn query_posts_for_listing(
    ctx: &GraphQLContext,
    listing_id: Uuid,
) -> FieldResult<Vec<crate::domains::organization::data::PostData>> {
    use crate::common::ListingId;
    use crate::domains::organization::data::PostData;
    use crate::domains::organization::models::Post;

    let listing_id = ListingId::from_uuid(listing_id);

    let posts = Post::find_by_listing_id(listing_id, &ctx.db_pool)
        .await
        .map_err(|e| {
            juniper::FieldError::new(
                format!("Failed to fetch posts: {}", e),
                juniper::Value::null(),
            )
        })?;

    Ok(posts.into_iter().map(PostData::from).collect())
}

/// Get published posts (for public feed)
pub async fn query_published_posts(
    ctx: &GraphQLContext,
    limit: Option<i32>,
) -> FieldResult<Vec<crate::domains::organization::data::PostData>> {
    use crate::domains::organization::data::PostData;
    use crate::domains::organization::models::Post;

    let limit = Some(limit.unwrap_or(50).min(100) as i64);

    let posts = Post::find_published(limit, &ctx.db_pool)
        .await
        .map_err(|e| {
            juniper::FieldError::new(
                format!("Failed to fetch published posts: {}", e),
                juniper::Value::null(),
            )
        })?;

    Ok(posts.into_iter().map(PostData::from).collect())
}

/// Get a single website by ID
pub async fn query_website(
    pool: &PgPool,
    id: Uuid,
) -> FieldResult<Option<crate::domains::organization::data::WebsiteData>> {
    use crate::common::WebsiteId;
    use crate::domains::organization::data::WebsiteData;
    use crate::domains::scraping::models::Website;

    let website_id = WebsiteId::from_uuid(id);

    match Website::find_by_id(website_id, pool).await {
        Ok(website) => Ok(Some(WebsiteData::from(website))),
        Err(_) => Ok(None),
    }
}

/// Query all websites with optional status and agent filters
pub async fn query_websites(
    pool: &PgPool,
    status: Option<String>,
    agent_id: Option<String>,
) -> FieldResult<Vec<crate::domains::organization::data::WebsiteData>> {
    use crate::domains::organization::data::WebsiteData;
    use crate::domains::scraping::models::Website;
    use anyhow::Context;
    use uuid::Uuid;

    // If agent_id is provided, filter by agent
    if let Some(agent_id_str) = agent_id {
        let agent_uuid = Uuid::parse_str(&agent_id_str).map_err(|_| {
            juniper::FieldError::new("Invalid agent ID format", juniper::Value::null())
        })?;

        let websites = Website::find_by_agent_id(agent_uuid, pool)
            .await
            .map_err(|e| {
                juniper::FieldError::new(
                    format!("Failed to fetch websites by agent: {}", e),
                    juniper::Value::null(),
                )
            })?;

        return Ok(websites.into_iter().map(WebsiteData::from).collect());
    }

    let websites = if let Some(status_filter) = status {
        match status_filter.as_str() {
            "pending_review" => Website::find_pending_review(pool).await,
            "approved" => Website::find_approved(pool).await,
            _ => Website::find_active(pool).await,
        }
    } else {
        // Return all websites if no filter specified
        sqlx::query_as::<_, Website>("SELECT * FROM websites ORDER BY created_at DESC")
            .fetch_all(pool)
            .await
            .context("Failed to query all websites")
    }
    .map_err(|e| {
        juniper::FieldError::new(
            format!("Failed to fetch websites: {}", e),
            juniper::Value::null(),
        )
    })?;

    Ok(websites.into_iter().map(WebsiteData::from).collect())
}

/// Query websites pending review (for admin approval queue)
pub async fn query_pending_websites(
    pool: &PgPool,
) -> FieldResult<Vec<crate::domains::organization::data::WebsiteData>> {
    use crate::domains::organization::data::WebsiteData;
    use crate::domains::scraping::models::Website;

    let websites = Website::find_pending_review(pool).await.map_err(|e| {
        juniper::FieldError::new(
            format!("Failed to fetch pending websites: {}", e),
            juniper::Value::null(),
        )
    })?;

    Ok(websites.into_iter().map(WebsiteData::from).collect())
}

/// Get all reports (admin only)
pub async fn query_listing_reports(
    ctx: &GraphQLContext,
    status: Option<String>,
    limit: Option<i32>,
    offset: Option<i32>,
) -> FieldResult<Vec<ListingReportDetail>> {
    let user = ctx
        .auth_user
        .as_ref()
        .ok_or_else(|| FieldError::new("Authentication required", juniper::Value::null()))?;

    if !user.is_admin {
        return Err(FieldError::new(
            "Admin access required",
            juniper::Value::null(),
        ));
    }

    let limit = limit.unwrap_or(50) as i64;
    let offset = offset.unwrap_or(0) as i64;

    let reports = match status.as_deref() {
        Some("pending") | None => ListingReportRecord::query_pending(limit, offset, &ctx.db_pool)
            .await
            .map_err(|e| {
                FieldError::new(
                    format!("Failed to fetch reports: {}", e),
                    juniper::Value::null(),
                )
            })?,
        _ => ListingReportRecord::query_all(limit, offset, &ctx.db_pool)
            .await
            .map_err(|e| {
                FieldError::new(
                    format!("Failed to fetch reports: {}", e),
                    juniper::Value::null(),
                )
            })?,
    };

    Ok(reports.into_iter().map(|r| r.into()).collect())
}

/// Get reports for a specific listing (admin only)
pub async fn query_reports_for_listing(
    ctx: &GraphQLContext,
    listing_id: Uuid,
) -> FieldResult<Vec<ListingReport>> {
    let user = ctx
        .auth_user
        .as_ref()
        .ok_or_else(|| FieldError::new("Authentication required", juniper::Value::null()))?;

    if !user.is_admin {
        return Err(FieldError::new(
            "Admin access required",
            juniper::Value::null(),
        ));
    }

    let listing_id = ListingId::from_uuid(listing_id);
    let reports = ListingReportRecord::query_for_listing(listing_id, &ctx.db_pool)
        .await
        .map_err(|e| {
            FieldError::new(
                format!("Failed to fetch reports: {}", e),
                juniper::Value::null(),
            )
        })?;

    Ok(reports.into_iter().map(|r| r.into()).collect())
}
