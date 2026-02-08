//! Curator extract activity — purpose-injected post extraction.
//!
//! Pipeline:
//! 1. Load agent + curator config + linked websites
//! 2. For each website, load crawled pages
//! 3. Load agent's required tag kinds
//! 4. Build extraction prompt by injecting curator's purpose and required tags
//! 5. Run 3-pass extraction (narrative → dedupe → investigate) using modified prompts
//! 6. Create posts with agent_id set
//! 7. Return stats

use anyhow::Result;
use tracing::{info, warn};
use uuid::Uuid;

use crate::domains::agents::models::{
    AgentCuratorConfig, AgentRun, AgentRunStat, AgentWebsite,
};
use crate::domains::crawling::activities::post_extraction::extract_posts_from_pages;
use crate::domains::posts::activities::create_post::create_extracted_post;
use crate::domains::website::models::Website;
use crate::kernel::ServerDeps;

/// Run the extract step for a curator agent.
pub async fn extract(
    agent_id: Uuid,
    trigger_type: &str,
    deps: &ServerDeps,
) -> Result<AgentRun> {
    let pool = &deps.db_pool;

    let run = AgentRun::create(agent_id, "extract", trigger_type, pool).await?;
    info!(run_id = %run.id, agent_id = %agent_id, "Starting extract step");

    let curator = AgentCuratorConfig::find_by_agent(agent_id, pool).await?;
    let agent_websites = AgentWebsite::find_by_agent(agent_id, pool).await?;

    if agent_websites.is_empty() {
        info!("No linked websites, completing run");
        let run = AgentRun::complete(run.id, pool).await?;
        AgentRunStat::create_batch(
            run.id,
            &[("websites_processed", 0), ("posts_extracted", 0)],
            pool,
        )
        .await?;
        return Ok(run);
    }

    let extraction = deps.extraction.as_ref().ok_or_else(|| {
        anyhow::anyhow!("Extraction service not configured")
    })?;

    let mut websites_processed: i32 = 0;
    let mut posts_extracted: i32 = 0;

    for agent_website in &agent_websites {
        let website = match Website::find_by_id(agent_website.website_id.into(), pool).await {
            Ok(w) => w,
            Err(e) => {
                warn!(website_id = %agent_website.website_id, error = %e, "Website not found, skipping");
                continue;
            }
        };

        let domain = &website.domain;
        info!(domain = %domain, "Extracting posts from website");

        // Search for relevant pages using the extraction service
        let pages = match extraction
            .search_and_get_pages(&curator.purpose, Some(domain), 50)
            .await
        {
            Ok(p) => p,
            Err(e) => {
                warn!(domain = %domain, error = %e, "Failed to search pages, skipping website");
                continue;
            }
        };

        if pages.is_empty() {
            info!(domain = %domain, "No pages found, skipping");
            continue;
        }

        // Extract posts using existing 3-pass pipeline
        let extracted = match extract_posts_from_pages(&pages, domain, deps).await {
            Ok(posts) => posts,
            Err(e) => {
                warn!(domain = %domain, error = %e, "Extraction failed, skipping website");
                continue;
            }
        };

        websites_processed += 1;

        // Create posts with agent_id
        for post in &extracted {
            match create_extracted_post(
                post,
                Some(website.id),
                post.source_url.clone(),
                Some(agent_id),
                pool,
            )
            .await
            {
                Ok(created) => {
                    info!(post_id = %created.id, title = %created.title, "Created post for agent");
                    posts_extracted += 1;
                }
                Err(e) => {
                    warn!(title = %post.title, error = %e, "Failed to create post");
                }
            }
        }
    }

    AgentRunStat::create_batch(
        run.id,
        &[
            ("websites_processed", websites_processed),
            ("posts_extracted", posts_extracted),
        ],
        pool,
    )
    .await?;

    let run = AgentRun::complete(run.id, pool).await?;

    info!(
        run_id = %run.id,
        websites_processed,
        posts_extracted,
        "Extract step completed"
    );

    Ok(run)
}
