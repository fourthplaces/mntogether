//! Global website discovery activity.
//!
//! Loads all active search queries, runs Tavily search, deduplicates by domain,
//! and creates new websites. No agents, no filter rules.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::domains::website::models::{CreateWebsite, SearchQuery, Website};
use crate::impl_restate_serde;
use crate::kernel::ServerDeps;

/// Default location for {location} placeholder substitution.
const DEFAULT_LOCATION: &str = "Twin Cities, Minnesota";

/// Result of a global discovery run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryResult {
    pub queries_executed: i32,
    pub total_results: i32,
    pub websites_created: i32,
    pub websites_skipped: i32,
}

impl_restate_serde!(DiscoveryResult);

/// Run global discovery: search the web for relevant websites using all active queries.
pub async fn run_discovery(deps: &ServerDeps) -> Result<DiscoveryResult> {
    let pool = &deps.db_pool;

    let queries = SearchQuery::find_active(pool).await?;

    if queries.is_empty() {
        info!("No active search queries");
        return Ok(DiscoveryResult {
            queries_executed: 0,
            total_results: 0,
            websites_created: 0,
            websites_skipped: 0,
        });
    }

    let mut total_results: i32 = 0;
    let mut websites_created: i32 = 0;
    let mut websites_skipped: i32 = 0;
    let queries_executed = queries.len() as i32;

    for query in &queries {
        let search_query = query.query_text.replace("{location}", DEFAULT_LOCATION);

        info!(query_id = %query.id, query = %search_query, "Running discovery search");

        let results = match deps.web_searcher.search_with_limit(&search_query, 10).await {
            Ok(r) => r,
            Err(e) => {
                warn!(query = %search_query, error = %e, "Search failed, skipping");
                continue;
            }
        };

        info!(query = %search_query, results_count = results.len(), "Search returned results");

        // Deduplicate by domain within this query
        let mut seen_domains = std::collections::HashSet::new();

        for result in &results {
            total_results += 1;

            if result.score.unwrap_or(0.0) < 0.5 {
                continue;
            }

            let domain = match extract_domain(result.url.as_str()) {
                Some(d) => d,
                None => continue,
            };

            if !seen_domains.insert(domain.clone()) {
                continue;
            }

            // Skip if website already exists
            if let Ok(Some(_)) = Website::find_by_domain(&domain, pool).await {
                websites_skipped += 1;
                continue;
            }

            // Create new website
            match Website::create(
                CreateWebsite::builder()
                    .url_or_domain(domain.clone())
                    .submitter_type("system".to_string())
                    .submission_context(Some(format!("Discovery: {}", query.query_text)))
                    .max_crawl_depth(3)
                    .build(),
                pool,
            )
            .await
            {
                Ok(_) => {
                    info!(domain = %domain, "Created website from discovery");
                    websites_created += 1;
                }
                Err(e) => {
                    warn!(domain = %domain, error = %e, "Failed to create website");
                }
            }
        }
    }

    info!(
        queries_executed,
        total_results, websites_created, websites_skipped, "Discovery completed"
    );

    Ok(DiscoveryResult {
        queries_executed,
        total_results,
        websites_created,
        websites_skipped,
    })
}

fn extract_domain(url: &str) -> Option<String> {
    url::Url::parse(url).ok()?.host_str().map(|s| s.to_string())
}
