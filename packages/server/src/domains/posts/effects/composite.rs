use anyhow::Result;
use async_trait::async_trait;
use seesaw_core::{Effect, EffectContext};

use super::deps::ServerDeps;
use super::{AIEffect, CrawlerEffect, PostEffect, ScraperEffect, SyncEffect};
use crate::domains::posts::commands::PostCommand;
use crate::domains::posts::events::PostEvent;

/// Composite Effect - Routes PostCommand to appropriate sub-effect
///
/// This composite effect solves the problem of having multiple effects for the same command type.
/// The dispatcher requires one effect per command type, so this effect routes based on the command variant.
pub struct PostCompositeEffect {
    scraper: ScraperEffect,
    crawler: CrawlerEffect,
    ai: AIEffect,
    sync: SyncEffect,
    listing: PostEffect,
}

impl PostCompositeEffect {
    pub fn new() -> Self {
        Self {
            scraper: ScraperEffect,
            crawler: CrawlerEffect,
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
            PostCommand::ExtractListings { .. } => self.ai.execute(cmd, ctx).await,
            PostCommand::ExtractListingsFromResourceLink { .. } => {
                self.ai.execute(cmd, ctx).await
            }

            // Route to SyncEffect
            PostCommand::SyncListings { .. } => self.sync.execute(cmd, ctx).await,

            // Route to CrawlerEffect (multi-page crawling commands)
            PostCommand::CrawlWebsite { .. }
            | PostCommand::ExtractListingsFromPages { .. }
            | PostCommand::RetryWebsiteCrawl { .. }
            | PostCommand::MarkWebsiteNoListings { .. }
            | PostCommand::SyncCrawledListings { .. }
            | PostCommand::RegeneratePosts { .. }
            | PostCommand::RegeneratePageSummaries { .. }
            | PostCommand::RegeneratePageSummary { .. }
            | PostCommand::RegeneratePagePosts { .. } => self.crawler.execute(cmd, ctx).await,

            // Route to PostEffect (all other commands)
            PostCommand::CreateWebsiteFromLink { .. }
            | PostCommand::CreateListing { .. }
            | PostCommand::CreateListingsFromResourceLink { .. }
            | PostCommand::UpdatePostStatus { .. }
            | PostCommand::UpdateListingAndApprove { .. }
            | PostCommand::CreatePost { .. }
            | PostCommand::GenerateListingEmbedding { .. }
            | PostCommand::CreateCustomPost { .. }
            | PostCommand::RepostListing { .. }
            | PostCommand::ExpirePost { .. }
            | PostCommand::ArchivePost { .. }
            | PostCommand::IncrementPostView { .. }
            | PostCommand::IncrementPostClick { .. }
            | PostCommand::DeleteListing { .. }
            | PostCommand::CreateReport { .. }
            | PostCommand::ResolveReport { .. }
            | PostCommand::DismissReport { .. } => self.listing.execute(cmd, ctx).await,
        }
    }
}
