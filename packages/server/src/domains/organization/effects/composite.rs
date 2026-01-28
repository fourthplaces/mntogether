use anyhow::Result;
use async_trait::async_trait;
use seesaw::{Effect, EffectContext};

use super::deps::ServerDeps;
use super::{AIEffect, NeedEffect, ScraperEffect, SyncEffect};
use crate::domains::organization::commands::OrganizationCommand;
use crate::domains::organization::events::OrganizationEvent;

/// Composite Effect - Routes OrganizationCommand to appropriate sub-effect
///
/// This composite effect solves the problem of having multiple effects for the same command type.
/// The dispatcher requires one effect per command type, so this effect routes based on the command variant.
pub struct OrganizationEffect {
    scraper: ScraperEffect,
    ai: AIEffect,
    sync: SyncEffect,
    need: NeedEffect,
}

impl OrganizationEffect {
    pub fn new() -> Self {
        Self {
            scraper: ScraperEffect,
            ai: AIEffect,
            sync: SyncEffect,
            need: NeedEffect,
        }
    }
}

impl Default for OrganizationEffect {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Effect<OrganizationCommand, ServerDeps> for OrganizationEffect {
    type Event = OrganizationEvent;

    async fn execute(
        &self,
        cmd: OrganizationCommand,
        ctx: EffectContext<ServerDeps>,
    ) -> Result<OrganizationEvent> {
        match &cmd {
            // Route to ScraperEffect
            OrganizationCommand::ScrapeSource { .. } => self.scraper.execute(cmd, ctx).await,
            OrganizationCommand::ScrapeResourceLink { .. } => self.scraper.execute(cmd, ctx).await,

            // Route to AIEffect
            OrganizationCommand::ExtractNeeds { .. } => self.ai.execute(cmd, ctx).await,
            OrganizationCommand::ExtractNeedsFromResourceLink { .. } => self.ai.execute(cmd, ctx).await,

            // Route to SyncEffect
            OrganizationCommand::SyncNeeds { .. } => self.sync.execute(cmd, ctx).await,

            // Route to NeedEffect
            OrganizationCommand::CreateNeed { .. }
            | OrganizationCommand::CreateNeedsFromResourceLink { .. }
            | OrganizationCommand::UpdateNeedStatus { .. }
            | OrganizationCommand::UpdateNeedAndApprove { .. }
            | OrganizationCommand::CreatePost { .. }
            | OrganizationCommand::GenerateNeedEmbedding { .. }
            | OrganizationCommand::CreateCustomPost { .. }
            | OrganizationCommand::RepostNeed { .. }
            | OrganizationCommand::ExpirePost { .. }
            | OrganizationCommand::ArchivePost { .. }
            | OrganizationCommand::IncrementPostView { .. }
            | OrganizationCommand::IncrementPostClick { .. } => self.need.execute(cmd, ctx).await,
        }
    }
}
