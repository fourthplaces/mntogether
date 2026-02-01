use crate::common::PostId;
use crate::domains::posts::data::post_report::{PostReport, PostReportDetail};
use crate::domains::posts::data::{BusinessInfo, PostConnection, PostStatusData, PostType};
use crate::domains::posts::models::post_report::PostReportRecord;
use crate::domains::posts::models::{BusinessPost, Post};
use crate::server::graphql::context::GraphQLContext;
use juniper::{FieldError, FieldResult};
use sqlx::PgPool;
use uuid::Uuid;

/// Convert a Post to PostType, loading business_info for business listings
async fn post_to_post_type(post: Post, pool: &PgPool) -> PostType {
    let mut post_type = PostType::from(post.clone());

    // Load business_info if this is a business listing
    if post.post_type == "business" {
        if let Ok(Some(business)) = BusinessPost::find_by_post_id(post.id, pool).await {
            post_type.business_info = Some(BusinessInfo {
                accepts_donations: business.accepts_donations,
                donation_link: business.donation_link,
                gift_cards_available: business.gift_cards_available,
                gift_card_link: business.gift_card_link,
                online_ordering_link: business.online_ordering_link,
                delivery_available: business.delivery_available,
                proceeds_percentage: business.proceeds_percentage,
                proceeds_beneficiary_id: business.proceeds_beneficiary_id.map(|id| id.into_uuid()),
                proceeds_description: business.proceeds_description,
                impact_statement: business.impact_statement,
            });
        }
    }

    post_type
}

/// Query listings with filters and pagination
pub async fn query_posts(
    pool: &PgPool,
    status: Option<PostStatusData>,
    limit: Option<i32>,
    offset: Option<i32>,
) -> FieldResult<PostConnection> {
    let limit = limit.unwrap_or(50).min(100); // Cap at 100
    let offset = offset.unwrap_or(0);

    // Default to active status if not specified
    let status_filter = match status {
        Some(PostStatusData::Active) | None => "active",
        Some(PostStatusData::PendingApproval) => "pending_approval",
        Some(PostStatusData::Rejected) => "rejected",
        Some(PostStatusData::Expired) => "expired",
        Some(PostStatusData::Filled) => "filled",
    };

    // Fetch posts using model method
    let posts = Post::find_by_status(status_filter, limit as i64, offset as i64, pool)
        .await
        .map_err(|_| FieldError::new("Database error", juniper::Value::null()))?;

    // Count total using model method
    let total_count = Post::count_by_status(status_filter, pool)
        .await
        .map_err(|_| FieldError::new("Database error", juniper::Value::null()))?;

    let has_next_page = (offset + limit) < total_count as i32;

    // Convert posts with business_info
    let mut nodes = Vec::with_capacity(posts.len());
    for post in posts {
        nodes.push(post_to_post_type(post, pool).await);
    }

    Ok(PostConnection {
        nodes,
        total_count: total_count as i32,
        has_next_page,
    })
}

/// Get a single listing by ID (for admin/GraphQL)
pub async fn query_listing(pool: &PgPool, id: Uuid) -> FieldResult<Option<PostType>> {
    // Convert to typed ID
    let post_id = PostId::from_uuid(id);

    // Use model method - converts Result to Option for non-existent records
    let post = Post::find_by_id(post_id, pool).await.ok().flatten();

    // Convert with business_info loading
    match post {
        Some(p) => Ok(Some(post_to_post_type(p, pool).await)),
        None => Ok(None),
    }
}

/// Get a single post by ID
pub async fn query_post(
    ctx: &GraphQLContext,
    id: Uuid,
) -> FieldResult<Option<crate::domains::posts::data::PostData>> {
    use crate::common::PostId;
    use crate::domains::posts::data::PostData;
    use crate::domains::posts::models::Post;

    let post_id = PostId::from_uuid(id);

    match Post::find_by_id(post_id, &ctx.db_pool).await {
        Ok(Some(post)) => Ok(Some(PostData::from(post))),
        Ok(None) => Ok(None),
        Err(_) => Ok(None),
    }
}

/// Get posts for a specific listing
/// Note: This function is deprecated - the announcement model was removed
pub async fn query_posts_for_post(
    _ctx: &GraphQLContext,
    _post_id: Uuid,
) -> FieldResult<Vec<crate::domains::posts::data::PostData>> {
    // The announcement model was removed, so this function now returns an empty list
    Ok(vec![])
}

/// Get published posts (for public feed)
/// Note: This function is deprecated - the announcement model was removed
pub async fn query_published_posts(
    _ctx: &GraphQLContext,
    _limit: Option<i32>,
) -> FieldResult<Vec<crate::domains::posts::data::PostData>> {
    // The announcement model was removed, so this function now returns an empty list
    Ok(vec![])
}

// Re-export website queries from the website domain
pub use crate::domains::website::edges::query::{
    query_pending_websites, query_website, query_websites,
};

/// Get all reports (admin only)
pub async fn query_post_reports(
    ctx: &GraphQLContext,
    status: Option<String>,
    limit: Option<i32>,
    offset: Option<i32>,
) -> FieldResult<Vec<PostReportDetail>> {
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
        Some("pending") | None => PostReportRecord::query_pending(limit, offset, &ctx.db_pool)
            .await
            .map_err(|e| {
                FieldError::new(
                    format!("Failed to fetch reports: {}", e),
                    juniper::Value::null(),
                )
            })?,
        _ => PostReportRecord::query_all(limit, offset, &ctx.db_pool)
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
pub async fn query_reports_for_post(
    ctx: &GraphQLContext,
    post_id: Uuid,
) -> FieldResult<Vec<PostReport>> {
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

    let post_id = PostId::from_uuid(post_id);
    let reports = PostReportRecord::query_for_post(post_id, &ctx.db_pool)
        .await
        .map_err(|e| {
            FieldError::new(
                format!("Failed to fetch reports: {}", e),
                juniper::Value::null(),
            )
        })?;

    Ok(reports.into_iter().map(|r| r.into()).collect())
}
