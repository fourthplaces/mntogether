//! Main discovery pipeline action
//!
//! Replaces the hardcoded discovery in posts/effects/discovery.rs with a
//! database-driven pipeline that includes AI pre-filtering.
//!
//! Pipeline:
//! 1. Load active queries from DB
//! 2. Execute Tavily searches with {location} substitution
//! 3. Deduplicate by domain (skip existing websites)
//! 4. AI pre-filter against applicable rules
//! 5. Create pending websites for passing results
//! 6. Store all results in discovery_run_results for lineage

use anyhow::Result;
use tracing::{info, warn};

use crate::domains::discovery::activities::evaluate_filter::{
    evaluate_websites_against_filters, WebsiteCandidate,
};
use crate::domains::discovery::events::DiscoveryEvent;
use crate::domains::discovery::models::{
    DiscoveryFilterRule, DiscoveryQuery, DiscoveryRun, DiscoveryRunResult,
};
use crate::domains::website::models::{CreateWebsite, Website};
use crate::kernel::ServerDeps;
use extraction::WebSearcher;

/// Default location for {location} placeholder substitution
const DEFAULT_LOCATION: &str = "Twin Cities, Minnesota";

/// Run the full discovery pipeline.
///
/// Returns a DiscoveryEvent with run statistics.
pub async fn run_discovery(trigger_type: &str, deps: &ServerDeps) -> Result<DiscoveryEvent> {
    let pool = &deps.db_pool;

    // Create the run record
    let run = DiscoveryRun::create(trigger_type, pool).await?;

    info!(run_id = %run.id, trigger_type, "Starting discovery run");

    // Load active queries
    let queries = DiscoveryQuery::find_active(pool).await?;
    if queries.is_empty() {
        info!("No active discovery queries, completing run");
        let run = DiscoveryRun::complete(run.id, 0, 0, 0, 0, pool).await?;
        return Ok(DiscoveryEvent::DiscoveryRunCompleted {
            run_id: run.id,
            queries_executed: 0,
            total_results: 0,
            websites_created: 0,
            websites_filtered: 0,
        });
    }

    // Load global filter rules once
    let global_rules = DiscoveryFilterRule::find_global(pool).await?;

    let mut total_results: i32 = 0;
    let mut websites_created: i32 = 0;
    let mut websites_filtered: i32 = 0;
    let queries_executed = queries.len() as i32;

    for query in &queries {
        let search_query = query.query_text.replace("{location}", DEFAULT_LOCATION);

        info!(query_id = %query.id, query = %search_query, "Running discovery search");

        // Execute Tavily search
        let results = match deps.web_searcher.search_with_limit(&search_query, 10).await {
            Ok(r) => r,
            Err(e) => {
                warn!(query = %search_query, error = %e, "Search failed, skipping");
                continue;
            }
        };

        info!(query = %search_query, results_count = results.len(), "Search returned results");

        // Filter low relevance and extract domain info
        let mut candidates: Vec<(WebsiteCandidate, Option<f64>)> = Vec::new();
        for result in &results {
            total_results += 1;

            if result.score.unwrap_or(0.0) < 0.5 {
                continue;
            }

            let domain = match extract_domain(result.url.as_str()) {
                Some(d) => d,
                None => continue,
            };

            // Skip if website already exists in our system
            if Website::find_by_domain(&domain, pool).await?.is_some() {
                continue;
            }

            candidates.push((
                WebsiteCandidate {
                    domain,
                    url: result.url.to_string(),
                    title: result.title.clone().unwrap_or_default(),
                    snippet: result.snippet.clone().unwrap_or_default(),
                },
                result.score.map(|s| s as f64),
            ));
        }

        if candidates.is_empty() {
            continue;
        }

        // Deduplicate candidates by domain within this query
        let mut seen_domains = std::collections::HashSet::new();
        candidates.retain(|(c, _)| seen_domains.insert(c.domain.clone()));

        // Load per-query filter rules
        let query_rules = DiscoveryFilterRule::find_by_query(query.id, pool).await?;

        // AI pre-filter evaluation
        let candidate_refs: Vec<WebsiteCandidate> =
            candidates.iter().map(|(c, _)| c.clone()).collect();
        let evaluations = evaluate_websites_against_filters(
            &candidate_refs,
            &global_rules,
            &query_rules,
            &deps.ai,
        )
        .await?;

        // Process results
        for ((candidate, score), evaluation) in candidates.iter().zip(evaluations.iter()) {
            let filter_result = if evaluation.passed {
                "passed"
            } else {
                "filtered"
            };

            let mut website_id = None;

            if evaluation.passed {
                // Create pending website
                match Website::create(
                    CreateWebsite::builder()
                        .url_or_domain(candidate.domain.clone())
                        .submitter_type("system")
                        .submission_context(Some(format!("Discovery: {}", search_query)))
                        .max_crawl_depth(3)
                        .build(),
                    pool,
                )
                .await
                {
                    Ok(w) => {
                        info!(domain = %candidate.domain, "Created website from discovery");
                        website_id = Some(w.id.into_uuid());
                        websites_created += 1;
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

            // Store result for lineage tracking
            let _ = DiscoveryRunResult::create(
                run.id,
                query.id,
                candidate.domain.clone(),
                candidate.url.clone(),
                Some(candidate.title.clone()),
                Some(candidate.snippet.clone()),
                *score,
                filter_result,
                Some(evaluation.reason.clone()),
                website_id,
                pool,
            )
            .await;
        }
    }

    // Complete the run
    let run = DiscoveryRun::complete(
        run.id,
        queries_executed,
        total_results,
        websites_created,
        websites_filtered,
        pool,
    )
    .await?;

    info!(
        run_id = %run.id,
        queries_executed,
        total_results,
        websites_created,
        websites_filtered,
        "Discovery run completed"
    );

    Ok(DiscoveryEvent::DiscoveryRunCompleted {
        run_id: run.id,
        queries_executed: queries_executed as usize,
        total_results: total_results as usize,
        websites_created: websites_created as usize,
        websites_filtered: websites_filtered as usize,
    })
}

/// Extract domain from URL
fn extract_domain(url: &str) -> Option<String> {
    url::Url::parse(url).ok()?.host_str().map(|s| s.to_string())
}
