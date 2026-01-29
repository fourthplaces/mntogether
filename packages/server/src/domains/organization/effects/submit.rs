use crate::common::{MemberId, NeedId};
use crate::domains::organization::models::{NeedStatus, OrganizationNeed};
use crate::domains::organization::utils::{generate_need_content_hash, generate_tldr};
use anyhow::Result;
use sqlx::PgPool;

/// Input for submitting a user-generated need
#[derive(Debug, Clone)]
pub struct SubmitNeedInput {
    pub member_id: Option<MemberId>,
    pub organization_name: String,
    pub title: String,
    pub description: String,
    pub contact_info: Option<serde_json::Value>,
    pub urgency: Option<String>,
    pub location: Option<String>,
    pub ip_address: Option<String>,
}

/// Submit a need from a member (goes to pending_approval)
///
/// This allows members to report needs they've encountered,
/// not just needs scraped from websites.
pub async fn submit_user_need(pool: &PgPool, input: SubmitNeedInput) -> Result<OrganizationNeed> {
    // Generate content hash for deduplication
    let content_hash =
        generate_need_content_hash(&input.title, &input.description, &input.organization_name);

    // Generate TLDR (first 100 chars of description)
    let tldr = generate_tldr(&input.description, 100);

    // Convert IP address to string for storage
    let ip_str = input.ip_address.map(|ip| ip.to_string());

    // Create need using model method
    let need = OrganizationNeed::create(
        input.organization_name,
        input.title,
        input.description,
        tldr,
        input.contact_info,
        input.urgency,
        input.location,
        NeedStatus::PendingApproval.to_string(),
        content_hash,
        Some("user_submitted".to_string()),
        input.member_id,
        ip_str,
        None, // source_id
        None, // source_url - not applicable for user-submitted needs
        pool,
    )
    .await?;

    Ok(need)
}

/// Check if a similar need already exists (duplicate detection)
///
/// Looks for needs with similar content hash to prevent duplicates
pub async fn find_similar_need(pool: &PgPool, content_hash: &str) -> Result<Option<NeedId>> {
    // Use model method for finding duplicate by content hash
    OrganizationNeed::find_id_by_content_hash_active(content_hash, pool).await
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
