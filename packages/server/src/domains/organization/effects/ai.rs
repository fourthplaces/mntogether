use anyhow::{Context, Result};
use async_trait::async_trait;
use seesaw::{Effect, EffectContext};
use uuid::Uuid;

use super::ServerDeps;
use crate::domains::organization::commands::OrganizationCommand;
use crate::domains::organization::events::{ContactInfo, ExtractedNeed, OrganizationEvent};
use crate::domains::organization::models::source::OrganizationSource;

/// AI Effect - Handles ExtractNeeds command
pub struct AIEffect;

#[async_trait]
impl Effect<OrganizationCommand, ServerDeps> for AIEffect {
    type Event = OrganizationEvent;

    async fn execute(
        &self,
        cmd: OrganizationCommand,
        ctx: EffectContext<ServerDeps>,
    ) -> Result<OrganizationEvent> {
        match cmd {
            OrganizationCommand::ExtractNeeds {
                source_id,
                job_id,
                organization_name,
                content,
            } => {
                // Extract needs using AI
                let source = OrganizationSource::find_by_id(source_id, &ctx.deps().db_pool)
                    .await
                    .context("Failed to find source")?;

                let ai_needs = ctx
                    .deps()
                    .need_extractor
                    .extract_needs(&organization_name, &content, &source.source_url)
                    .await
                    .context("AI extraction failed")?;

                // Convert to event format
                let extracted_needs: Vec<ExtractedNeed> = ai_needs
                    .into_iter()
                    .map(|need| ExtractedNeed {
                        title: need.title,
                        description: need.description,
                        tldr: need.tldr,
                        contact: need.contact.map(|c| ContactInfo {
                            email: c.email,
                            phone: c.phone,
                            website: c.website,
                        }),
                        urgency: need.urgency,
                        confidence: need.confidence,
                    })
                    .collect();

                // Return fact event
                Ok(OrganizationEvent::NeedsExtracted {
                    source_id,
                    job_id,
                    needs: extracted_needs,
                })
            }
            _ => anyhow::bail!("AIEffect: Unexpected command"),
        }
    }
}
