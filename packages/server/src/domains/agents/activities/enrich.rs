//! Curator enrich activity — investigate posts for missing data.
//!
//! Pipeline:
//! 1. Load agent's required tag kinds
//! 2. Load agent's posts that are missing required tags
//! 3. Run agentic investigation for each incomplete post
//! 4. Update posts with findings, apply discovered tags
//! 5. Return stats

use anyhow::Result;
use tracing::{info, warn};
use uuid::Uuid;

use crate::domains::agents::models::{AgentRequiredTagKind, AgentRun, AgentRunStat};
use crate::domains::crawling::activities::post_extraction::{investigate_post, NarrativePost};
use crate::domains::posts::activities::create_post::{
    save_contact_info, tag_post_from_extracted, tag_with_audience_roles,
};
use crate::domains::posts::models::Post;
use crate::domains::tag::models::tag_kind_config::{build_tag_instructions, TagKindConfig};
use crate::domains::tag::Tag;
use crate::kernel::ServerDeps;

/// Run the enrich step for a curator agent.
pub async fn enrich(
    agent_id: Uuid,
    trigger_type: &str,
    deps: &ServerDeps,
) -> Result<AgentRun> {
    let pool = &deps.db_pool;

    let run = AgentRun::create(agent_id, "enrich", trigger_type, pool).await?;
    info!(run_id = %run.id, agent_id = %agent_id, "Starting enrich step");

    // Load agent's required tag kinds
    let required = AgentRequiredTagKind::find_by_agent(agent_id, pool).await?;
    let mut required_kind_slugs: Vec<String> = Vec::new();
    for req in &required {
        if let Ok(kind) = TagKindConfig::find_by_id(req.tag_kind_id, pool).await {
            required_kind_slugs.push(kind.slug.clone());
        }
    }

    // Load agent's posts
    let posts = Post::find_by_agent(agent_id, pool).await?;

    if posts.is_empty() {
        info!("No posts for this agent, completing run");
        let run = AgentRun::complete(run.id, pool).await?;
        AgentRunStat::create_batch(run.id, &[("posts_enriched", 0)], pool).await?;
        return Ok(run);
    }

    // Find posts missing required tags
    let mut posts_needing_enrichment = Vec::new();
    for post in &posts {
        let post_tags = Tag::find_for_post(post.id, pool).await.unwrap_or_default();
        let post_tag_kinds: Vec<&str> = post_tags.iter().map(|t| t.kind.as_str()).collect();

        let missing: Vec<&str> = required_kind_slugs
            .iter()
            .filter(|slug| !post_tag_kinds.contains(&slug.as_str()))
            .map(|s| s.as_str())
            .collect();

        if !missing.is_empty() {
            posts_needing_enrichment.push((post.clone(), missing.iter().map(|s| s.to_string()).collect::<Vec<_>>()));
        }
    }

    if posts_needing_enrichment.is_empty() {
        info!("All posts have required tags, completing run");
        let run = AgentRun::complete(run.id, pool).await?;
        AgentRunStat::create_batch(
            run.id,
            &[("posts_enriched", 0), ("posts_still_missing_tags", 0)],
            pool,
        )
        .await?;
        return Ok(run);
    }

    info!(
        posts_to_enrich = posts_needing_enrichment.len(),
        "Found posts needing enrichment"
    );

    let tag_instructions = build_tag_instructions(pool).await.unwrap_or_default();
    let mut posts_enriched: i32 = 0;
    let mut posts_still_missing: i32 = 0;

    for (post, missing_kinds) in &posts_needing_enrichment {
        info!(
            post_id = %post.id,
            title = %post.title,
            missing = ?missing_kinds,
            "Enriching post"
        );

        // Build a narrative for investigation
        let narrative = NarrativePost {
            title: post.title.clone(),
            tldr: post.tldr.clone().unwrap_or_default(),
            description: post.description.clone(),
            source_url: post.source_url.clone().unwrap_or_default(),
        };

        match investigate_post(&narrative, &tag_instructions, deps).await {
            Ok(info) => {
                // Apply newly discovered tags
                let tags_map = crate::common::TagEntry::to_map(&info.tags);
                tag_post_from_extracted(post.id, &tags_map, pool).await;

                // Apply audience roles if found
                if !info.audience_roles.is_empty() {
                    tag_with_audience_roles(post.id, &info.audience_roles, pool).await;
                }

                // Save contact info if found
                if let Some(ref contact) = info.contact_or_none() {
                    save_contact_info(post.id, contact, pool).await;
                }

                posts_enriched += 1;

                // Check if still missing after enrichment
                let post_tags = Tag::find_for_post(post.id, pool).await.unwrap_or_default();
                let post_tag_kinds: Vec<&str> = post_tags.iter().map(|t| t.kind.as_str()).collect();
                let still_missing: Vec<&str> = required_kind_slugs
                    .iter()
                    .filter(|slug| !post_tag_kinds.contains(&slug.as_str()))
                    .map(|s| s.as_str())
                    .collect();
                if !still_missing.is_empty() {
                    posts_still_missing += 1;
                }
            }
            Err(e) => {
                warn!(post_id = %post.id, error = %e, "Enrichment failed for post");
                posts_still_missing += 1;
            }
        }
    }

    AgentRunStat::create_batch(
        run.id,
        &[
            ("posts_enriched", posts_enriched),
            ("posts_still_missing_tags", posts_still_missing),
        ],
        pool,
    )
    .await?;

    let run = AgentRun::complete(run.id, pool).await?;

    info!(
        run_id = %run.id,
        posts_enriched,
        posts_still_missing,
        "Enrich step completed"
    );

    Ok(run)
}
