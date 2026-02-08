//! Curator monitor activity — re-crawl and sync agent's websites.
//!
//! Pipeline:
//! 1. Load agent's websites (via agent_websites)
//! 2. Trigger re-crawl for each website
//! 3. Re-extract and compare against existing posts
//! 4. Return stats

use anyhow::Result;
use tracing::{info, warn};
use uuid::Uuid;

use crate::domains::agents::models::{AgentRun, AgentRunStat, AgentWebsite};
use crate::domains::posts::models::Post;
use crate::domains::website::models::Website;
use crate::kernel::ServerDeps;

/// Run the monitor step for a curator agent.
pub async fn monitor(
    agent_id: Uuid,
    trigger_type: &str,
    deps: &ServerDeps,
) -> Result<AgentRun> {
    let pool = &deps.db_pool;

    let run = AgentRun::create(agent_id, "monitor", trigger_type, pool).await?;
    info!(run_id = %run.id, agent_id = %agent_id, "Starting monitor step");

    let agent_websites = AgentWebsite::find_by_agent(agent_id, pool).await?;

    if agent_websites.is_empty() {
        info!("No linked websites, completing run");
        let run = AgentRun::complete(run.id, pool).await?;
        AgentRunStat::create_batch(run.id, &[("websites_checked", 0)], pool).await?;
        return Ok(run);
    }

    let extraction = deps.extraction.as_ref().ok_or_else(|| {
        anyhow::anyhow!("Extraction service not configured")
    })?;

    let mut websites_checked: i32 = 0;

    for agent_website in &agent_websites {
        let website = match Website::find_by_id(agent_website.website_id.into(), pool).await {
            Ok(w) => w,
            Err(e) => {
                warn!(website_id = %agent_website.website_id, error = %e, "Website not found, skipping");
                continue;
            }
        };

        let domain = &website.domain;
        info!(domain = %domain, "Monitoring website for updates");

        // Re-search pages
        let pages = match extraction
            .search_and_get_pages("", Some(domain), 50)
            .await
        {
            Ok(p) => p,
            Err(e) => {
                warn!(domain = %domain, error = %e, "Failed to search pages, skipping");
                continue;
            }
        };

        if pages.is_empty() {
            info!(domain = %domain, "No pages found during monitoring");
            continue;
        }

        websites_checked += 1;

        // For now, log what was found. Full sync/proposal generation is a future enhancement
        // that will compare extracted posts against existing agent posts and generate
        // sync proposals for human review.
        let existing_posts = Post::find_by_agent(agent_id, pool).await.unwrap_or_default();
        info!(
            domain = %domain,
            pages_found = pages.len(),
            existing_posts = existing_posts.len(),
            "Monitor check complete for website"
        );
    }

    AgentRunStat::create_batch(
        run.id,
        &[("websites_checked", websites_checked)],
        pool,
    )
    .await?;

    let run = AgentRun::complete(run.id, pool).await?;

    info!(
        run_id = %run.id,
        websites_checked,
        "Monitor step completed"
    );

    Ok(run)
}
