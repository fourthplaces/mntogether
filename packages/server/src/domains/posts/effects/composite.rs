//! Post composite effect - routes events to appropriate sub-effects
//!
//! This effect is a thin orchestration layer that dispatches request events to handlers.
//! Following CLAUDE.md: Effects must be thin orchestration layers, business logic in actions.

use anyhow::Result;
use async_trait::async_trait;
use seesaw_core::{Effect, EffectContext};

use crate::kernel::ServerDeps;
use super::{AIEffect, PostEffect, ScraperEffect, SyncEffect};
use crate::domains::posts::events::PostEvent;

/// Composite Effect - Routes PostEvent to appropriate sub-effect
///
/// This composite effect solves the problem of having multiple effects for the same event type.
/// The dispatcher requires one effect per event type, so this effect routes based on the event variant.
///
/// NOTE: Crawling events have been moved to the `crawling` domain.
/// See `crate::domains::crawling::effects::CrawlerEffect`.
pub struct PostCompositeEffect {
    scraper: ScraperEffect,
    ai: AIEffect,
    sync: SyncEffect,
    listing: PostEffect,
}

impl PostCompositeEffect {
    pub fn new() -> Self {
        Self {
            scraper: ScraperEffect,
            ai: AIEffect,
            sync: SyncEffect,
            listing: PostEffect,
        }
    }
}

impl Default for PostCompositeEffect {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Effect<PostEvent, ServerDeps> for PostCompositeEffect {
    type Event = PostEvent;

    async fn handle(
        &mut self,
        event: PostEvent,
        ctx: EffectContext<ServerDeps>,
    ) -> Result<Option<PostEvent>> {
        match &event {
            // =================================================================
            // Route to ScraperEffect
            // =================================================================
            PostEvent::ScrapeSourceRequested { .. }
            | PostEvent::ScrapeResourceLinkRequested { .. } => {
                self.scraper.handle(event, ctx).await
            }

            // =================================================================
            // Route to AIEffect
            // =================================================================
            PostEvent::ExtractPostsRequested { .. }
            | PostEvent::ExtractPostsFromResourceLinkRequested { .. } => {
                self.ai.handle(event, ctx).await
            }

            // =================================================================
            // Route to SyncEffect
            // =================================================================
            PostEvent::SyncPostsRequested { .. } => self.sync.handle(event, ctx).await,

            // =================================================================
            // Route to PostEffect (all other request events)
            // =================================================================
            PostEvent::CreateWebsiteFromLinkRequested { .. }
            | PostEvent::CreatePostEntryRequested { .. }
            | PostEvent::CreatePostsFromResourceLinkRequested { .. }
            | PostEvent::UpdatePostStatusRequested { .. }
            | PostEvent::EditAndApproveListingRequested { .. }
            | PostEvent::CreatePostRequested { .. }
            | PostEvent::GeneratePostEmbeddingRequested { .. }
            | PostEvent::CreateCustomPostRequested { .. }
            | PostEvent::RepostPostRequested { .. }
            | PostEvent::ExpirePostRequested { .. }
            | PostEvent::ArchivePostRequested { .. }
            | PostEvent::PostViewedRequested { .. }
            | PostEvent::PostClickedRequested { .. }
            | PostEvent::DeletePostRequested { .. }
            | PostEvent::ReportListingRequested { .. }
            | PostEvent::ResolveReportRequested { .. }
            | PostEvent::DismissReportRequested { .. }
            | PostEvent::DeduplicatePostsRequested { .. }
            | PostEvent::SubmitListingRequested { .. }
            | PostEvent::SubmitResourceLinkRequested { .. }
            | PostEvent::ApproveListingRequested { .. }
            | PostEvent::RejectListingRequested { .. } => {
                self.listing.handle(event, ctx).await
            }

            // =================================================================
            // Fact Events → Terminal, no follow-up needed
            // =================================================================
            PostEvent::SourceScraped { .. }
            | PostEvent::ResourceLinkScraped { .. }
            | PostEvent::PostsExtracted { .. }
            | PostEvent::ResourceLinkPostsExtracted { .. }
            | PostEvent::PostsSynced { .. }
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
            | PostEvent::WebsiteCreatedFromLink { .. }
            | PostEvent::WebsitePendingApproval { .. } => Ok(None),
        }
    }
}
