use anyhow::Result;
use async_trait::async_trait;
use seesaw_core::{Effect, EffectContext};

use super::deps::ServerDeps;
use crate::common::{WebsiteId, JobId, MemberId};
use crate::domains::listings::commands::ListingCommand;
use crate::domains::listings::events::ListingEvent;
use crate::domains::scraping::models::{Agent, Website};

/// Sync Effect - Handles SyncListings command
///
/// This effect is a thin orchestration layer that dispatches commands to handler functions.
pub struct SyncEffect;

#[async_trait]
impl Effect<ListingCommand, ServerDeps> for SyncEffect {
    type Event = ListingEvent;

    async fn execute(
        &self,
        cmd: ListingCommand,
        ctx: EffectContext<ServerDeps>,
    ) -> Result<ListingEvent> {
        match cmd {
            ListingCommand::SyncListings {
                source_id,
                job_id,
                listings,
            } => handle_sync_listings(source_id, job_id, listings, &ctx).await,
            _ => anyhow::bail!("SyncEffect: Unexpected command"),
        }
    }
}

// ============================================================================
// Handler function
// ============================================================================

async fn handle_sync_listings(
    source_id: WebsiteId,
    job_id: JobId,
    listings: Vec<crate::common::ExtractedListing>,
    ctx: &EffectContext<ServerDeps>,
) -> Result<ListingEvent> {
    tracing::info!(
        source_id = %source_id,
        job_id = %job_id,
        listings_count = listings.len(),
        "Starting database sync for extracted listings"
    );

    let result =
        match super::syncing::sync_extracted_listings(source_id, listings, &ctx.deps().db_pool).await {
            Ok(r) => {
                tracing::info!(
                    source_id = %source_id,
                    new_count = r.new_count,
                    changed_count = r.changed_count,
                    disappeared_count = r.disappeared_count,
                    "Database sync completed successfully"
                );
                r
            }
            Err(e) => {
                tracing::error!(
                    source_id = %source_id,
                    error = %e,
                    "Database sync failed"
                );
                return Ok(ListingEvent::SyncFailed {
                    source_id,
                    job_id,
                    reason: format!("Failed to sync listings: {}", e),
                });
            }
        };

    // Auto-approve website if agent has auto_approve_websites enabled and listings were found
    if result.new_count > 0 {
        match auto_approve_website_if_enabled(source_id, &ctx.deps().db_pool).await {
            Ok(approved) => {
                if approved {
                    tracing::info!(
                        source_id = %source_id,
                        new_listings = result.new_count,
                        "Website auto-approved by agent after finding listings"
                    );
                }
            }
            Err(e) => {
                tracing::warn!(
                    source_id = %source_id,
                    error = %e,
                    "Failed to auto-approve website (continuing anyway)"
                );
            }
        }
    }

    tracing::info!(
        source_id = %source_id,
        job_id = %job_id,
        "Emitting ListingsSynced event"
    );
    Ok(ListingEvent::ListingsSynced {
        source_id,
        job_id,
        new_count: result.new_count,
        changed_count: result.changed_count,
        disappeared_count: result.disappeared_count,
    })
}

/// Auto-approve website if it has an agent with auto_approve_websites enabled
///
/// Returns: Ok(true) if website was auto-approved, Ok(false) if not applicable, Err on failure
async fn auto_approve_website_if_enabled(
    website_id: WebsiteId,
    pool: &sqlx::PgPool,
) -> Result<bool> {
    // Load website
    let website = Website::find_by_id(website_id, pool).await?;

    // Check if website is still pending review (only auto-approve if pending)
    if website.status != "pending_review" {
        return Ok(false);
    }

    // Check if website has an agent_id
    let agent_id = sqlx::query_scalar::<_, Option<uuid::Uuid>>(
        "SELECT agent_id FROM websites WHERE id = $1"
    )
    .bind(website.id)
    .fetch_one(pool)
    .await?;

    let agent_id = match agent_id {
        Some(id) => id,
        None => return Ok(false), // No agent, no auto-approval
    };

    // Load agent
    let agent = Agent::find_by_id(agent_id, pool).await?;

    // Check if agent has auto_approve_domains enabled
    if !agent.auto_approve_domains {
        return Ok(false);
    }

    // Auto-approve the website (system user)
    tracing::info!(
        website_id = %website_id,
        agent_name = %agent.name,
        "Auto-approving website discovered by agent"
    );

    Website::approve(website_id, MemberId::nil(), pool).await?;

    // Increment agent's approved count
    Agent::increment_approved_count(agent_id, pool).await?;

    Ok(true)
}
