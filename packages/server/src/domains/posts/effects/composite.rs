//! Post composite effect - watches FACT events and calls cascade handlers
//!
//! Architecture (seesaw 0.6.0 direct-call pattern):
//!   GraphQL → process(action) → emit(FactEvent) → Effect watches facts → calls handlers
//!
//! NO *Requested events. Effects watch FACT events and cascade directly.
//!
//! Cascade flows:
//!   SourceScraped → handle_extract_posts → PostsExtracted → handle_sync_posts → PostsSynced
//!   ResourceLinkScraped → handle_extract_from_resource_link → ResourceLinkPostsExtracted → handle_create_from_resource_link
//!   WebsiteCreatedFromLink → handle_scrape_resource_link → ResourceLinkScraped → ...

use seesaw_core::effect;
use std::sync::Arc;

use crate::common::AppState;
use crate::domains::posts::events::PostEvent;
use crate::kernel::ServerDeps;

use super::ai::{handle_extract_posts, handle_extract_posts_from_resource_link};
use super::post::handle_create_posts_from_resource_link;
use super::scraper::handle_scrape_resource_link;
use super::sync::handle_sync_posts;

/// Build the post composite effect handler using the 0.6.0 builder pattern.
///
/// This composite effect watches FACT events and calls cascade handlers.
/// Entry-point handlers are called directly from GraphQL via process().
pub fn post_composite_effect() -> seesaw_core::effect::Effect<AppState, ServerDeps> {
    effect::on::<PostEvent>().run(|event: Arc<PostEvent>, ctx| async move {
        match event.as_ref() {
            // =================================================================
            // Cascade: SourceScraped → extract posts
            // =================================================================
            PostEvent::SourceScraped {
                source_id,
                job_id,
                organization_name,
                content,
                ..
            } => {
                handle_extract_posts(
                    *source_id,
                    *job_id,
                    organization_name.clone(),
                    content.clone(),
                    &ctx,
                )
                .await
            }

            // =================================================================
            // Cascade: ResourceLinkScraped → extract posts from resource link
            // =================================================================
            PostEvent::ResourceLinkScraped {
                job_id,
                url,
                content,
                context,
                submitter_contact,
                ..
            } => {
                handle_extract_posts_from_resource_link(
                    *job_id,
                    url.clone(),
                    content.clone(),
                    context.clone(),
                    submitter_contact.clone(),
                    &ctx,
                )
                .await
            }

            // =================================================================
            // Cascade: PostsExtracted → sync posts to database
            // =================================================================
            PostEvent::PostsExtracted {
                source_id,
                job_id,
                posts,
            } => handle_sync_posts(*source_id, *job_id, posts.clone(), &ctx).await,

            // =================================================================
            // Cascade: ResourceLinkPostsExtracted → create posts from resource link
            // =================================================================
            PostEvent::ResourceLinkPostsExtracted {
                job_id,
                url,
                posts,
                context,
                submitter_contact,
            } => {
                handle_create_posts_from_resource_link(
                    *job_id,
                    url.clone(),
                    posts.clone(),
                    context.clone(),
                    submitter_contact.clone(),
                    &ctx,
                )
                .await
            }

            // =================================================================
            // Cascade: WebsiteCreatedFromLink → scrape resource link
            // =================================================================
            PostEvent::WebsiteCreatedFromLink {
                job_id,
                url,
                submitter_contact,
                ..
            } => {
                handle_scrape_resource_link(
                    *job_id,
                    url.clone(),
                    None, // context is not available in this event
                    submitter_contact.clone(),
                    &ctx,
                )
                .await
            }

            // =================================================================
            // Terminal events - no cascade needed
            // =================================================================
            PostEvent::PostsSynced { .. }
            | PostEvent::ScrapeFailed { .. }
            | PostEvent::ResourceLinkScrapeFailed { .. }
            | PostEvent::ExtractFailed { .. }
            | PostEvent::SyncFailed { .. }
            | PostEvent::PostEntryCreated { .. }
            | PostEvent::PostApproved { .. }
            | PostEvent::PostRejected { .. }
            | PostEvent::ListingUpdated { .. }
            | PostEvent::PostCreated { .. }
            | PostEvent::PostExpired { .. }
            | PostEvent::PostArchived { .. }
            | PostEvent::PostViewed { .. }
            | PostEvent::PostClicked { .. }
            | PostEvent::PostDeleted { .. }
            | PostEvent::PostReported { .. }
            | PostEvent::ReportResolved { .. }
            | PostEvent::ReportDismissed { .. }
            | PostEvent::PostEmbeddingGenerated { .. }
            | PostEvent::ListingEmbeddingFailed { .. }
            | PostEvent::AuthorizationDenied { .. }
            | PostEvent::PostsDeduplicated { .. }
            | PostEvent::DeduplicationFailed { .. }
            | PostEvent::WebsitePendingApproval { .. } => Ok(()),
        }
    })
}
