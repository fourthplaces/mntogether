//! Discovery module - static search queries for finding community resources
//!
//! Replaces the agent-based search system with a simple, maintainable list of queries.
//! Add new queries here to expand discovery coverage.

use anyhow::Result;
use sqlx::PgPool;
use tracing::info;

use crate::domains::scraping::models::Website;
use crate::kernel::traits::BaseSearchService;

/// Search queries for discovering community resources.
///
/// Use `{location}` placeholder for geographic targeting.
/// Add new queries here - they run daily via the scheduler.
pub const DISCOVERY_QUERIES: &[&str] = &[
    // Services - help for people in need
    "community resources social services {location}",
    "food assistance food shelf food bank {location}",
    "housing assistance rental help {location}",
    "emergency shelter homeless services {location}",
    "utility assistance bill help {location}",
    "healthcare free clinic sliding scale {location}",
    "mental health services counseling {location}",
    "childcare assistance programs {location}",
    "senior services elderly assistance {location}",
    "disability services support {location}",
    // Professionals - people who help
    "immigration lawyer attorney {location}",
    "pro bono legal services {location}",
    "nonprofit legal aid {location}",
    "immigration help DACA {location}",
    // Businesses - places to support
    "immigrant owned business {location}",
    "refugee owned restaurant {location}",
    "minority owned business {location}",
    "social enterprise {location}",
    // Opportunities - things to do
    "volunteer opportunities nonprofit {location}",
    "community service opportunities {location}",
    "tutoring mentoring volunteer {location}",
    "refugee resettlement volunteer {location}",
    // Events & Fundraising
    "community fundraising event {location}",
    "nonprofit fundraiser gala {location}",
    "charity event benefit {location}",
    "community benefit dinner {location}",
    "immigrant community event {location}",
    "cultural celebration festival {location}",
];

/// Default location for searches
pub const DEFAULT_LOCATION: &str = "Twin Cities, Minnesota";

/// Result of a discovery search run
#[derive(Debug)]
pub struct DiscoveryResult {
    pub queries_run: usize,
    pub total_results: usize,
    pub websites_created: usize,
}

/// Run all discovery searches and create websites from results.
///
/// Simple flow:
/// 1. For each query in DISCOVERY_QUERIES
/// 2. Run Tavily search
/// 3. Create pending websites for new domains
/// 4. Human review takes it from there
pub async fn run_discovery_searches(
    search_service: &dyn BaseSearchService,
    pool: &PgPool,
) -> Result<DiscoveryResult> {
    let mut total_results = 0;
    let mut websites_created = 0;

    for query_template in DISCOVERY_QUERIES {
        let query = query_template.replace("{location}", DEFAULT_LOCATION);

        info!(query = %query, "Running discovery search");

        // Run search with sensible defaults
        let results = match search_service
            .search(&query, Some(10), Some("basic"), Some(30))
            .await
        {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!(query = %query, error = %e, "Search failed, skipping");
                continue;
            }
        };

        total_results += results.len();

        // Create websites for new domains
        for result in results {
            // Skip low relevance
            if result.score < 0.5 {
                continue;
            }

            let domain = match extract_domain(&result.url) {
                Some(d) => d,
                None => continue,
            };

            // Skip if exists
            if Website::find_by_domain(&domain, pool).await?.is_some() {
                continue;
            }

            // Create pending website
            match Website::create(
                domain.clone(),
                None,
                "discovery".to_string(),
                Some(format!("Found via: {}", query)),
                3, // Default max_crawl_depth
                pool,
            )
            .await
            {
                Ok(_) => {
                    info!(domain = %domain, "Created website from discovery");
                    websites_created += 1;
                }
                Err(e) => {
                    tracing::warn!(domain = %domain, error = %e, "Failed to create website");
                }
            }
        }
    }

    info!(
        queries_run = DISCOVERY_QUERIES.len(),
        total_results = total_results,
        websites_created = websites_created,
        "Discovery searches completed"
    );

    Ok(DiscoveryResult {
        queries_run: DISCOVERY_QUERIES.len(),
        total_results,
        websites_created,
    })
}

/// Extract domain from URL
fn extract_domain(url: &str) -> Option<String> {
    url::Url::parse(url).ok()?.host_str().map(|s| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_domain() {
        assert_eq!(
            extract_domain("https://example.com/path"),
            Some("example.com".to_string())
        );
        assert_eq!(
            extract_domain("https://www.example.org/page?q=1"),
            Some("www.example.org".to_string())
        );
        assert_eq!(extract_domain("not-a-url"), None);
    }

    #[test]
    fn test_query_substitution() {
        let query = "food bank {location}".replace("{location}", DEFAULT_LOCATION);
        assert_eq!(query, "food bank Twin Cities, Minnesota");
    }
}
