//! Curator monitor activity — re-crawl, re-extract, and sync agent's websites.
//!
//! Pipeline:
//! 1. Load agent + curator config + linked websites
//! 2. For each website:
//!    a. Re-crawl (best-effort)
//!    b. Re-extract posts using agent's purpose and tag instructions
//!    c. LLM sync: compare fresh extraction against existing agent posts → proposals
//! 3. Return stats (websites_monitored, proposals_insert, proposals_update, etc.)

use anyhow::Result;
use tracing::{info, warn};
use uuid::Uuid;

use crate::common::MemberId;
use crate::domains::agents::activities::extract::extract_posts_for_website;
use crate::domains::agents::models::{
    AgentCuratorConfig, AgentRequiredTagKind, AgentRun, AgentRunStat, AgentWebsite,
};
use crate::domains::crawling::activities::ingest_website::ingest_website;
use crate::domains::posts::activities::llm_sync::llm_sync_posts;
use crate::domains::tag::models::tag_kind_config::build_tag_instructions_for_kinds;
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
        AgentRunStat::create_batch(run.id, &[("websites_monitored", 0)], pool).await?;
        return Ok(run);
    }

    // Load curator config for purpose + tag instructions
    let curator = AgentCuratorConfig::find_by_agent(agent_id, pool).await?;
    let required_tag_kinds = AgentRequiredTagKind::find_by_agent(agent_id, pool)
        .await
        .unwrap_or_default();
    let tag_kind_ids: Vec<Uuid> = required_tag_kinds.iter().map(|r| r.tag_kind_id).collect();
    let tag_instructions = build_tag_instructions_for_kinds(&tag_kind_ids, pool)
        .await
        .unwrap_or_default();

    let mut websites_monitored: i32 = 0;
    let mut proposals_insert: i32 = 0;
    let mut proposals_update: i32 = 0;
    let mut proposals_delete: i32 = 0;
    let mut proposals_merge: i32 = 0;
    let mut errors: i32 = 0;

    for agent_website in &agent_websites {
        let website = match Website::find_by_id(agent_website.website_id.into(), pool).await {
            Ok(w) => w,
            Err(e) => {
                warn!(website_id = %agent_website.website_id, error = %e, "Website not found, skipping");
                errors += 1;
                continue;
            }
        };

        let domain = &website.domain;
        info!(domain = %domain, "Monitoring website for updates");

        // Step 1: Re-crawl (best-effort)
        if let Err(e) = ingest_website(
            agent_website.website_id,
            MemberId::nil().into_uuid(),
            true,  // is_admin (system process)
            deps,
        )
        .await
        {
            warn!(domain = %domain, error = %e, "Re-crawl failed, continuing with existing pages");
        }

        // Step 2: Re-extract using agent's purpose
        let extraction_result = match extract_posts_for_website(
            agent_website.website_id,
            &curator.purpose,
            &tag_instructions,
            deps,
        )
        .await
        {
            Ok(r) => r,
            Err(e) => {
                warn!(domain = %domain, error = %e, "Extraction failed during monitor, skipping");
                errors += 1;
                continue;
            }
        };

        if extraction_result.posts.is_empty() {
            info!(domain = %domain, "No posts extracted during monitor");
            continue;
        }

        websites_monitored += 1;

        // Step 3: LLM sync — creates proposals for admin review
        let website_id = crate::common::WebsiteId::from_uuid(agent_website.website_id);
        match llm_sync_posts(
            website_id,
            Some(agent_id),
            extraction_result.posts,
            deps.ai.as_ref(),
            pool,
        )
        .await
        {
            Ok(result) => {
                proposals_insert += result.staged_inserts as i32;
                proposals_update += result.staged_updates as i32;
                proposals_delete += result.staged_deletes as i32;
                proposals_merge += result.staged_merges as i32;
                info!(
                    domain = %domain,
                    inserts = result.staged_inserts,
                    updates = result.staged_updates,
                    deletes = result.staged_deletes,
                    merges = result.staged_merges,
                    "Monitor sync complete for website"
                );
            }
            Err(e) => {
                warn!(domain = %domain, error = %e, "LLM sync failed during monitor");
                errors += 1;
            }
        }
    }

    let total_proposals = proposals_insert + proposals_update + proposals_delete + proposals_merge;

    AgentRunStat::create_batch(
        run.id,
        &[
            ("websites_monitored", websites_monitored),
            ("proposals_insert", proposals_insert),
            ("proposals_update", proposals_update),
            ("proposals_delete", proposals_delete),
            ("proposals_merge", proposals_merge),
            ("proposals_total", total_proposals),
            ("errors", errors),
        ],
        pool,
    )
    .await?;

    let run = AgentRun::complete(run.id, pool).await?;

    info!(
        run_id = %run.id,
        websites_monitored,
        total_proposals,
        errors,
        "Monitor step completed"
    );

    Ok(run)
}
