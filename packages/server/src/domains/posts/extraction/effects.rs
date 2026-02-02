//! Post extraction effects
//!
//! Handles AI extraction of posts from crawled page content and syncing to database.
//! This effect is a thin orchestration layer that dispatches to handler functions.

use anyhow::Result;
use async_trait::async_trait;
use seesaw_core::{Effect, EffectContext};
use tracing::{info, warn};

use crate::common::{ExtractedPost, JobId, WebsiteId};
use crate::domains::chatrooms::ChatRequestState;
use crate::domains::crawling::effects::extraction::{
    hash_content, summarize_pages, synthesize_posts, PageToSummarize, SynthesisInput,
};
use crate::domains::crawling::models::PageSnapshot;
use crate::domains::posts::effects::syncing::sync_extracted_posts;
use crate::domains::posts::extraction::commands::PostExtractionCommand;
use crate::domains::posts::extraction::events::PostExtractionEvent;
use crate::domains::website::models::Website;
use crate::kernel::ServerDeps;

/// Post extraction effect
///
/// Handles extraction commands and emits PostExtractionEvent
pub struct PostExtractionEffect;

#[async_trait]
impl Effect<PostExtractionCommand, ServerDeps, ChatRequestState> for PostExtractionEffect {
    type Event = PostExtractionEvent;

    async fn execute(
        &self,
        cmd: PostExtractionCommand,
        ctx: EffectContext<ServerDeps, ChatRequestState>,
    ) -> Result<PostExtractionEvent> {
        match cmd {
            PostExtractionCommand::ExtractPostsFromPages {
                website_id,
                job_id,
                page_snapshot_ids,
            } => {
                handle_extract_posts_from_pages(website_id, job_id, page_snapshot_ids, &ctx).await
            }
        }
    }
}

// ============================================================================
// Handler Functions
// ============================================================================

async fn handle_extract_posts_from_pages(
    website_id: WebsiteId,
    job_id: JobId,
    page_snapshot_ids: Vec<uuid::Uuid>,
    ctx: &EffectContext<ServerDeps, ChatRequestState>,
) -> Result<PostExtractionEvent> {
    info!(
        website_id = %website_id,
        job_id = %job_id,
        pages_count = page_snapshot_ids.len(),
        "Starting post extraction from pages"
    );

    // Get website domain for synthesis
    let website = match Website::find_by_id(website_id, &ctx.deps().db_pool).await {
        Ok(w) => w,
        Err(e) => {
            warn!(website_id = %website_id, error = %e, "Failed to find website for extraction");
            return Ok(PostExtractionEvent::ExtractionFailed {
                website_id,
                job_id,
                reason: format!("Website not found: {}", e),
            });
        }
    };

    // Fetch page snapshots
    let mut pages_to_summarize = Vec::new();
    for snapshot_id in &page_snapshot_ids {
        match PageSnapshot::find_by_id(&ctx.deps().db_pool, *snapshot_id).await {
            Ok(snapshot) => {
                // Get content - prefer markdown, fall back to HTML
                let raw_content = snapshot
                    .markdown
                    .clone()
                    .unwrap_or_else(|| snapshot.html.clone());
                let content_hash = hash_content(&raw_content);

                pages_to_summarize.push(PageToSummarize {
                    snapshot_id: snapshot.id,
                    url: snapshot.url.clone(),
                    raw_content,
                    content_hash,
                });
            }
            Err(e) => {
                warn!(
                    snapshot_id = %snapshot_id,
                    error = %e,
                    "Failed to fetch page snapshot, skipping"
                );
            }
        }
    }

    if pages_to_summarize.is_empty() {
        warn!(website_id = %website_id, "No valid page snapshots found for extraction");
        return Ok(PostExtractionEvent::ExtractionFailed {
            website_id,
            job_id,
            reason: "No valid page snapshots found".to_string(),
        });
    }

    let pages_count = pages_to_summarize.len();
    info!(
        website_id = %website_id,
        pages_count = pages_count,
        "Summarizing pages"
    );

    // Pass 1: Summarize pages
    let summarized_pages = match summarize_pages(
        pages_to_summarize,
        ctx.deps().ai.as_ref(),
        &ctx.deps().db_pool,
    )
    .await
    {
        Ok(pages) => pages,
        Err(e) => {
            warn!(website_id = %website_id, error = %e, "Failed to summarize pages");
            return Ok(PostExtractionEvent::ExtractionFailed {
                website_id,
                job_id,
                reason: format!("Page summarization failed: {}", e),
            });
        }
    };

    info!(
        website_id = %website_id,
        summarized_count = summarized_pages.len(),
        "Pages summarized, synthesizing posts"
    );

    // Pass 2: Synthesize posts from summaries
    let synthesis_input = SynthesisInput {
        website_domain: website.domain.clone(),
        pages: summarized_pages,
    };

    let extracted_posts = match synthesize_posts(synthesis_input, ctx.deps().ai.as_ref()).await {
        Ok(posts) => posts,
        Err(e) => {
            warn!(website_id = %website_id, error = %e, "Failed to synthesize posts");
            return Ok(PostExtractionEvent::ExtractionFailed {
                website_id,
                job_id,
                reason: format!("Post synthesis failed: {}", e),
            });
        }
    };

    // Convert extraction types to common ExtractedPost type
    let posts: Vec<ExtractedPost> = extracted_posts
        .into_iter()
        .map(|p| ExtractedPost {
            title: p.title,
            tldr: p.tldr,
            description: p.description,
            contact: p.contact.map(|c| crate::common::ContactInfo {
                phone: c.phone,
                email: c.email,
                website: c.website,
            }),
            location: p.location,
            urgency: None,
            confidence: None,
            audience_roles: p
                .primary_audience
                .map(|a| vec![a])
                .unwrap_or_default(),
        })
        .collect();

    let posts_extracted = posts.len();

    // Handle case where no posts were found
    if posts.is_empty() {
        info!(
            website_id = %website_id,
            pages_processed = pages_count,
            "No posts found in crawled pages"
        );

        // Update website status to indicate no posts found
        let _ = Website::complete_crawl(
            website_id,
            "no_posts_found",
            pages_count as i32,
            &ctx.deps().db_pool,
        )
        .await;

        return Ok(PostExtractionEvent::NoPostsFound {
            website_id,
            job_id,
            pages_processed: pages_count,
        });
    }

    info!(
        website_id = %website_id,
        posts_count = posts_extracted,
        "Post extraction completed, syncing to database"
    );

    // Sync posts to database
    let sync_result = match sync_extracted_posts(website_id, posts, &ctx.deps().db_pool).await {
        Ok(result) => result,
        Err(e) => {
            warn!(website_id = %website_id, error = %e, "Failed to sync posts");
            return Ok(PostExtractionEvent::ExtractionFailed {
                website_id,
                job_id,
                reason: format!("Post sync failed: {}", e),
            });
        }
    };

    // Update website status to completed
    let _ = Website::complete_crawl(
        website_id,
        "completed",
        pages_count as i32,
        &ctx.deps().db_pool,
    )
    .await;

    info!(
        website_id = %website_id,
        job_id = %job_id,
        posts_extracted = posts_extracted,
        posts_created = sync_result.new_count,
        posts_updated = sync_result.updated_count,
        pages_processed = pages_count,
        "Post extraction and sync completed"
    );

    Ok(PostExtractionEvent::PostsExtractedAndSynced {
        website_id,
        job_id,
        pages_processed: pages_count,
        posts_extracted,
        posts_created: sync_result.new_count,
        posts_updated: sync_result.updated_count,
    })
}
