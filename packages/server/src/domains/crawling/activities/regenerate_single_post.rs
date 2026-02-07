//! Regenerate a single post from its source extraction pages.
//!
//! Loads the post's source URL(s), fetches the extraction page content,
//! re-runs the three-pass extraction, and updates the post with the best match.

use anyhow::{anyhow, Result};
use tracing::{info, warn};
use uuid::Uuid;

use crate::common::PostId;
use crate::domains::extraction::data::ExtractionPageData;
use crate::domains::posts::activities::create_post::{save_contact_info, tag_with_audience_roles};
use crate::domains::contacts::Contact;
use crate::domains::posts::models::{Post, UpdatePostContent};
use crate::domains::tag::models::{Tag, Taggable};
use crate::domains::website::models::Website;
use crate::kernel::ServerDeps;

use super::post_extraction::extract_posts_from_content;

/// Regenerate a single post by re-extracting from its source pages.
///
/// Steps:
/// 1. Load the post and its source URL(s)
/// 2. Load extraction page content for each source URL
/// 3. Run three-pass extraction on the combined content
/// 4. Find the best matching result (by title similarity)
/// 5. Update the post with fresh data
pub async fn regenerate_single_post(post_id: Uuid, deps: &ServerDeps) -> Result<()> {
    let post_id_typed = PostId::from_uuid(post_id);

    // 1. Load the post
    let post = Post::find_by_id(post_id_typed, &deps.db_pool)
        .await?
        .ok_or_else(|| anyhow!("Post not found: {}", post_id))?;

    // 2. Get source URL(s) and website domain
    let source_url = post
        .source_url
        .as_ref()
        .ok_or_else(|| anyhow!("Post has no source_url, cannot regenerate"))?;

    let website_id = post
        .website_id
        .ok_or_else(|| anyhow!("Post has no website_id, cannot regenerate"))?;

    let website = Website::find_by_id(website_id, &deps.db_pool).await?;

    info!(
        post_id = %post_id,
        source_url = %source_url,
        domain = %website.domain,
        "Regenerating single post from source pages"
    );

    // 3. Split comma-separated URLs and load each extraction page
    let urls: Vec<&str> = source_url.split(',').map(|u| u.trim()).collect();
    let mut combined_content = String::new();

    for url in &urls {
        match ExtractionPageData::find_by_url(url, &deps.db_pool).await? {
            Some(page) => {
                if !combined_content.is_empty() {
                    combined_content.push_str("\n\n---\n\n");
                }
                combined_content.push_str(&format!("## Source: {}\n\n{}", page.url, page.content));
            }
            None => {
                warn!(url = %url, "Extraction page not found for source URL");
            }
        }
    }

    if combined_content.is_empty() {
        return Err(anyhow!(
            "No extraction page content found for source URLs: {}",
            source_url
        ));
    }

    // 4. Run three-pass extraction
    let extracted = extract_posts_from_content(&combined_content, &website.domain, deps).await?;

    if extracted.is_empty() {
        return Err(anyhow!("Extraction produced no posts from source content"));
    }

    info!(
        post_id = %post_id,
        extracted_count = extracted.len(),
        "Extraction complete, finding best match"
    );

    // 5. Find best match by title word overlap similarity
    let best_match = extracted
        .iter()
        .max_by_key(|ep| title_similarity(&post.title, &ep.title))
        .unwrap(); // safe: extracted is non-empty

    let similarity = title_similarity(&post.title, &best_match.title);
    info!(
        post_id = %post_id,
        original_title = %post.title,
        matched_title = %best_match.title,
        similarity = similarity,
        "Best match found"
    );

    // 6. Update the post content
    Post::update_content(
        UpdatePostContent::builder()
            .id(post_id_typed)
            .title(Some(best_match.title.clone()))
            .description(Some(best_match.description.clone()))
            .description_markdown(None::<String>)
            .tldr(Some(best_match.tldr.clone()))
            .urgency(best_match.urgency.clone())
            .location(best_match.location.clone())
            .build(),
        &deps.db_pool,
    )
    .await?;

    // 7. Update contacts: delete existing, create new
    Contact::delete_all_for_post(post_id_typed, &deps.db_pool).await?;
    if let Some(ref contact) = best_match.contact {
        save_contact_info(post_id_typed, contact, &deps.db_pool).await;
    }

    // 8. Update audience role tags: delete existing audience_role tags, re-tag
    let existing_tags = Tag::find_for_post(post_id_typed, &deps.db_pool).await?;
    for tag in existing_tags.iter().filter(|t| t.kind == "audience_role") {
        let _ = Taggable::delete_post_tag(post_id_typed, tag.id, &deps.db_pool).await;
    }
    tag_with_audience_roles(post_id_typed, &best_match.audience_roles, &deps.db_pool).await;

    info!(post_id = %post_id, "Single post regeneration complete");

    Ok(())
}

/// Simple word-overlap similarity between two titles.
/// Returns count of shared lowercase words.
fn title_similarity(a: &str, b: &str) -> usize {
    let a_words: std::collections::HashSet<String> = a
        .to_lowercase()
        .split_whitespace()
        .map(String::from)
        .collect();
    let b_words: std::collections::HashSet<String> = b
        .to_lowercase()
        .split_whitespace()
        .map(String::from)
        .collect();
    a_words.intersection(&b_words).count()
}
