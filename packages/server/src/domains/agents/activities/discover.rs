//! Curator discover activity — search the web for relevant websites.
//!
//! Pipeline:
//! 1. Load agent + curator config + active search queries
//! 2. For each query, execute Tavily search
//! 3. Deduplicate by domain (skip websites already linked to this agent)
//! 4. AI pre-filter against agent's filter rules
//! 5. Create websites that don't exist yet
//! 6. Create agent_websites join rows for all passing results
//! 7. Return stats

use anyhow::Result;
use tracing::{info, warn};
use uuid::Uuid;

use crate::domains::agents::activities::evaluate_filter::{
    evaluate_websites_against_filters, WebsiteCandidate,
};
use crate::domains::agents::models::{
    AgentCuratorConfig, AgentFilterRule, AgentRun, AgentRunStat, AgentSearchQuery, AgentWebsite,
};
use crate::domains::website::models::{CreateWebsite, Website};
use crate::kernel::ServerDeps;

/// Default location for {location} placeholder substitution.
const DEFAULT_LOCATION: &str = "Twin Cities, Minnesota";

/// Run the discover step for a curator agent.
pub async fn discover(
    agent_id: Uuid,
    trigger_type: &str,
    deps: &ServerDeps,
) -> Result<AgentRun> {
    let pool = &deps.db_pool;

    let run = AgentRun::create(agent_id, "discover", trigger_type, pool).await?;
    info!(run_id = %run.id, agent_id = %agent_id, "Starting discover step");

    let curator = AgentCuratorConfig::find_by_agent(agent_id, pool).await?;
    let queries = AgentSearchQuery::find_active_by_agent(agent_id, pool).await?;

    if queries.is_empty() {
        info!("No active search queries, completing run");
        let run = AgentRun::complete(run.id, pool).await?;
        AgentRunStat::create_batch(
            run.id,
            &[
                ("queries_executed", 0),
                ("total_results", 0),
                ("websites_created", 0),
                ("websites_filtered", 0),
            ],
            pool,
        )
        .await?;
        return Ok(run);
    }

    let filter_rules = AgentFilterRule::find_active_by_agent(agent_id, pool).await?;

    let mut total_results: i32 = 0;
    let mut websites_created: i32 = 0;
    let mut websites_filtered: i32 = 0;
    let queries_executed = queries.len() as i32;

    for query in &queries {
        let search_query = query.query_text.replace("{location}", DEFAULT_LOCATION);

        info!(query_id = %query.id, query = %search_query, "Running agent search");

        let results = match deps.web_searcher.search_with_limit(&search_query, 10).await {
            Ok(r) => r,
            Err(e) => {
                warn!(query = %search_query, error = %e, "Search failed, skipping");
                continue;
            }
        };

        info!(query = %search_query, results_count = results.len(), "Search returned results");

        // Filter low relevance and extract domain info
        let mut candidates: Vec<WebsiteCandidate> = Vec::new();
        for result in &results {
            total_results += 1;

            if result.score.unwrap_or(0.0) < 0.5 {
                continue;
            }

            let domain = match extract_domain(result.url.as_str()) {
                Some(d) => d,
                None => continue,
            };

            // Skip if website already exists AND is linked to this agent
            if let Ok(Some(website)) = Website::find_by_domain(&domain, pool).await {
                // Check if already linked to this agent
                let linked = AgentWebsite::find_by_agent(agent_id, pool).await?;
                if linked.iter().any(|aw| aw.website_id == website.id.into_uuid()) {
                    continue;
                }
            }

            candidates.push(WebsiteCandidate {
                domain,
                url: result.url.to_string(),
                title: result.title.clone().unwrap_or_default(),
                snippet: result.snippet.clone().unwrap_or_default(),
            });
        }

        if candidates.is_empty() {
            continue;
        }

        // Deduplicate candidates by domain within this query
        let mut seen_domains = std::collections::HashSet::new();
        candidates.retain(|c| seen_domains.insert(c.domain.clone()));

        // AI pre-filter evaluation
        let evaluations = evaluate_websites_against_filters(
            &candidates,
            &filter_rules,
            &curator.purpose,
            &deps.ai,
        )
        .await?;

        // Process results
        for (candidate, evaluation) in candidates.iter().zip(evaluations.iter()) {
            if evaluation.passed {
                // Find or create website
                match Website::find_or_create(
                    CreateWebsite::builder()
                        .url_or_domain(candidate.domain.clone())
                        .submitter_type("agent")
                        .submission_context(Some(format!(
                            "Agent discover: {}",
                            query.query_text
                        )))
                        .max_crawl_depth(3)
                        .build(),
                    pool,
                )
                .await
                {
                    Ok(w) => {
                        // Link website to agent
                        if let Err(e) =
                            AgentWebsite::link(agent_id, w.id.into_uuid(), pool).await
                        {
                            warn!(domain = %candidate.domain, error = %e, "Failed to link website to agent");
                        } else {
                            info!(domain = %candidate.domain, "Linked website to agent");
                            websites_created += 1;
                        }
                    }
                    Err(e) => {
                        warn!(domain = %candidate.domain, error = %e, "Failed to create website");
                    }
                }
            } else {
                websites_filtered += 1;
                info!(
                    domain = %candidate.domain,
                    reason = %evaluation.reason,
                    "Filtered website"
                );
            }
        }
    }

    AgentRunStat::create_batch(
        run.id,
        &[
            ("queries_executed", queries_executed),
            ("total_results", total_results),
            ("websites_created", websites_created),
            ("websites_filtered", websites_filtered),
        ],
        pool,
    )
    .await?;

    let run = AgentRun::complete(run.id, pool).await?;

    info!(
        run_id = %run.id,
        queries_executed,
        total_results,
        websites_created,
        websites_filtered,
        "Discover step completed"
    );

    Ok(run)
}

fn extract_domain(url: &str) -> Option<String> {
    url::Url::parse(url).ok()?.host_str().map(|s| s.to_string())
}
