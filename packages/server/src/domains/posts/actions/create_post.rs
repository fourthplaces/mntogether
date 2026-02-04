//! Post creation action
//!
//! Centralized logic for creating posts with all associated data
//! (contact info, audience role tags, page source links).

use anyhow::Result;
use sqlx::PgPool;
use tracing::warn;
use uuid::Uuid;

use crate::common::{ContactInfo, ExtractedPost, PostId, WebsiteId};
use crate::domains::posts::models::{Post, PostContact, PostStatus};
use crate::domains::tag::models::{Tag, Taggable};

/// Valid urgency values per database constraint
const VALID_URGENCY_VALUES: &[&str] = &["low", "medium", "high", "urgent"];

/// Normalize urgency value to a valid database value.
/// Returns None if the input is invalid or None.
fn normalize_urgency(urgency: Option<&str>) -> Option<String> {
    urgency.and_then(|u| {
        let normalized = u.to_lowercase();
        if VALID_URGENCY_VALUES.contains(&normalized.as_str()) {
            Some(normalized)
        } else {
            warn!(urgency = %u, "Invalid urgency value from AI, ignoring");
            None
        }
    })
}

/// Create a post from extracted data with all associated records.
///
/// This is the single place that handles:
/// - Creating the post record
/// - Creating contact info records
/// - Tagging with audience roles
/// - Linking to source page snapshot
///
/// All sync functions should use this instead of calling Post::create directly.
pub async fn create_extracted_post(
    organization_name: &str,
    post: &ExtractedPost,
    website_id: Option<WebsiteId>,
    source_url: Option<String>,
    pool: &PgPool,
) -> Result<Post> {
    let urgency = normalize_urgency(post.urgency.as_deref());

    // Create the post
    let created = Post::create(
        organization_name.to_string(),
        post.title.clone(),
        post.description.clone(),
        Some(post.tldr.clone()),
        "opportunity".to_string(),
        "general".to_string(),
        Some("accepting".to_string()),
        urgency,
        post.location.clone(),
        PostStatus::PendingApproval.to_string(),
        "en".to_string(),
        Some("scraped".to_string()),
        None, // submitted_by_admin_id
        website_id,
        source_url,
        None, // organization_id
        None, // revision_of_post_id
        pool,
    )
    .await?;

    // Create contact info if available
    if let Some(ref contact) = post.contact {
        save_contact_info(created.id, contact, pool).await;
    }

    // Tag post with audience roles
    tag_with_audience_roles(created.id, &post.audience_roles, pool).await;

    // Link post to source page snapshot
    if let Some(page_snapshot_id) = post.source_page_snapshot_id {
        link_to_page_source(created.id, page_snapshot_id, pool).await;
    }

    Ok(created)
}

/// Save contact info for a post.
async fn save_contact_info(post_id: PostId, contact: &ContactInfo, pool: &PgPool) {
    let contact_json = serde_json::json!({
        "phone": contact.phone,
        "email": contact.email,
        "website": contact.website,
        "intake_form_url": contact.intake_form_url,
        "contact_name": contact.contact_name,
    });

    if let Err(e) = PostContact::create_from_json(post_id, &contact_json, pool).await {
        warn!(
            post_id = %post_id,
            error = %e,
            "Failed to save contact info"
        );
    }
}

/// Tag a post with audience roles.
pub async fn tag_with_audience_roles(post_id: PostId, audience_roles: &[String], pool: &PgPool) {
    for role in audience_roles {
        let normalized_role = role.to_lowercase();
        match Tag::find_by_kind_value("audience_role", &normalized_role, pool).await {
            Ok(Some(tag)) => {
                if let Err(e) = Taggable::create_post_tag(post_id, tag.id, pool).await {
                    warn!(
                        post_id = %post_id,
                        role = %normalized_role,
                        error = %e,
                        "Failed to tag post with audience role"
                    );
                }
            }
            Ok(None) => {
                warn!(role = %normalized_role, "Unknown audience role from AI");
            }
            Err(e) => {
                warn!(
                    role = %normalized_role,
                    error = %e,
                    "Failed to look up audience role tag"
                );
            }
        }
    }
}

/// Link a post to its source page snapshot.
async fn link_to_page_source(post_id: PostId, page_snapshot_id: Uuid, pool: &PgPool) {
    if let Err(e) = sqlx::query(
        "INSERT INTO post_page_sources (post_id, page_snapshot_id) VALUES ($1, $2) ON CONFLICT DO NOTHING"
    )
    .bind(post_id.into_uuid())
    .bind(page_snapshot_id)
    .execute(pool)
    .await
    {
        warn!(
            post_id = %post_id,
            page_snapshot_id = %page_snapshot_id,
            error = %e,
            "Failed to link post to page source"
        );
    }
}
