//! Tavily-powered page discovery
//!
//! Uses site-scoped search queries to find relevant pages on a website,
//! instead of traditional link-following crawl.

use std::collections::HashSet;

use anyhow::Result;
use tracing::info;

use extraction::WebSearcher;

/// A page discovered via search
#[derive(Debug, Clone)]
pub struct DiscoveredPage {
    pub url: String,
    pub title: String,
    pub content: String,
    pub relevance_score: f64,
    pub query_matched: String,
}

/// Discovery queries for finding community resource content
const DISCOVERY_QUERIES: &[&str] = &[
    "volunteer opportunities",
    "donate donation giving",
    "services programs",
    "food pantry meals",
    "housing shelter",
    "help resources assistance",
    "events calendar",
    "get involved",
];

/// Discover relevant pages on a domain using search
///
/// Runs multiple site-scoped queries and dedupes results.
/// Returns pages sorted by relevance score.
pub async fn discover_pages(
    domain: &str,
    web_searcher: &dyn WebSearcher,
    max_pages: usize,
) -> Result<Vec<DiscoveredPage>> {
    let mut all_results: Vec<DiscoveredPage> = Vec::new();
    let mut seen_urls: HashSet<String> = HashSet::new();

    for query_terms in DISCOVERY_QUERIES {
        let query = format!("site:{} {}", domain, query_terms);

        info!(
            domain = %domain,
            query = %query,
            "Running discovery search"
        );

        let results = web_searcher.search_with_limit(&query, 5).await?;

        info!(
            domain = %domain,
            query = %query,
            results_count = results.len(),
            "Discovery search returned results"
        );

        // Log each discovered URL
        for result in &results {
            info!(
                url = %result.url,
                title = ?result.title,
                score = ?result.score,
                snippet_preview = ?result.snippet.as_ref().map(|s| s.chars().take(100).collect::<String>()),
                "Discovered page"
            );
        }

        for result in results {
            // Normalize URL for deduplication
            let normalized_url = normalize_url(result.url.as_str());

            if seen_urls.insert(normalized_url.clone()) {
                all_results.push(DiscoveredPage {
                    url: result.url.to_string(),
                    title: result.title.unwrap_or_default(),
                    content: result.snippet.unwrap_or_default(),
                    relevance_score: result.score.unwrap_or(0.0) as f64,
                    query_matched: query_terms.to_string(),
                });
            }
        }
    }

    // Sort by relevance score (highest first)
    all_results.sort_by(|a, b| {
        b.relevance_score
            .partial_cmp(&a.relevance_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Limit to max pages
    all_results.truncate(max_pages);

    // Log final list of all discovered URLs
    info!(
        domain = %domain,
        total_discovered = all_results.len(),
        "Discovery complete - final URLs:"
    );
    for (i, page) in all_results.iter().enumerate() {
        info!(
            rank = i + 1,
            url = %page.url,
            title = %page.title,
            score = %page.relevance_score,
            matched_query = %page.query_matched,
            "Final discovered page"
        );
    }

    Ok(all_results)
}

/// Normalize URL for deduplication (remove trailing slash, fragments, etc.)
fn normalize_url(url: &str) -> String {
    let url = url.trim_end_matches('/');
    // Remove fragment
    let url = url.split('#').next().unwrap_or(url);
    // Remove common tracking params
    let url = url.split('?').next().unwrap_or(url);
    url.to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_url() {
        assert_eq!(
            normalize_url("https://example.org/page/"),
            "https://example.org/page"
        );
        assert_eq!(
            normalize_url("https://example.org/page#section"),
            "https://example.org/page"
        );
        assert_eq!(
            normalize_url("https://example.org/page?utm_source=test"),
            "https://example.org/page"
        );
        assert_eq!(
            normalize_url("HTTPS://Example.Org/Page"),
            "https://example.org/page"
        );
    }
}
