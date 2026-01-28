use super::types::{Need, NeedConnection, NeedStatusData, OrganizationSourceData};
use crate::common::{NeedId, SourceId};
use crate::domains::organization::models::{
    source::OrganizationSource, NeedStatus, OrganizationNeed,
};
use juniper::{FieldError, FieldResult};
use sqlx::PgPool;
use uuid::Uuid;

/// Query needs with filters and pagination
pub async fn query_needs(
    pool: &PgPool,
    status: Option<NeedStatusData>,
    limit: Option<i32>,
    offset: Option<i32>,
) -> FieldResult<NeedConnection> {
    let limit = limit.unwrap_or(50).min(100); // Cap at 100
    let offset = offset.unwrap_or(0);

    // Default to active status if not specified
    let status_filter = match status {
        Some(NeedStatusData::Active) | None => NeedStatus::Active,
        Some(NeedStatusData::PendingApproval) => NeedStatus::PendingApproval,
        Some(NeedStatusData::Rejected) => NeedStatus::Rejected,
        Some(NeedStatusData::Expired) => NeedStatus::Expired,
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
    // Convert to typed ID
    let need_id = NeedId::from_uuid(id);

    // Use model method - converts Result to Option for non-existent records
    let need = OrganizationNeed::find_by_id(need_id, pool).await.ok(); // Convert Result<Need> to Option<Need>

    Ok(need.map(Need::from))
}

/// Query all organization sources
pub async fn query_organization_sources(pool: &PgPool) -> FieldResult<Vec<OrganizationSourceData>> {
    let sources = OrganizationSource::find_active(pool).await.map_err(|e| {
        FieldError::new(
            format!("Failed to fetch organization sources: {}", e),
            juniper::Value::null(),
        )
    })?;

    Ok(sources
        .into_iter()
        .map(OrganizationSourceData::from)
        .collect())
}

/// Get a single organization source by ID
pub async fn query_organization_source(
    pool: &PgPool,
    id: Uuid,
) -> FieldResult<Option<OrganizationSourceData>> {
    // Convert to typed ID
    let source_id = SourceId::from_uuid(id);

    let source = OrganizationSource::find_by_id(source_id, pool).await.ok();

    Ok(source.map(OrganizationSourceData::from))
}
