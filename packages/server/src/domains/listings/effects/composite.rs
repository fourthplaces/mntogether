use anyhow::Result;
use async_trait::async_trait;
use seesaw_core::{Effect, EffectContext};

use super::deps::ServerDeps;
use super::{AIEffect, CrawlerEffect, ListingEffect, ScraperEffect, SyncEffect};
use crate::domains::listings::commands::ListingCommand;
use crate::domains::listings::events::ListingEvent;

/// Composite Effect - Routes ListingCommand to appropriate sub-effect
///
/// This composite effect solves the problem of having multiple effects for the same command type.
/// The dispatcher requires one effect per command type, so this effect routes based on the command variant.
pub struct ListingCompositeEffect {
    scraper: ScraperEffect,
    crawler: CrawlerEffect,
    ai: AIEffect,
    sync: SyncEffect,
    listing: ListingEffect,
}

impl ListingCompositeEffect {
    pub fn new() -> Self {
        Self {
            scraper: ScraperEffect,
            crawler: CrawlerEffect,
            ai: AIEffect,
            sync: SyncEffect,
            listing: ListingEffect,
        }
    }
}

impl Default for ListingCompositeEffect {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Effect<ListingCommand, ServerDeps> for ListingCompositeEffect {
    type Event = ListingEvent;

    async fn execute(
        &self,
        cmd: ListingCommand,
        ctx: EffectContext<ServerDeps>,
    ) -> Result<ListingEvent> {
        match &cmd {
            // Route to ScraperEffect
            ListingCommand::ScrapeSource { .. } => self.scraper.execute(cmd, ctx).await,
            ListingCommand::ScrapeResourceLink { .. } => self.scraper.execute(cmd, ctx).await,

            // Route to AIEffect
            ListingCommand::ExtractListings { .. } => self.ai.execute(cmd, ctx).await,
            ListingCommand::ExtractListingsFromResourceLink { .. } => {
                self.ai.execute(cmd, ctx).await
            }

            // Route to SyncEffect
            ListingCommand::SyncListings { .. } => self.sync.execute(cmd, ctx).await,

            // Route to CrawlerEffect (multi-page crawling commands)
            ListingCommand::CrawlWebsite { .. }
            | ListingCommand::ExtractListingsFromPages { .. }
            | ListingCommand::RetryWebsiteCrawl { .. }
            | ListingCommand::MarkWebsiteNoListings { .. }
            | ListingCommand::SyncCrawledListings { .. }
            | ListingCommand::RegeneratePosts { .. }
            | ListingCommand::RegeneratePageSummaries { .. }
            | ListingCommand::RegeneratePageSummary { .. }
            | ListingCommand::RegeneratePagePosts { .. } => self.crawler.execute(cmd, ctx).await,

            // Route to ListingEffect (all other commands)
            ListingCommand::CreateWebsiteFromLink { .. }
            | ListingCommand::CreateListing { .. }
            | ListingCommand::CreateListingsFromResourceLink { .. }
            | ListingCommand::UpdateListingStatus { .. }
            | ListingCommand::UpdateListingAndApprove { .. }
            | ListingCommand::CreatePost { .. }
            | ListingCommand::GenerateListingEmbedding { .. }
            | ListingCommand::CreateCustomPost { .. }
            | ListingCommand::RepostListing { .. }
            | ListingCommand::ExpirePost { .. }
            | ListingCommand::ArchivePost { .. }
            | ListingCommand::IncrementPostView { .. }
            | ListingCommand::IncrementPostClick { .. }
            | ListingCommand::DeleteListing { .. }
            | ListingCommand::CreateReport { .. }
            | ListingCommand::ResolveReport { .. }
            | ListingCommand::DismissReport { .. } => self.listing.execute(cmd, ctx).await,
        }
    }
}
