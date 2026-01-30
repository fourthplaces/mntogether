use crate::domains::listings::data::{ListingConnection, ListingStatusData, ListingType};
use crate::common::ListingId;
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
    let listings = Listing::find_by_status(
        status_filter,
        limit as i64,
        offset as i64,
        pool,
    )
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
        .map_err(|e| juniper::FieldError::new(
            format!("Failed to fetch posts: {}", e),
            juniper::Value::null(),
        ))?;

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
        .map_err(|e| juniper::FieldError::new(
            format!("Failed to fetch published posts: {}", e),
            juniper::Value::null(),
        ))?;

    Ok(posts.into_iter().map(PostData::from).collect())
}

/// Get all organization sources
pub async fn query_organization_sources(
    pool: &PgPool,
) -> FieldResult<Vec<crate::domains::organization::data::SourceData>> {
    use crate::domains::organization::data::SourceData;
    use crate::domains::scraping::models::Domain;

    let sources = Domain::find_active(pool)
        .await
        .map_err(|e| juniper::FieldError::new(
            format!("Failed to fetch organization sources: {}", e),
            juniper::Value::null(),
        ))?;

    Ok(sources.into_iter().map(SourceData::from).collect())
}

/// Get a single organization source by ID
pub async fn query_organization_source(
    pool: &PgPool,
    id: Uuid,
) -> FieldResult<Option<crate::domains::organization::data::SourceData>> {
    use crate::common::DomainId;
    use crate::domains::organization::data::SourceData;
    use crate::domains::scraping::models::Domain;

    let source_id = DomainId::from_uuid(id);

    match Domain::find_by_id(source_id, pool).await {
        Ok(source) => Ok(Some(SourceData::from(source))),
        Err(_) => Ok(None),
    }
}

/// Query all domains with optional status filter
pub async fn query_domains(
    pool: &PgPool,
    status: Option<String>,
) -> FieldResult<Vec<crate::domains::organization::data::SourceData>> {
    use crate::domains::organization::data::SourceData;
    use crate::domains::scraping::models::Domain;
    use anyhow::Context;

    let domains = if let Some(status_filter) = status {
        match status_filter.as_str() {
            "pending_review" => Domain::find_pending_review(pool).await,
            "approved" => Domain::find_approved(pool).await,
            _ => Domain::find_active(pool).await,
        }
    } else {
        // Return all domains if no filter specified
        sqlx::query_as::<_, Domain>("SELECT * FROM domains ORDER BY created_at DESC")
            .fetch_all(pool)
            .await
            .context("Failed to query all domains")
    }
    .map_err(|e| juniper::FieldError::new(
        format!("Failed to fetch domains: {}", e),
        juniper::Value::null(),
    ))?;

    Ok(domains.into_iter().map(SourceData::from).collect())
}

/// Query domains pending review (for admin approval queue)
pub async fn query_pending_domains(
    pool: &PgPool,
) -> FieldResult<Vec<crate::domains::organization::data::SourceData>> {
    use crate::domains::organization::data::SourceData;
    use crate::domains::scraping::models::Domain;

    let domains = Domain::find_pending_review(pool)
        .await
        .map_err(|e| juniper::FieldError::new(
            format!("Failed to fetch pending domains: {}", e),
            juniper::Value::null(),
        ))?;

    Ok(domains.into_iter().map(SourceData::from).collect())
}
