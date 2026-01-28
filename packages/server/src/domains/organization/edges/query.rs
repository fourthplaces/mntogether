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

    // Fetch needs
    let needs = sqlx::query_as::<_, OrganizationNeed>(
        r#"
        SELECT *
        FROM organization_needs
        WHERE status = $1
        ORDER BY created_at DESC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(status_filter.to_string())
    .bind(limit as i64)
    .bind(offset as i64)
    .fetch_all(pool)
    .await
    .map_err(|e| FieldError::new("Database error", juniper::Value::null()))?;

    // Count total
    let total_count = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)
        FROM organization_needs
        WHERE status = $1
        "#,
    )
    .bind(status_filter.to_string())
    .fetch_one(pool)
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
    let need = sqlx::query_as::<_, OrganizationNeed>(
        r#"
        SELECT *
        FROM organization_needs
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|_| FieldError::new("Database error", juniper::Value::null()))?;

    Ok(need.map(Need::from))
}
