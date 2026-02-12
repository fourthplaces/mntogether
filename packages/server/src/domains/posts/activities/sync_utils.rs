use crate::common::utils::generate_summary;
use crate::common::PostId;
use crate::domains::contacts::Contact;
use crate::domains::posts::activities::tag_post_from_extracted;
use crate::domains::posts::models::{CreatePost, Post, UpdatePostContent};
use anyhow::Result;
use sqlx::PgPool;
use std::collections::HashMap;
use uuid::Uuid;

/// Valid urgency values per database constraint
const VALID_URGENCY_VALUES: &[&str] = &["low", "medium", "high", "urgent"];

/// Normalize urgency value to a valid database value
/// Returns None if the input is invalid or None
fn normalize_urgency(urgency: Option<String>) -> Option<String> {
    urgency.and_then(|u| {
        let normalized = u.to_lowercase();
        if VALID_URGENCY_VALUES.contains(&normalized.as_str()) {
            Some(normalized)
        } else {
            tracing::warn!(
                urgency = %u,
                "Invalid urgency value from AI, ignoring"
            );
            None
        }
    })
}

/// Result of title-based post matching/sync
#[derive(Debug)]
pub struct TitleMatchSyncResult {
    pub new_posts: Vec<PostId>,
    pub updated_posts: Vec<PostId>,
    pub unchanged_posts: Vec<PostId>,
}

/// Extracted listing input (from AI)
#[derive(Debug, Clone)]
pub struct ExtractedPostInput {
    pub title: String,
    pub description: String,
    pub description_markdown: Option<String>,
    pub summary: Option<String>,
    pub contact: Option<serde_json::Value>,
    pub location: Option<String>,
    pub urgency: Option<String>,
    pub confidence: Option<String>,
    pub source_url: Option<String>,
    /// Dynamic tags from AI extraction, keyed by tag kind slug.
    #[allow(dead_code)]
    pub tags: HashMap<String, Vec<String>>,
}

/// Synchronize extracted posts with database
///
/// Algorithm:
/// 1. For each extracted post:
///    - Check if post exists by website + title (fast path)
///    - If title match: update if content changed
///    - If no match: create new post
///
/// NOTE: Semantic deduplication is handled separately by LLM-based deduplication.
/// This function only does exact title matching.
pub async fn sync_posts(
    pool: &PgPool,
    source_type: &str,
    source_id: Uuid,
    extracted_posts: Vec<ExtractedPostInput>,
) -> Result<TitleMatchSyncResult> {
    tracing::info!(
        source_type = %source_type,
        source_id = %source_id,
        post_count = extracted_posts.len(),
        "Syncing posts"
    );

    let mut new_posts = Vec::new();
    let mut updated_posts = Vec::new();
    let mut unchanged_posts = Vec::new();

    for post_input in extracted_posts {
        // Check for exact title match
        let existing =
            Post::find_by_source_and_title(source_type, source_id, &post_input.title, pool).await?;

        if let Some(existing_post) = existing {
            // Post exists by title - check if content changed
            let content_changed = existing_post.description != post_input.description
                || existing_post.summary.as_deref() != post_input.summary.as_deref()
                || existing_post.location != post_input.location;

            if content_changed {
                // Update existing post
                Post::update_content(
                    UpdatePostContent::builder()
                        .id(existing_post.id)
                        .description(Some(post_input.description.clone()))
                        .summary(post_input.summary.clone())
                        .location(post_input.location.clone())
                        .build(),
                    pool,
                )
                .await?;

                tracing::info!(
                    post_id = %existing_post.id,
                    title = %post_input.title,
                    "Updated existing post (title match)"
                );
                updated_posts.push(existing_post.id);
            } else {
                tracing::debug!(
                    post_id = %existing_post.id,
                    title = %post_input.title,
                    "Post unchanged"
                );
                unchanged_posts.push(existing_post.id);
            }
        } else {
            // No title match - create new post
            // (LLM-based deduplication will handle semantic duplicates after sync)
            let summary = post_input
                .summary
                .clone()
                .or_else(|| Some(generate_summary(&post_input.description, 250)));

            let urgency = normalize_urgency(post_input.urgency.clone());

            match Post::create(
                CreatePost::builder()
                    .title(post_input.title.clone())
                    .description(post_input.description.clone())
                    .summary(summary)
                    .capacity_status(Some("accepting".to_string()))
                    .urgency(urgency)
                    .location(post_input.location.clone())
                    .submission_type(Some("scraped".to_string()))
                    .source_url(post_input.source_url.clone())
                    .build(),
                pool,
            )
            .await
            {
                Ok(created) => {
                    tracing::info!(
                        post_id = %created.id,
                        title = %post_input.title,
                        "Created new post"
                    );

                    // Link to source via post_sources
                    use crate::domains::posts::models::PostSource;
                    if let Err(e) = PostSource::create(
                        created.id,
                        source_type,
                        source_id,
                        post_input.source_url.as_deref(),
                        pool,
                    )
                    .await
                    {
                        tracing::warn!(
                            post_id = %created.id,
                            error = %e,
                            "Failed to create post source link"
                        );
                    }

                    // Save contact info if present
                    if let Some(ref contact_info) = post_input.contact {
                        if let Err(e) =
                            Contact::create_from_json_for_post(created.id, contact_info, pool).await
                        {
                            tracing::warn!(
                                post_id = %created.id,
                                error = %e,
                                "Failed to save contact info"
                            );
                        }
                    }

                    // Tag post with dynamic extracted tags (includes audience_role)
                    tag_post_from_extracted(created.id, &post_input.tags, pool).await;

                    new_posts.push(created.id);
                }
                Err(e) => {
                    tracing::error!(
                        error = %e,
                        title = %post_input.title,
                        "Failed to create post during sync"
                    );
                }
            }
        }
    }

    tracing::info!(
        new = new_posts.len(),
        updated = updated_posts.len(),
        unchanged = unchanged_posts.len(),
        "Sync complete"
    );

    Ok(TitleMatchSyncResult {
        new_posts,
        updated_posts,
        unchanged_posts,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Urgency Normalization Tests
    // =========================================================================

    #[test]
    fn test_normalize_urgency_valid_values() {
        assert_eq!(
            normalize_urgency(Some("low".to_string())),
            Some("low".to_string())
        );
        assert_eq!(
            normalize_urgency(Some("medium".to_string())),
            Some("medium".to_string())
        );
        assert_eq!(
            normalize_urgency(Some("high".to_string())),
            Some("high".to_string())
        );
        assert_eq!(
            normalize_urgency(Some("urgent".to_string())),
            Some("urgent".to_string())
        );
    }

    #[test]
    fn test_normalize_urgency_case_insensitive() {
        assert_eq!(
            normalize_urgency(Some("LOW".to_string())),
            Some("low".to_string())
        );
        assert_eq!(
            normalize_urgency(Some("High".to_string())),
            Some("high".to_string())
        );
        assert_eq!(
            normalize_urgency(Some("URGENT".to_string())),
            Some("urgent".to_string())
        );
    }

    #[test]
    fn test_normalize_urgency_invalid_values() {
        assert_eq!(normalize_urgency(Some("critical".to_string())), None);
        assert_eq!(normalize_urgency(Some("asap".to_string())), None);
        assert_eq!(normalize_urgency(Some("normal".to_string())), None);
        assert_eq!(normalize_urgency(None), None);
    }

    #[test]
    fn test_normalize_urgency_mixed_case() {
        assert_eq!(
            normalize_urgency(Some("MeDiUm".to_string())),
            Some("medium".to_string())
        );
    }

    #[test]
    fn test_normalize_urgency_empty_string() {
        // Empty string is not a valid urgency
        assert_eq!(normalize_urgency(Some("".to_string())), None);
    }

    #[test]
    fn test_normalize_urgency_whitespace() {
        // Whitespace-only is not a valid urgency
        assert_eq!(normalize_urgency(Some("  ".to_string())), None);
    }

    // =========================================================================
    // ExtractedPostInput Tests
    // =========================================================================

    #[test]
    fn test_extracted_post_input_default_fields() {
        let input = ExtractedPostInput {
            title: "Test Title".to_string(),
            description: "Test Description".to_string(),
            description_markdown: None,
            summary: None,
            contact: None,
            location: None,
            urgency: None,
            confidence: None,
            source_url: None,
            tags: HashMap::new(),
        };

        assert_eq!(input.title, "Test Title");
        assert!(input.tags.is_empty());
    }

    #[test]
    fn test_extracted_post_input_with_tags() {
        let mut tags = HashMap::new();
        tags.insert(
            "audience_role".to_string(),
            vec!["volunteer".to_string(), "donor".to_string()],
        );

        let input = ExtractedPostInput {
            title: "Volunteer Opportunity".to_string(),
            description: "Help needed".to_string(),
            description_markdown: None,
            summary: Some("Volunteer work".to_string()),
            contact: None,
            location: Some("Minneapolis".to_string()),
            urgency: Some("high".to_string()),
            confidence: Some("high".to_string()),
            source_url: Some("https://example.org/volunteer".to_string()),
            tags,
        };

        let audience_roles = input.tags.get("audience_role").unwrap();
        assert_eq!(audience_roles.len(), 2);
        assert!(audience_roles.contains(&"volunteer".to_string()));
        assert!(audience_roles.contains(&"donor".to_string()));
    }
}
