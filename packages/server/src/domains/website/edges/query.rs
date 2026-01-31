use crate::common::WebsiteId;
use crate::domains::website::data::WebsiteData;
use crate::domains::website::models::Website;
use anyhow::Context;
use juniper::FieldResult;
use sqlx::PgPool;
use uuid::Uuid;

/// Get a single website by ID
pub async fn query_website(pool: &PgPool, id: Uuid) -> FieldResult<Option<WebsiteData>> {
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
) -> FieldResult<Vec<WebsiteData>> {
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
pub async fn query_pending_websites(pool: &PgPool) -> FieldResult<Vec<WebsiteData>> {
    let websites = Website::find_pending_review(pool).await.map_err(|e| {
        juniper::FieldError::new(
            format!("Failed to fetch pending websites: {}", e),
            juniper::Value::null(),
        )
    })?;

    Ok(websites.into_iter().map(WebsiteData::from).collect())
}
