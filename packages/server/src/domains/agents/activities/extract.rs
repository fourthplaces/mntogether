//! Curator extract activity — purpose-injected post extraction.
//!
//! Pipeline:
//! 1. Load agent + curator config + linked websites
//! 2. Load agent's required tag kinds → build tag instructions
//! 3. For each website:
//!    a. Search for crawled pages
//!    b. If none found, trigger crawl via ingest_website, then retry
//! 4. Run 3-pass extraction with agent-specific tag instructions
//! 5. Create posts with agent_id set (skip duplicates)
//! 6. Return stats

use anyhow::Result;
use tracing::{info, warn};
use uuid::Uuid;

use crate::common::{ExtractedPost, MemberId};
use crate::domains::agents::models::{
    Agent, AgentCuratorConfig, AgentRequiredTagKind, AgentRun, AgentRunStat, AgentWebsite,
};
use crate::domains::crawling::activities::ingest_website::ingest_website;
use crate::domains::crawling::activities::post_extraction::extract_posts_from_pages_with_tags_and_purpose;
use crate::domains::posts::activities::create_post::create_extracted_post;
use crate::domains::tag::models::tag_kind_config::build_tag_instructions_for_kinds;
use crate::domains::website::models::Website;
use crate::kernel::ServerDeps;

/// Result of extracting posts for a single website.
pub struct WebsiteExtractionResult {
    pub posts: Vec<ExtractedPost>,
    pub website_crawled: bool,
}

/// Extract posts for a single website using an agent's purpose and tag kinds.
/// Returns extracted posts — caller decides persistence strategy.
pub async fn extract_posts_for_website(
    website_id: uuid::Uuid,
    curator_purpose: &str,
    tag_instructions: &str,
    deps: &ServerDeps,
) -> Result<WebsiteExtractionResult> {
    let pool = &deps.db_pool;

    let extraction = deps.extraction.as_ref().ok_or_else(|| {
        anyhow::anyhow!("Extraction service not configured")
    })?;

    let website = Website::find_by_id(website_id.into(), pool).await?;
    let domain = &website.domain;

    info!(domain = %domain, "Extracting posts from website");

    // Search for relevant pages using purpose-driven query
    let mut pages = match extraction
        .search_and_get_pages(curator_purpose, Some(domain), 50)
        .await
    {
        Ok(p) => p,
        Err(e) => {
            warn!(domain = %domain, error = %e, "Failed to search pages");
            return Err(e.into());
        }
    };

    let mut website_crawled = false;

    // If no pages found, trigger a crawl and retry
    if pages.is_empty() {
        info!(domain = %domain, "No pages found, triggering crawl");

        match ingest_website(
            website.id.into_uuid(),
            MemberId::nil().into_uuid(),
            true,  // use_firecrawl
            true,  // is_admin
            deps,
        )
        .await
        {
            Ok(result) => {
                info!(
                    domain = %domain,
                    pages_crawled = result.pages_crawled,
                    "Crawl completed, retrying page search"
                );
                website_crawled = true;

                // Retry search after crawl
                pages = match extraction
                    .search_and_get_pages(curator_purpose, Some(domain), 50)
                    .await
                {
                    Ok(p) => p,
                    Err(e) => {
                        warn!(domain = %domain, error = %e, "Retry search failed after crawl");
                        return Err(e.into());
                    }
                };
            }
            Err(e) => {
                warn!(domain = %domain, error = %e, "Crawl failed");
                return Err(e.into());
            }
        }
    }

    if pages.is_empty() {
        info!(domain = %domain, "Still no pages after crawl");
        return Ok(WebsiteExtractionResult {
            posts: vec![],
            website_crawled,
        });
    }

    // Extract posts using 3-pass pipeline with agent-specific tag instructions and purpose
    let posts = extract_posts_from_pages_with_tags_and_purpose(
        &pages,
        domain,
        tag_instructions,
        Some(curator_purpose),
        deps,
    )
    .await?;

    Ok(WebsiteExtractionResult {
        posts,
        website_crawled,
    })
}

/// Run the extract step for a curator agent.
pub async fn extract(
    agent_id: Uuid,
    trigger_type: &str,
    deps: &ServerDeps,
) -> Result<AgentRun> {
    let pool = &deps.db_pool;

    let run = AgentRun::create(agent_id, "extract", trigger_type, pool).await?;
    info!(run_id = %run.id, agent_id = %agent_id, "Starting extract step");

    let agent = Agent::find_by_id(agent_id, pool).await?;
    let curator = AgentCuratorConfig::find_by_agent(agent_id, pool).await?;
    let agent_websites = AgentWebsite::find_by_agent(agent_id, pool).await?;

    if agent_websites.is_empty() {
        info!("No linked websites, completing run");
        let run = AgentRun::complete(run.id, pool).await?;
        AgentRunStat::create_batch(
            run.id,
            &[("websites_processed", 0), ("posts_extracted", 0), ("websites_crawled", 0)],
            pool,
        )
        .await?;
        return Ok(run);
    }

    // Build tag instructions from agent's required tag kinds (if any)
    let required_tag_kinds = AgentRequiredTagKind::find_by_agent(agent_id, pool).await?;
    let tag_kind_ids: Vec<Uuid> = required_tag_kinds.iter().map(|r| r.tag_kind_id).collect();
    let tag_instructions = build_tag_instructions_for_kinds(&tag_kind_ids, pool)
        .await
        .unwrap_or_default();

    if tag_instructions.is_empty() {
        info!("No required tag kinds configured, tags will be empty");
    } else {
        info!(tag_kind_count = tag_kind_ids.len(), "Built tag instructions from required tag kinds");
    }

    let mut websites_processed: i32 = 0;
    let mut posts_extracted: i32 = 0;
    let mut posts_skipped: i32 = 0;
    let mut websites_crawled: i32 = 0;

    for agent_website in &agent_websites {
        let result = match extract_posts_for_website(
            agent_website.website_id,
            &curator.purpose,
            &tag_instructions,
            deps,
        )
        .await
        {
            Ok(r) => r,
            Err(e) => {
                warn!(website_id = %agent_website.website_id, error = %e, "Extraction failed, skipping website");
                continue;
            }
        };

        if result.website_crawled {
            websites_crawled += 1;
        }

        if result.posts.is_empty() {
            continue;
        }

        websites_processed += 1;

        let website = Website::find_by_id(agent_website.website_id.into(), pool).await?;

        // Create posts with submitted_by_id = agent.member_id (skip duplicates gracefully)
        for post in &result.posts {
            match create_extracted_post(
                post,
                Some(website.id),
                post.source_url.clone(),
                Some(agent.member_id),
                pool,
            )
            .await
            {
                Ok(created) => {
                    info!(post_id = %created.id, title = %created.title, "Created post for agent");
                    posts_extracted += 1;
                }
                Err(e) => {
                    let err_str = e.to_string();
                    if err_str.contains("duplicate key") || err_str.contains("unique constraint") {
                        info!(title = %post.title, "Post already exists, skipping");
                        posts_skipped += 1;
                    } else {
                        warn!(title = %post.title, error = %e, "Failed to create post");
                    }
                }
            }
        }
    }

    AgentRunStat::create_batch(
        run.id,
        &[
            ("websites_processed", websites_processed),
            ("posts_extracted", posts_extracted),
            ("posts_skipped_duplicate", posts_skipped),
            ("websites_crawled", websites_crawled),
        ],
        pool,
    )
    .await?;

    let run = AgentRun::complete(run.id, pool).await?;

    info!(
        run_id = %run.id,
        websites_processed,
        posts_extracted,
        posts_skipped,
        websites_crawled,
        "Extract step completed"
    );

    Ok(run)
}
