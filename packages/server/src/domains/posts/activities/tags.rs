//! Post tag actions - entry-point functions for tag operations
//!
//! These are called from Restate virtual objects.
//! Actions are self-contained: they take raw input, handle ID parsing, and return results.
//! Authorization is handled at the API layer.

use anyhow::{Context, Result};
use schemars::JsonSchema;
use serde::Deserialize;
use sqlx::PgPool;
use tracing::info;
use uuid::Uuid;

use crate::common::{PostId, TagEntry, TagId};
use crate::domains::posts::activities::create_post::tag_post_from_extracted;
use crate::domains::posts::models::Post;
use crate::domains::tag::models::tag_kind_config::build_tag_instructions;
use crate::domains::tag::{Tag, Taggable};
use crate::kernel::ServerDeps;
use crate::kernel::GPT_5_MINI;

/// Tag input for batch operations
#[derive(Debug, Clone)]
pub struct TagInput {
    pub kind: String,
    pub value: String,
}

/// Update listing tags (replaces all existing tags with new ones)
/// Returns the updated Post.
pub async fn update_post_tags(post_id: Uuid, tags: Vec<TagInput>, pool: &PgPool) -> Result<Post> {
    let post_id = PostId::from_uuid(post_id);

    info!(post_id = %post_id, tag_count = tags.len(), "Updating post tags");

    // Delete existing tags
    Taggable::delete_all_for_post(post_id, pool).await?;

    // Add new tags
    for tag_input in tags {
        let tag = Tag::find_or_create(&tag_input.kind, &tag_input.value, None, pool).await?;
        Taggable::create_post_tag(post_id, tag.id, pool).await?;
    }

    Post::find_by_id(post_id, pool)
        .await?
        .context("Post not found")
}

/// Add a single tag to a post
/// Returns the created Tag.
pub async fn add_post_tag(
    post_id: Uuid,
    tag_kind: String,
    tag_value: String,
    display_name: Option<String>,
    color: Option<String>,
    pool: &PgPool,
) -> Result<Tag> {
    let post_id = PostId::from_uuid(post_id);

    info!(post_id = %post_id, tag_kind = %tag_kind, tag_value = %tag_value, "Adding post tag");

    let mut tag = Tag::find_or_create(&tag_kind, &tag_value, display_name, pool).await?;

    if color.is_some() {
        tag = Tag::update_color(tag.id, color.as_deref(), pool).await?;
    }

    Taggable::create_post_tag(post_id, tag.id, pool).await?;

    Ok(tag)
}

/// Remove a tag from a post
/// Returns true on success.
pub async fn remove_post_tag(post_id: Uuid, tag_id: String, pool: &PgPool) -> Result<bool> {
    let post_id = PostId::from_uuid(post_id);
    let tag_id = TagId::parse(&tag_id).context("Invalid tag ID")?;

    info!(post_id = %post_id, tag_id = %tag_id, "Removing post tag");

    Taggable::delete_post_tag(post_id, tag_id, pool).await?;

    Ok(true)
}

/// AI-extracted tags for tag-only regeneration.
///
/// Uses Vec<TagEntry> instead of HashMap to be compatible with OpenAI strict mode
/// (which requires all object schemas to have named properties, not dynamic keys).
#[derive(Debug, Deserialize, JsonSchema)]
struct ExtractedTags {
    pub tags: Vec<TagEntry>,
}

/// Regenerate tags for a post using AI, without re-extracting other fields.
///
/// Loads the post's title/description, asks AI to classify it against
/// the current tag vocabulary, then clears and re-applies tags.
pub async fn regenerate_post_tags(post_id: Uuid, deps: &ServerDeps) -> Result<()> {
    let post_id_typed = PostId::from_uuid(post_id);

    let post = Post::find_by_id(post_id_typed, &deps.db_pool)
        .await?
        .context("Post not found")?;

    let tag_instructions = build_tag_instructions(&deps.db_pool).await?;
    if tag_instructions.is_empty() {
        return Ok(());
    }

    // Load existing tags so the LLM can see what was previously applied (including manual edits)
    let existing_tags = Tag::find_for_post(post_id_typed, &deps.db_pool).await?;
    let existing_section = if existing_tags.is_empty() {
        String::from("No tags currently applied.")
    } else {
        let mut lines = Vec::new();
        for tag in &existing_tags {
            lines.push(format!("  - {}: {}", tag.kind, tag.value));
        }
        format!("Currently applied tags (some may be manually entered by an admin):\n{}", lines.join("\n"))
    };

    let system_prompt = format!(
        "Classify this post into the appropriate tags.\n\
         Return a JSON object with a \"tags\" array. Each entry has \"kind\" (tag kind slug) \
         and \"values\" (array of matching tag values from the provided lists).\n\
         Only use values from the provided lists. Pick all that apply.\n\n\
         You will be shown the post's existing tags. Consider keeping manually-entered tags \
         that seem reasonable, even if you wouldn't have chosen them yourself.\n\n\
         Available tag kinds:\n{}",
        tag_instructions
    );

    let description = post
        .description_markdown
        .as_deref()
        .unwrap_or(&post.description);

    let user_prompt = format!(
        "Title: {}\n\nDescription:\n{}\n\n{}",
        post.title, description, existing_section
    );

    info!(post_id = %post_id, "Regenerating tags via AI");

    let result = deps
        .ai
        .extract::<ExtractedTags>(GPT_5_MINI, &system_prompt, &user_prompt)
        .await
        .context("AI tag extraction failed")?;

    // Normalize kind slugs to lowercase — AI sometimes capitalizes them
    let normalized: Vec<TagEntry> = result
        .tags
        .into_iter()
        .map(|e| TagEntry {
            kind: e.kind.to_lowercase(),
            values: e.values,
        })
        .collect();
    
    let tags_map = TagEntry::to_map(&normalized);

    info!(
        post_id = %post_id,
        tag_count = tags_map.values().map(|v| v.len()).sum::<usize>(),
        "AI tag extraction complete, applying tags"
    );

    Taggable::delete_all_for_post(post_id_typed, &deps.db_pool).await?;
    tag_post_from_extracted(post_id_typed, &tags_map, &deps.db_pool).await;

    Ok(())
}
