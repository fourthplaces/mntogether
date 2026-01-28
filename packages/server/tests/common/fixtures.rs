//! Test fixtures for creating test data.

use anyhow::Result;
use server_core::domains::organization::models::{NeedStatus, OrganizationNeed, OrganizationSource};
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

/// Create a test organization source
pub async fn create_test_source(
    pool: &PgPool,
    organization_name: &str,
    source_url: &str,
) -> Result<Uuid> {
    let row = sqlx::query!(
        r#"
        INSERT INTO organization_sources (organization_name, source_url, active)
        VALUES ($1, $2, true)
        RETURNING id
        "#,
        organization_name,
        source_url
    )
    .fetch_one(pool)
    .await?;

    Ok(row.id)
}

/// Create a test need with pending_approval status
pub async fn create_test_need_pending(
    pool: &PgPool,
    source_id: Option<Uuid>,
    title: &str,
    description: &str,
) -> Result<Uuid> {
    let row = sqlx::query!(
        r#"
        INSERT INTO organization_needs (
            organization_name,
            title,
            description,
            tldr,
            status,
            source_id
        ) VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING id
        "#,
        "Test Organization",
        title,
        description,
        "Test TLDR",
        NeedStatus::PendingApproval.to_string(),
        source_id
    )
    .fetch_one(pool)
    .await?;

    Ok(row.id)
}

/// Create a test need with active status
pub async fn create_test_need_active(
    pool: &PgPool,
    title: &str,
    description: &str,
) -> Result<Uuid> {
    let row = sqlx::query!(
        r#"
        INSERT INTO organization_needs (
            organization_name,
            title,
            description,
            tldr,
            status
        ) VALUES ($1, $2, $3, $4, $5)
        RETURNING id
        "#,
        "Test Organization",
        title,
        description,
        "Test TLDR",
        NeedStatus::Active.to_string()
    )
    .fetch_one(pool)
    .await?;

    Ok(row.id)
}

/// Create a full test need with all fields
pub async fn create_test_need_full(
    pool: &PgPool,
    organization_name: &str,
    title: &str,
    description: &str,
    tldr: &str,
    contact_json: Option<serde_json::Value>,
    urgency: Option<&str>,
    status: NeedStatus,
) -> Result<Uuid> {
    let row = sqlx::query!(
        r#"
        INSERT INTO organization_needs (
            organization_name,
            title,
            description,
            tldr,
            contact_info,
            urgency,
            status
        ) VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING id
        "#,
        organization_name,
        title,
        description,
        tldr,
        contact_json,
        urgency,
        status.to_string()
    )
    .fetch_one(pool)
    .await?;

    Ok(row.id)
}

/// Clean all needs from database (for test isolation)
pub async fn clean_needs(pool: &PgPool) -> Result<()> {
    sqlx::query!("DELETE FROM organization_needs")
        .execute(pool)
        .await?;
    Ok(())
}

/// Clean all sources from database (for test isolation)
pub async fn clean_sources(pool: &PgPool) -> Result<()> {
    sqlx::query!("DELETE FROM organization_sources")
        .execute(pool)
        .await?;
    Ok(())
}
