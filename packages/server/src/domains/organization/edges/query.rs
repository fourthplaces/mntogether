use super::types::{Need, NeedConnection, NeedStatusGql};
use crate::domains::organization::models::{NeedStatus, OrganizationNeed};
use juniper::{FieldError, FieldResult};
use sqlx::PgPool;
use uuid::Uuid;

/// Query needs with filters and pagination
pub async fn query_needs(
    pool: &PgPool,
    status: Option<NeedStatusGql>,
    limit: Option<i32>,
    offset: Option<i32>,
) -> FieldResult<NeedConnection> {
    let limit = limit.unwrap_or(50).min(100); // Cap at 100
    let offset = offset.unwrap_or(0);

    // Default to active status if not specified
    let status_filter = match status {
        Some(NeedStatusGql::Active) | None => NeedStatus::Active,
        Some(NeedStatusGql::PendingApproval) => NeedStatus::PendingApproval,
        Some(NeedStatusGql::Rejected) => NeedStatus::Rejected,
        Some(NeedStatusGql::Expired) => NeedStatus::Expired,
    };

    // Fetch needs using model method
    let needs = OrganizationNeed::find_by_status(
        &status_filter.to_string(),
        limit as i64,
        offset as i64,
        pool,
    )
    .await
    .map_err(|_| FieldError::new("Database error", juniper::Value::null()))?;

    // Count total using model method
    let total_count = OrganizationNeed::count_by_status(&status_filter.to_string(), pool)
        .await
        .map_err(|_| FieldError::new("Database error", juniper::Value::null()))?;

    let has_next_page = (offset + limit) < total_count as i32;

    Ok(NeedConnection {
        nodes: needs.into_iter().map(Need::from).collect(),
        total_count: total_count as i32,
        has_next_page,
    })
}

/// Get a single need by ID
pub async fn query_need(pool: &PgPool, id: Uuid) -> FieldResult<Option<Need>> {
    // Use model method - converts Result to Option for non-existent records
    let need = OrganizationNeed::find_by_id(id, pool).await.ok(); // Convert Result<Need> to Option<Need>

    Ok(need.map(Need::from))
}
