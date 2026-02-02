//! Post tag actions - entry-point functions for tag operations
//!
//! These are called directly from GraphQL mutations via `process()`.
//! Actions are self-contained: they take raw input, handle ID parsing, and return results.

use anyhow::{Context, Result};
use seesaw_core::EffectContext;
use tracing::info;
use uuid::Uuid;

use crate::common::{AppState, PostId, TagId};
use crate::domains::posts::models::Post;
use crate::domains::tag::{Tag, Taggable};
use crate::kernel::ServerDeps;

/// Tag input for batch operations
#[derive(Debug, Clone)]
pub struct TagInput {
    pub kind: String,
    pub value: String,
}

/// Update listing tags (replaces all existing tags with new ones)
/// Returns the updated Post.
pub async fn update_post_tags(
    post_id: Uuid,
    tags: Vec<TagInput>,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<Post> {
    let post_id = PostId::from_uuid(post_id);

    info!(post_id = %post_id, tag_count = tags.len(), "Updating post tags");

    // Delete existing tags
    Taggable::delete_all_for_post(post_id, &ctx.deps().db_pool).await?;

    // Add new tags
    for tag_input in tags {
        let tag =
            Tag::find_or_create(&tag_input.kind, &tag_input.value, None, &ctx.deps().db_pool)
                .await?;
        Taggable::create_post_tag(post_id, tag.id, &ctx.deps().db_pool).await?;
    }

    Post::find_by_id(post_id, &ctx.deps().db_pool)
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
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<Tag> {
    let post_id = PostId::from_uuid(post_id);

    info!(post_id = %post_id, tag_kind = %tag_kind, tag_value = %tag_value, "Adding post tag");

    let tag =
        Tag::find_or_create(&tag_kind, &tag_value, display_name, &ctx.deps().db_pool).await?;
    Taggable::create_post_tag(post_id, tag.id, &ctx.deps().db_pool).await?;

    Ok(tag)
}

/// Remove a tag from a post
/// Returns true on success.
pub async fn remove_post_tag(
    post_id: Uuid,
    tag_id: String,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<bool> {
    let post_id = PostId::from_uuid(post_id);
    let tag_id = TagId::parse(&tag_id).context("Invalid tag ID")?;

    info!(post_id = %post_id, tag_id = %tag_id, "Removing post tag");

    Taggable::delete_post_tag(post_id, tag_id, &ctx.deps().db_pool).await?;

    Ok(true)
}
