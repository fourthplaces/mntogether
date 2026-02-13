//! Configuration types for extraction and crawling.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Configuration for the extraction pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionConfig {
    /// Maximum summaries to send to LLM for partitioning.
    ///
    /// For sites with many pages, embedding search finds top N
    /// before LLM partitioning. Default: 50.
    pub max_summaries_for_partition: usize,

    /// Discard claims with "Assumed" grounding (no evidence).
    ///
    /// When true: only Direct and Inferred claims kept.
    /// When false: Assumed claims included but may be hallucinations.
    ///
    /// Default: true (recommended for accuracy).
    pub strict_mode: bool,

    /// Output language for summaries and extractions.
    ///
    /// If set, all output will be in this language regardless of
    /// source language. If None, preserves source language.
    pub output_language: Option<String>,

    /// Hints for extraction (e.g., ["title", "date", "location"]).
    ///
    /// Optional focus areas to help the LLM extract specific fields.
    #[serde(default)]
    pub hints: Vec<String>,

    /// Enable conflict detection across sources.
    ///
    /// Default: true.
    pub detect_conflicts: bool,

    /// Enable hybrid recall (semantic + BM25).
    ///
    /// Default: true.
    pub hybrid_recall: bool,

    /// Weight for semantic search in hybrid recall (0.0 to 1.0).
    ///
    /// The remaining weight goes to BM25 keyword search.
    /// Default: 0.6 (semantic-weighted).
    pub semantic_weight: f32,
}

impl Default for ExtractionConfig {
    fn default() -> Self {
        Self {
            max_summaries_for_partition: 50,
            strict_mode: true,
            output_language: None,
            hints: vec![],
            detect_conflicts: true,
            hybrid_recall: true,
            semantic_weight: 0.6,
        }
    }
}

impl ExtractionConfig {
    /// Create a new config with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set strict mode.
    pub fn with_strict_mode(mut self, strict: bool) -> Self {
        self.strict_mode = strict;
        self
    }

    /// Set output language.
    pub fn with_output_language(mut self, language: impl Into<String>) -> Self {
        self.output_language = Some(language.into());
        self
    }

    /// Add extraction hints.
    pub fn with_hints(mut self, hints: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.hints = hints.into_iter().map(|h| h.into()).collect();
        self
    }

    /// Set max summaries for partition.
    pub fn with_max_summaries(mut self, max: usize) -> Self {
        self.max_summaries_for_partition = max;
        self
    }
}

/// Filter for scoping queries to specific sites or time ranges.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QueryFilter {
    /// Only include pages from these sites (empty = all sites).
    #[serde(default)]
    pub include_sites: Vec<String>,

    /// Exclude pages from these sites.
    #[serde(default)]
    pub exclude_sites: Vec<String>,

    /// Only pages fetched after this date.
    pub min_date: Option<DateTime<Utc>>,

    /// Only pages fetched before this date.
    pub max_date: Option<DateTime<Utc>>,
}

impl QueryFilter {
    /// Create a new empty filter (matches all).
    pub fn new() -> Self {
        Self::default()
    }

    /// Filter to include only specific sites.
    pub fn for_sites(sites: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            include_sites: sites.into_iter().map(|s| s.into()).collect(),
            ..Default::default()
        }
    }

    /// Filter to a single site.
    pub fn for_site(site: impl Into<String>) -> Self {
        Self::for_sites([site])
    }

    /// Exclude specific sites.
    pub fn excluding(sites: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            exclude_sites: sites.into_iter().map(|s| s.into()).collect(),
            ..Default::default()
        }
    }

    /// Set minimum date.
    pub fn with_min_date(mut self, date: DateTime<Utc>) -> Self {
        self.min_date = Some(date);
        self
    }

    /// Set maximum date.
    pub fn with_max_date(mut self, date: DateTime<Utc>) -> Self {
        self.max_date = Some(date);
        self
    }

    /// Check if a site URL matches this filter.
    pub fn matches_site(&self, site_url: &str) -> bool {
        // Check exclusions first
        if self.exclude_sites.iter().any(|s| site_url.contains(s)) {
            return false;
        }

        // If include list is empty, include all
        if self.include_sites.is_empty() {
            return true;
        }

        // Otherwise, must be in include list
        self.include_sites.iter().any(|s| site_url.contains(s))
    }

    /// Check if a date matches this filter.
    pub fn matches_date(&self, date: DateTime<Utc>) -> bool {
        if let Some(min) = self.min_date {
            if date < min {
                return false;
            }
        }
        if let Some(max) = self.max_date {
            if date > max {
                return false;
            }
        }
        true
    }
}

/// Configuration for crawl operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawlConfig {
    /// Starting URL to crawl
    pub url: String,

    /// Maximum number of pages to crawl
    pub max_pages: usize,

    /// Maximum depth to crawl (0 = only starting page)
    pub max_depth: usize,

    /// Delay between requests in milliseconds
    pub rate_limit_ms: u64,

    /// Respect robots.txt
    pub respect_robots: bool,

    /// Follow links to subdomains
    pub follow_subdomains: bool,

    /// URL patterns to include (regex)
    #[serde(default)]
    pub include_patterns: Vec<String>,

    /// URL patterns to exclude (regex)
    #[serde(default)]
    pub exclude_patterns: Vec<String>,
}

impl Default for CrawlConfig {
    fn default() -> Self {
        Self {
            url: String::new(),
            max_pages: 100,
            max_depth: 3,
            rate_limit_ms: 1000,
            respect_robots: true,
            follow_subdomains: false,
            include_patterns: vec![],
            exclude_patterns: vec![],
        }
    }
}

impl CrawlConfig {
    /// Create a new crawl config for a URL.
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            ..Default::default()
        }
    }

    /// Set maximum pages.
    pub fn with_max_pages(mut self, max: usize) -> Self {
        self.max_pages = max;
        self
    }

    /// Set maximum depth.
    pub fn with_max_depth(mut self, depth: usize) -> Self {
        self.max_depth = depth;
        self
    }

    /// Set rate limit.
    pub fn with_rate_limit_ms(mut self, ms: u64) -> Self {
        self.rate_limit_ms = ms;
        self
    }

    /// Disable robots.txt respect.
    pub fn ignore_robots(mut self) -> Self {
        self.respect_robots = false;
        self
    }

    /// Enable subdomain following.
    pub fn with_subdomains(mut self) -> Self {
        self.follow_subdomains = true;
        self
    }

    /// Add an include pattern.
    pub fn include(mut self, pattern: impl Into<String>) -> Self {
        self.include_patterns.push(pattern.into());
        self
    }

    /// Add an exclude pattern.
    pub fn exclude(mut self, pattern: impl Into<String>) -> Self {
        self.exclude_patterns.push(pattern.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_filter_matches_site() {
        let filter = QueryFilter::for_site("example.com");
        assert!(filter.matches_site("https://example.com/page"));
        assert!(!filter.matches_site("https://other.com/page"));

        let exclude = QueryFilter::excluding(["spam.com"]);
        assert!(exclude.matches_site("https://example.com"));
        assert!(!exclude.matches_site("https://spam.com/page"));
    }

    #[test]
    fn test_query_filter_matches_date() {
        let now = Utc::now();
        let yesterday = now - chrono::Duration::days(1);
        let tomorrow = now + chrono::Duration::days(1);

        let filter = QueryFilter::new().with_min_date(yesterday);
        assert!(filter.matches_date(now));
        assert!(!filter.matches_date(yesterday - chrono::Duration::hours(1)));

        let filter2 = QueryFilter::new().with_max_date(yesterday);
        assert!(!filter2.matches_date(tomorrow));
    }
}
