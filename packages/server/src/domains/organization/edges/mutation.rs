use super::types::{EditNeedInput, Need, ScrapeResult, SubmitNeedInput};
use crate::domains::organization::effects::{
    submit_user_need, sync_needs, ExtractedNeedInput, FirecrawlClient, NeedExtractor,
    SubmitNeedInput as SubmitNeedEffectInput,
};
use crate::domains::organization::models::{NeedStatus, OrganizationNeed};
use juniper::{FieldError, FieldResult};
use sqlx::PgPool;
use std::net::IpAddr;
use uuid::Uuid;

/// Scrape an organization source and sync needs
pub async fn scrape_organization(
    pool: &PgPool,
    firecrawl_client: &FirecrawlClient,
    need_extractor: &NeedExtractor,
    source_id: Uuid,
) -> FieldResult<ScrapeResult> {
    // Fetch source
    let source = sqlx::query!(
        r#"
        SELECT organization_name, source_url
        FROM organization_sources
        WHERE id = $1 AND active = true
        "#,
        source_id
    )
    .fetch_optional(pool)
    .await
    .map_err(|_| FieldError::new("Database error", juniper::Value::null()))?
    .ok_or_else(|| FieldError::new("Source not found", juniper::Value::null()))?;

    // Scrape website
    let scrape_result = firecrawl_client
        .scrape(&source.source_url)
        .await
        .map_err(|e| FieldError::new(format!("Scraping failed: {}", e), juniper::Value::null()))?;

    // Extract needs with AI
    let extracted_needs = need_extractor
        .extract_needs(
            &source.organization_name,
            &scrape_result.markdown,
            &source.source_url,
        )
        .await
        .map_err(|e| {
            FieldError::new(
                format!("AI extraction failed: {}", e),
                juniper::Value::null(),
            )
        })?;

    // Convert to sync input
    let sync_input: Vec<ExtractedNeedInput> = extracted_needs
        .into_iter()
        .map(|need| ExtractedNeedInput {
            organization_name: source.organization_name.clone(),
            title: need.title,
            description: need.description,
            description_markdown: None, // TODO: Generate markdown from description
            tldr: Some(need.tldr),
            contact: need.contact.and_then(|c| serde_json::to_value(c).ok()),
            urgency: need.urgency,
        })
        .collect();

    // Sync with database
    let sync_result = sync_needs(pool, source_id, sync_input)
        .await
        .map_err(|e| {
            FieldError::new(
                format!("Sync failed: {}", e),
                juniper::Value::null(),
            )
        })?;

    // Update last_scraped_at
    sqlx::query!(
        r#"
        UPDATE organization_sources
        SET last_scraped_at = NOW()
        WHERE id = $1
        "#,
        source_id
    )
    .execute(pool)
    .await
    .map_err(|_| FieldError::new("Database error", juniper::Value::null()))?;

    Ok(ScrapeResult {
        source_id,
        new_needs_count: sync_result.new_needs.len() as i32,
        changed_needs_count: sync_result.changed_needs.len() as i32,
        disappeared_needs_count: sync_result.disappeared_needs.len() as i32,
    })
}

/// Submit a need from a volunteer (user-submitted, goes to pending_approval)
pub async fn submit_need(
    pool: &PgPool,
    input: SubmitNeedInput,
    volunteer_id: Option<Uuid>,
    ip_address: Option<IpAddr>,
) -> FieldResult<Need> {
    let contact_json = input
        .contact_info
        .and_then(|c| serde_json::to_value(c).ok());

    let effect_input = SubmitNeedEffectInput {
        volunteer_id,
        organization_name: input.organization_name,
        title: input.title,
        description: input.description,
        contact_info: contact_json,
        urgency: input.urgency,
        location: input.location,
        ip_address,
    };

    let need = submit_user_need(pool, effect_input)
        .await
        .map_err(|e| {
            FieldError::new(
                format!("Failed to submit need: {}", e),
                juniper::Value::null(),
            )
        })?;

    Ok(Need::from(need))
}

/// Approve a need (human-in-the-loop)
pub async fn approve_need(pool: &PgPool, need_id: Uuid) -> FieldResult<Need> {
    let need = sqlx::query_as!(
        OrganizationNeed,
        r#"
        UPDATE organization_needs
        SET status = $1, updated_at = NOW()
        WHERE id = $2
        RETURNING
            id,
            organization_name,
            title,
            description,
            description_markdown,
            tldr,
            contact_info,
            urgency,
            status as "status!: String",
            content_hash,
            source_id,
            submission_type,
            submitted_by_volunteer_id,
            location,
            last_seen_at,
            disappeared_at,
            created_at,
            updated_at
        "#,
        NeedStatus::Active.to_string(),
        need_id
    )
    .fetch_one(pool)
    .await
    .map_err(|_| FieldError::new("Database error", juniper::Value::null()))?;

    // Parse status
    let mut need_with_status = need;
    need_with_status.status = NeedStatus::Active;

    Ok(Need::from(need_with_status))
}

/// Edit and approve a need (fix AI mistakes or improve user-submitted content)
pub async fn edit_and_approve_need(
    pool: &PgPool,
    need_id: Uuid,
    input: EditNeedInput,
) -> FieldResult<Need> {
    // Build dynamic update query
    let mut updates = Vec::new();
    let mut bind_index = 2; // $1 is need_id

    if input.title.is_some() {
        updates.push(format!("title = ${}", bind_index));
        bind_index += 1;
    }
    if input.description.is_some() {
        updates.push(format!("description = ${}", bind_index));
        bind_index += 1;
    }
    if input.description_markdown.is_some() {
        updates.push(format!("description_markdown = ${}", bind_index));
        bind_index += 1;
    }
    if input.tldr.is_some() {
        updates.push(format!("tldr = ${}", bind_index));
        bind_index += 1;
    }
    if input.contact_info.is_some() {
        updates.push(format!("contact_info = ${}", bind_index));
        bind_index += 1;
    }
    if input.urgency.is_some() {
        updates.push(format!("urgency = ${}", bind_index));
        bind_index += 1;
    }
    if input.location.is_some() {
        updates.push(format!("location = ${}", bind_index));
        bind_index += 1;
    }

    // Always set status to active and update timestamp
    updates.push(format!("status = '{}'", NeedStatus::Active));
    updates.push("updated_at = NOW()".to_string());

    let query = format!(
        r#"
        UPDATE organization_needs
        SET {}
        WHERE id = $1
        RETURNING
            id,
            organization_name,
            title,
            description,
            description_markdown,
            tldr,
            contact_info,
            urgency,
            status,
            content_hash,
            source_id,
            submission_type,
            submitted_by_volunteer_id,
            location,
            last_seen_at,
            disappeared_at,
            created_at,
            updated_at
        "#,
        updates.join(", ")
    );

    let mut query_builder = sqlx::query_as::<_, OrganizationNeed>(&query).bind(need_id);

    // Bind values in same order as updates
    if let Some(title) = input.title {
        query_builder = query_builder.bind(title);
    }
    if let Some(description) = input.description {
        query_builder = query_builder.bind(description);
    }
    if let Some(description_markdown) = input.description_markdown {
        query_builder = query_builder.bind(description_markdown);
    }
    if let Some(tldr) = input.tldr {
        query_builder = query_builder.bind(tldr);
    }
    if let Some(contact_info) = input.contact_info {
        let contact_json = serde_json::to_value(contact_info)
            .map_err(|_| FieldError::new("Invalid contact info", juniper::Value::null()))?;
        query_builder = query_builder.bind(contact_json);
    }
    if let Some(urgency) = input.urgency {
        query_builder = query_builder.bind(urgency);
    }
    if let Some(location) = input.location {
        query_builder = query_builder.bind(location);
    }

    let need = query_builder
        .fetch_one(pool)
        .await
        .map_err(|_| FieldError::new("Database error", juniper::Value::null()))?;

    Ok(Need::from(need))
}

/// Reject a need (hide forever)
pub async fn reject_need(pool: &PgPool, need_id: Uuid, reason: String) -> FieldResult<bool> {
    sqlx::query!(
        r#"
        UPDATE organization_needs
        SET status = $1, updated_at = NOW()
        WHERE id = $2
        "#,
        NeedStatus::Rejected.to_string(),
        need_id
    )
    .execute(pool)
    .await
    .map_err(|_| FieldError::new("Database error", juniper::Value::null()))?;

    // TODO: Log rejection reason somewhere

    Ok(true)
}
