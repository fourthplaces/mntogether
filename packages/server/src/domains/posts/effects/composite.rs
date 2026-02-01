use anyhow::Result;
use async_trait::async_trait;
use seesaw_core::{Effect, EffectContext};

use crate::kernel::ServerDeps;
use super::{AIEffect, PostEffect, ScraperEffect, SyncEffect};
use crate::domains::posts::commands::PostCommand;
use crate::domains::posts::events::PostEvent;

/// Composite Effect - Routes PostCommand to appropriate sub-effect
///
/// This composite effect solves the problem of having multiple effects for the same command type.
/// The dispatcher requires one effect per command type, so this effect routes based on the command variant.
///
/// NOTE: Crawling commands have been moved to the `crawling` domain.
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
impl Effect<PostCommand, ServerDeps> for PostCompositeEffect {
    type Event = PostEvent;

    async fn execute(
        &self,
        cmd: PostCommand,
        ctx: EffectContext<ServerDeps>,
    ) -> Result<PostEvent> {
        match &cmd {
            // Route to ScraperEffect
            PostCommand::ScrapeSource { .. } => self.scraper.execute(cmd, ctx).await,
            PostCommand::ScrapeResourceLink { .. } => self.scraper.execute(cmd, ctx).await,

            // Route to AIEffect
            PostCommand::ExtractPosts { .. } => self.ai.execute(cmd, ctx).await,
            PostCommand::ExtractPostsFromResourceLink { .. } => {
                self.ai.execute(cmd, ctx).await
            }

            // Route to SyncEffect
            PostCommand::SyncPosts { .. } => self.sync.execute(cmd, ctx).await,

            // Route to PostEffect (all other commands)
            PostCommand::CreateWebsiteFromLink { .. }
            | PostCommand::CreatePostEntry { .. }
            | PostCommand::CreatePostsFromResourceLink { .. }
            | PostCommand::UpdatePostStatus { .. }
            | PostCommand::UpdatePostAndApprove { .. }
            | PostCommand::CreatePost { .. }
            | PostCommand::GeneratePostEmbedding { .. }
            | PostCommand::CreateCustomPost { .. }
            | PostCommand::RepostPost { .. }
            | PostCommand::ExpirePost { .. }
            | PostCommand::ArchivePost { .. }
            | PostCommand::IncrementPostView { .. }
            | PostCommand::IncrementPostClick { .. }
            | PostCommand::DeletePost { .. }
            | PostCommand::CreateReport { .. }
            | PostCommand::ResolveReport { .. }
            | PostCommand::DismissReport { .. }
            | PostCommand::DeduplicatePosts { .. } => self.listing.execute(cmd, ctx).await,
        }
    }
}
