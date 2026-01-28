use crate::common::utils::generate_content_hash;
use crate::domains::organization::models::{NeedStatus, OrganizationNeed};
use anyhow::Result;
use sqlx::PgPool;
use std::net::IpAddr;
use uuid::Uuid;

/// Input for submitting a user-generated need
#[derive(Debug, Clone)]
pub struct SubmitNeedInput {
    pub volunteer_id: Option<Uuid>,
    pub organization_name: String,
    pub title: String,
    pub description: String,
    pub contact_info: Option<serde_json::Value>,
    pub urgency: Option<String>,
    pub location: Option<String>,
    pub ip_address: Option<String>,
}

/// Submit a need from a volunteer (goes to pending_approval)
///
/// This allows volunteers to report needs they've encountered,
/// not just needs scraped from websites.
pub async fn submit_user_need(pool: &PgPool, input: SubmitNeedInput) -> Result<OrganizationNeed> {
    // Generate content hash for deduplication
    let content_hash = generate_content_hash(&format!(
        "{} {} {}",
        input.title, input.description, input.organization_name
    ));

    // Generate TLDR (first 100 chars of description)
    let tldr = if input.description.len() > 100 {
        format!("{}...", &input.description[..97])
    } else {
        input.description.clone()
    };

    // Convert IP address to string for storage
    let ip_str = input.ip_address.map(|ip| ip.to_string());

    let need = sqlx::query_as!(
        OrganizationNeed,
        r#"
        INSERT INTO organization_needs (
            organization_name,
            title,
            description,
            tldr,
            contact_info,
            urgency,
            location,
            status,
            content_hash,
            submission_type,
            submitted_by_volunteer_id,
            submitted_from_ip
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12::inet)
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
        input.organization_name,
        input.title,
        input.description,
        tldr,
        input.contact_info,
        input.urgency,
        input.location,
        NeedStatus::PendingApproval.to_string(),
        content_hash,
        "user_submitted",
        input.volunteer_id,
        ip_str
    )
    .fetch_one(pool)
    .await?;

    // Parse status string back to enum
    let mut need_with_status = need;
    need_with_status.status = match need_with_status.status.as_str() {
        "pending_approval" => NeedStatus::PendingApproval,
        "active" => NeedStatus::Active,
        "rejected" => NeedStatus::Rejected,
        "expired" => NeedStatus::Expired,
        _ => NeedStatus::PendingApproval,
    };

    Ok(need_with_status)
}

/// Check if a similar need already exists (duplicate detection)
///
/// Looks for needs with similar content hash to prevent duplicates
pub async fn find_similar_need(pool: &PgPool, content_hash: &str) -> Result<Option<Uuid>> {
    let row = sqlx::query!(
        r#"
        SELECT id
        FROM organization_needs
        WHERE content_hash = $1
          AND status IN ('pending_approval', 'active')
        LIMIT 1
        "#,
        content_hash
    )
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| r.id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tldr_generation() {
        let short_desc = "Short description";
        let tldr = if short_desc.len() > 100 {
            format!("{}...", &short_desc[..97])
        } else {
            short_desc.to_string()
        };
        assert_eq!(tldr, "Short description");

        let long_desc = "a".repeat(150);
        let tldr = if long_desc.len() > 100 {
            format!("{}...", &long_desc[..97])
        } else {
            long_desc.clone()
        };
        assert_eq!(tldr.len(), 100); // 97 chars + "..."
    }
}
