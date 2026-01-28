use anyhow::{Context, Result};
use async_trait::async_trait;
use seesaw::{Effect, EffectContext};

use super::deps::ServerDeps;
use super::{sync_needs, ExtractedNeedInput};
use crate::domains::organization::commands::OrganizationCommand;
use crate::domains::organization::events::OrganizationEvent;
use crate::domains::organization::models::source::OrganizationSource;

/// Sync Effect - Handles SyncNeeds command
pub struct SyncEffect;

#[async_trait]
impl Effect<OrganizationCommand, ServerDeps> for SyncEffect {
    type Event = OrganizationEvent;

    async fn execute(
        &self,
        cmd: OrganizationCommand,
        ctx: EffectContext<ServerDeps>,
    ) -> Result<OrganizationEvent> {
        match cmd {
            OrganizationCommand::SyncNeeds {
                source_id,
                job_id,
                needs,
            } => {
                // Get source to fetch organization_name
                let source = OrganizationSource::find_by_id(source_id, &ctx.deps().db_pool)
                    .await
                    .context("Failed to find source")?;

                // Convert event needs to sync input
                let sync_input: Vec<ExtractedNeedInput> = needs
                    .into_iter()
                    .map(|need| ExtractedNeedInput {
                        organization_name: source.organization_name.clone(),
                        title: need.title,
                        description: need.description,
                        description_markdown: None,
                        tldr: Some(need.tldr),
                        contact: need.contact.and_then(|c| {
                            serde_json::json!({
                                "email": c.email,
                                "phone": c.phone,
                                "website": c.website
                            })
                            .as_object()
                            .map(|obj| serde_json::Value::Object(obj.clone()))
                        }),
                        urgency: need.urgency,
                        confidence: need.confidence,
                    })
                    .collect();

                // Sync with database
                let sync_result = sync_needs(&ctx.deps().db_pool, source_id, sync_input)
                    .await
                    .context("Sync failed")?;

                // Return fact event
                Ok(OrganizationEvent::NeedsSynced {
                    source_id,
                    job_id,
                    new_count: sync_result.new_needs.len(),
                    changed_count: sync_result.changed_needs.len(),
                    disappeared_count: sync_result.disappeared_needs.len(),
                })
            }
            _ => anyhow::bail!("SyncEffect: Unexpected command"),
        }
    }
}
