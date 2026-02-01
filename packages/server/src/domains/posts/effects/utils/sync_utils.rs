use crate::common::{PostId, WebsiteId};
use crate::domains::posts::models::{Post, PostContact, PostStatus};
use crate::domains::organization::utils::generate_tldr;
use crate::domains::tag::models::{Tag, Taggable};
use crate::kernel::BaseEmbeddingService;
use anyhow::Result;
use sqlx::PgPool;

/// Similarity threshold for considering posts as duplicates
/// 0.90 catches posts like "Food Shelf Program" vs "SuperShelf Food Shelf" (90.9% similar)
const SIMILARITY_THRESHOLD: f32 = 0.90;

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

/// Sync result showing what changed
#[derive(Debug)]
pub struct SyncResult {
    pub new_posts: Vec<PostId>,
    pub updated_posts: Vec<PostId>,
    pub unchanged_posts: Vec<PostId>,
}

/// Extracted listing input (from AI)
#[derive(Debug, Clone)]
pub struct ExtractedPostInput {
    pub organization_name: String,
    pub title: String,
    pub description: String,
    pub description_markdown: Option<String>,
    pub tldr: Option<String>,
    pub contact: Option<serde_json::Value>,
    pub location: Option<String>,
    pub urgency: Option<String>,
    pub confidence: Option<String>,
    pub source_url: Option<String>,
    /// Audience roles: who this listing is for
    /// Valid values: "recipient", "donor", "volunteer", "participant"
    pub audience_roles: Vec<String>,
}

/// Synchronize extracted posts with database
///
/// Algorithm:
/// 1. For each extracted post:
///    - Check if post exists by website + title (fast path)
///    - If no title match, check for similar post by embedding (prevents duplicates)
///    - If similar post found: update it
///    - If no match: create new post
pub async fn sync_posts(
    pool: &PgPool,
    website_id: WebsiteId,
    extracted_posts: Vec<ExtractedPostInput>,
    embedding_service: Option<&dyn BaseEmbeddingService>,
) -> Result<SyncResult> {
    tracing::info!(
        website_id = %website_id,
        post_count = extracted_posts.len(),
        "Syncing posts"
    );

    let mut new_posts = Vec::new();
    let mut updated_posts = Vec::new();
    let mut unchanged_posts = Vec::new();

    // Load existing posts with embeddings for similarity matching
    let existing_with_embeddings = Post::find_with_embeddings_for_website(website_id, pool).await?;
    tracing::info!(
        existing_with_embeddings = existing_with_embeddings.len(),
        "Loaded existing posts with embeddings for duplicate detection"
    );

    for post_input in extracted_posts {
        // First try: exact title match (fast path)
        let existing = Post::find_by_domain_and_title(website_id, &post_input.title, pool).await?;

        if let Some(existing_post) = existing {
            // Post exists by title - check if content changed
            let content_changed = existing_post.description != post_input.description
                || existing_post.tldr.as_deref() != post_input.tldr.as_deref()
                || existing_post.location != post_input.location;

            if content_changed {
                // Update existing post
                Post::update_content(
                    existing_post.id,
                    None, // title - don't change
                    Some(post_input.description.clone()),
                    None, // description_markdown
                    post_input.tldr.clone(),
                    None, // category
                    None, // urgency
                    post_input.location.clone(),
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
            // No title match - try embedding similarity if we have an embedding service
            let similar_post = if let Some(emb_service) = embedding_service {
                find_similar_post_by_embedding(
                    &post_input,
                    &existing_with_embeddings,
                    emb_service,
                )
                .await
            } else {
                None
            };

            if let Some((similar_post_id, similarity, similar_title)) = similar_post {
                // Found similar post by embedding - update it instead of creating duplicate
                tracing::info!(
                    existing_post_id = %similar_post_id,
                    similarity = %similarity,
                    existing_title = %similar_title,
                    new_title = %post_input.title,
                    "Found similar post by embedding, updating instead of creating duplicate"
                );

                Post::update_content(
                    similar_post_id,
                    Some(post_input.title.clone()), // Update title to new version
                    Some(post_input.description.clone()),
                    None, // description_markdown
                    post_input.tldr.clone(),
                    None, // category
                    None, // urgency
                    post_input.location.clone(),
                    pool,
                )
                .await?;

                updated_posts.push(similar_post_id);
            } else {
                // No similar post found - create new
                let tldr = post_input
                    .tldr
                    .clone()
                    .or_else(|| Some(generate_tldr(&post_input.description, 100)));

                let urgency = normalize_urgency(post_input.urgency.clone());

                match Post::create(
                    post_input.organization_name.clone(),
                    post_input.title.clone(),
                    post_input.description.clone(),
                    tldr,
                    "opportunity".to_string(),
                    "general".to_string(),
                    Some("accepting".to_string()),
                    urgency,
                    post_input.location.clone(),
                    PostStatus::PendingApproval.to_string(),
                    "en".to_string(),
                    Some("scraped".to_string()),
                    None, // submitted_by_admin_id
                    Some(website_id),
                    post_input.source_url.clone(),
                    None, // organization_id
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

                        // Save contact info if present
                        if let Some(ref contact_info) = post_input.contact {
                            if let Err(e) =
                                PostContact::create_from_json(created.id, contact_info, pool).await
                            {
                                tracing::warn!(
                                    post_id = %created.id,
                                    error = %e,
                                    "Failed to save contact info"
                                );
                            }
                        }

                        // Tag post with audience roles
                        for role in &post_input.audience_roles {
                            let normalized_role = role.to_lowercase();
                            if let Ok(tag) =
                                Tag::find_by_kind_value("audience_role", &normalized_role, pool)
                                    .await
                            {
                                if let Some(tag) = tag {
                                    if let Err(e) =
                                        Taggable::create_post_tag(created.id, tag.id, pool).await
                                    {
                                        tracing::warn!(
                                            post_id = %created.id,
                                            role = %normalized_role,
                                            error = %e,
                                            "Failed to tag post with audience role"
                                        );
                                    }
                                } else {
                                    tracing::warn!(
                                        role = %normalized_role,
                                        "Unknown audience role from AI"
                                    );
                                }
                            }
                        }

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
    }

    tracing::info!(
        new = new_posts.len(),
        updated = updated_posts.len(),
        unchanged = unchanged_posts.len(),
        "Sync complete"
    );

    Ok(SyncResult {
        new_posts,
        updated_posts,
        unchanged_posts,
    })
}

/// Find a similar post by embedding similarity
async fn find_similar_post_by_embedding(
    post_input: &ExtractedPostInput,
    existing_posts: &[(PostId, Vec<f32>, String)],
    embedding_service: &dyn BaseEmbeddingService,
) -> Option<(PostId, f32, String)> {
    if existing_posts.is_empty() {
        return None;
    }

    // Generate embedding for the new post content
    let content_for_embedding = format!("{}\n\n{}", post_input.title, post_input.description);
    let new_embedding = match embedding_service.generate(&content_for_embedding).await {
        Ok(emb) => emb,
        Err(e) => {
            tracing::warn!(
                error = %e,
                title = %post_input.title,
                "Failed to generate embedding for similarity check, skipping duplicate detection"
            );
            return None;
        }
    };

    // Find the most similar existing post
    let mut best_match: Option<(PostId, f32, String)> = None;

    for (post_id, existing_embedding, title) in existing_posts {
        let similarity = cosine_similarity(&new_embedding, existing_embedding);

        if similarity >= SIMILARITY_THRESHOLD {
            if best_match.is_none() || similarity > best_match.as_ref().unwrap().1 {
                best_match = Some((*post_id, similarity, title.clone()));
            }
        }
    }

    best_match
}

/// Calculate cosine similarity between two vectors
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let magnitude_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let magnitude_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if magnitude_a == 0.0 || magnitude_b == 0.0 {
        return 0.0;
    }

    dot_product / (magnitude_a * magnitude_b)
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
    // Cosine Similarity Tests
    // =========================================================================

    #[test]
    fn test_cosine_similarity_identical_vectors() {
        let v1 = vec![1.0, 0.0, 0.0];
        let v2 = vec![1.0, 0.0, 0.0];
        let similarity = cosine_similarity(&v1, &v2);
        assert!((similarity - 1.0).abs() < 0.0001, "Identical vectors should have similarity 1.0");
    }

    #[test]
    fn test_cosine_similarity_orthogonal_vectors() {
        let v1 = vec![1.0, 0.0, 0.0];
        let v2 = vec![0.0, 1.0, 0.0];
        let similarity = cosine_similarity(&v1, &v2);
        assert!(similarity.abs() < 0.0001, "Orthogonal vectors should have similarity 0.0");
    }

    #[test]
    fn test_cosine_similarity_opposite_vectors() {
        let v1 = vec![1.0, 0.0, 0.0];
        let v2 = vec![-1.0, 0.0, 0.0];
        let similarity = cosine_similarity(&v1, &v2);
        assert!((similarity + 1.0).abs() < 0.0001, "Opposite vectors should have similarity -1.0");
    }

    #[test]
    fn test_cosine_similarity_empty_vectors() {
        let v1: Vec<f32> = vec![];
        let v2: Vec<f32> = vec![];
        let similarity = cosine_similarity(&v1, &v2);
        assert_eq!(similarity, 0.0, "Empty vectors should return 0.0");
    }

    #[test]
    fn test_cosine_similarity_mismatched_lengths() {
        let v1 = vec![1.0, 2.0, 3.0];
        let v2 = vec![1.0, 2.0];
        let similarity = cosine_similarity(&v1, &v2);
        assert_eq!(similarity, 0.0, "Mismatched lengths should return 0.0");
    }

    #[test]
    fn test_cosine_similarity_zero_magnitude_first() {
        let v1 = vec![0.0, 0.0, 0.0];
        let v2 = vec![1.0, 2.0, 3.0];
        let similarity = cosine_similarity(&v1, &v2);
        assert_eq!(similarity, 0.0, "Zero magnitude vector should return 0.0");
    }

    #[test]
    fn test_cosine_similarity_zero_magnitude_second() {
        let v1 = vec![1.0, 2.0, 3.0];
        let v2 = vec![0.0, 0.0, 0.0];
        let similarity = cosine_similarity(&v1, &v2);
        assert_eq!(similarity, 0.0, "Zero magnitude vector should return 0.0");
    }

    #[test]
    fn test_cosine_similarity_zero_magnitude_both() {
        let v1 = vec![0.0, 0.0, 0.0];
        let v2 = vec![0.0, 0.0, 0.0];
        let similarity = cosine_similarity(&v1, &v2);
        assert_eq!(similarity, 0.0, "Both zero magnitude vectors should return 0.0");
    }

    #[test]
    fn test_cosine_similarity_high_dimensional() {
        // Test with 1536 dimensions (like OpenAI embeddings)
        let v1: Vec<f32> = (0..1536).map(|i| (i as f32).sin()).collect();
        let v2: Vec<f32> = (0..1536).map(|i| (i as f32).sin()).collect();
        let similarity = cosine_similarity(&v1, &v2);
        assert!((similarity - 1.0).abs() < 0.0001, "Identical high-dimensional vectors should have similarity ~1.0");
    }

    #[test]
    fn test_cosine_similarity_similar_but_not_identical() {
        // Two vectors that are similar but not identical
        let v1 = vec![0.9, 0.1, 0.05];
        let v2 = vec![0.85, 0.15, 0.05];
        let similarity = cosine_similarity(&v1, &v2);
        assert!(similarity > 0.9, "Similar vectors should have high similarity: {}", similarity);
        assert!(similarity < 1.0, "Non-identical vectors should be < 1.0: {}", similarity);
    }

    #[test]
    fn test_cosine_similarity_threshold_boundary() {
        // Test at the exact threshold (0.90)
        // Create vectors that are exactly at the threshold
        let v1 = vec![1.0, 0.0];
        let v2 = vec![0.9, 0.436]; // cos(26°) ≈ 0.898, just under threshold
        let similarity = cosine_similarity(&v1, &v2);

        // This tests our threshold logic indirectly
        assert!(
            (similarity - 0.90).abs() < 0.05,
            "Boundary test: similarity should be near 0.90, got {}",
            similarity
        );
    }

    // =========================================================================
    // ExtractedPostInput Tests
    // =========================================================================

    #[test]
    fn test_extracted_post_input_default_fields() {
        let input = ExtractedPostInput {
            organization_name: "Test Org".to_string(),
            title: "Test Title".to_string(),
            description: "Test Description".to_string(),
            description_markdown: None,
            tldr: None,
            contact: None,
            location: None,
            urgency: None,
            confidence: None,
            source_url: None,
            audience_roles: vec![],
        };

        assert_eq!(input.organization_name, "Test Org");
        assert_eq!(input.title, "Test Title");
        assert!(input.audience_roles.is_empty());
    }

    #[test]
    fn test_extracted_post_input_with_audience_roles() {
        let input = ExtractedPostInput {
            organization_name: "Test Org".to_string(),
            title: "Volunteer Opportunity".to_string(),
            description: "Help needed".to_string(),
            description_markdown: None,
            tldr: Some("Volunteer work".to_string()),
            contact: None,
            location: Some("Minneapolis".to_string()),
            urgency: Some("high".to_string()),
            confidence: Some("high".to_string()),
            source_url: Some("https://example.org/volunteer".to_string()),
            audience_roles: vec!["volunteer".to_string(), "donor".to_string()],
        };

        assert_eq!(input.audience_roles.len(), 2);
        assert!(input.audience_roles.contains(&"volunteer".to_string()));
        assert!(input.audience_roles.contains(&"donor".to_string()));
    }
}
