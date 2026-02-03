//! Simple web scraper - replaces Firecrawl with local HTTP + HTML parsing
//!
//! This implementation:
//! - Uses reqwest for HTTP requests
//! - Uses scraper crate for HTML parsing
//! - Uses htmd for HTML to Markdown conversion
//! - Discovers links for multi-page crawling
//!
//! Benefits:
//! - No API costs (Firecrawl charges per page)
//! - Full control over scraping behavior
//! - Faster (no external API round-trips)
//!
//! Limitations:
//! - No JavaScript rendering (use for static HTML sites only)

use anyhow::{Context, Result};
use async_trait::async_trait;
use scraper::{Html, Selector};
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;
use tracing::{debug, info, warn};
use url::Url;

use super::{BaseWebScraper, CrawlResult, CrawledPage, LinkPriorities, ScrapeResult};

/// Maximum concurrent requests during crawl
const MAX_CONCURRENT_REQUESTS: usize = 5;

/// Simple web scraper using reqwest + scraper + htmd
pub struct SimpleScraper {
    client: reqwest::Client,
}

impl SimpleScraper {
    pub fn new() -> Result<Self> {
        // Use a browser-like User-Agent to avoid bot detection
        let user_agent = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";

        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::ACCEPT,
            "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8"
                .parse()
                .unwrap(),
        );
        headers.insert(
            reqwest::header::ACCEPT_LANGUAGE,
            "en-US,en;q=0.5".parse().unwrap(),
        );
        headers.insert(reqwest::header::CONNECTION, "keep-alive".parse().unwrap());
        headers.insert(
            reqwest::header::UPGRADE_INSECURE_REQUESTS,
            "1".parse().unwrap(),
        );

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent(user_agent)
            .default_headers(headers)
            .redirect(reqwest::redirect::Policy::limited(5))
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self { client })
    }

    /// Fetch raw HTML from a URL
    async fn fetch_html(&self, url: &str) -> Result<String> {
        let response = self
            .client
            .get(url)
            .send()
            .await
            .context("HTTP request failed")?;

        let status = response.status();
        if !status.is_success() {
            anyhow::bail!("HTTP {} for {}", status, url);
        }

        response
            .text()
            .await
            .context("Failed to read response body")
    }

    /// Extract title from HTML document
    fn extract_title(document: &Html) -> Option<String> {
        let title_selector = Selector::parse("title").ok()?;
        document
            .select(&title_selector)
            .next()
            .map(|el| el.text().collect::<String>().trim().to_string())
            .filter(|t| !t.is_empty())
    }

    /// Extract main content HTML, stripping nav/header/footer/aside
    fn extract_main_content(document: &Html) -> String {
        // Try to find main content area
        let main_selectors = [
            "main",
            "article",
            "[role='main']",
            "#content",
            "#main",
            ".content",
            ".main",
            ".post-content",
            ".entry-content",
        ];

        for selector_str in main_selectors {
            if let Ok(selector) = Selector::parse(selector_str) {
                if let Some(main) = document.select(&selector).next() {
                    return main.html();
                }
            }
        }

        // Fallback: use body but remove unwanted elements
        if let Ok(body_selector) = Selector::parse("body") {
            if let Some(body) = document.select(&body_selector).next() {
                let html = body.html();
                return Self::remove_boilerplate(&html);
            }
        }

        // Last resort: return entire document
        document.html()
    }

    /// Remove common boilerplate elements from HTML string
    fn remove_boilerplate(html: &str) -> String {
        // Parse and remove unwanted elements
        let document = Html::parse_document(html);
        let unwanted = [
            "nav",
            "header",
            "footer",
            "aside",
            ".nav",
            ".navbar",
            ".header",
            ".footer",
            ".sidebar",
            ".menu",
            ".advertisement",
            ".ads",
            "#nav",
            "#header",
            "#footer",
            "#sidebar",
            "script",
            "style",
            "noscript",
            "iframe",
        ];

        let mut result = html.to_string();
        for selector_str in unwanted {
            if let Ok(selector) = Selector::parse(selector_str) {
                for element in document.select(&selector) {
                    let element_html = element.html();
                    result = result.replace(&element_html, "");
                }
            }
        }

        result
    }

    /// Convert HTML to Markdown
    fn html_to_markdown(html: &str) -> String {
        htmd::convert(html).unwrap_or_else(|_| {
            // Fallback: strip tags and return plain text
            let document = Html::parse_document(html);
            document.root_element().text().collect::<String>()
        })
    }

    /// Extract same-domain links from HTML
    fn extract_links(document: &Html, base_url: &Url) -> Vec<String> {
        let link_selector = match Selector::parse("a[href]") {
            Ok(s) => s,
            Err(_) => return vec![],
        };

        let base_domain = base_url.domain().unwrap_or("");

        document
            .select(&link_selector)
            .filter_map(|el| el.value().attr("href"))
            .filter_map(|href| {
                // Resolve relative URLs
                base_url.join(href).ok()
            })
            .filter(|url| {
                // Same domain only
                url.domain() == Some(base_domain)
                    // HTTP/HTTPS only
                    && (url.scheme() == "http" || url.scheme() == "https")
                    // Skip fragments and common non-content paths
                    && url.fragment().is_none()
                    && !Self::is_skip_path(url.path())
            })
            .map(|url| {
                // Normalize: remove query params and trailing slash
                let mut normalized = url.clone();
                normalized.set_query(None);
                let path = normalized.path().trim_end_matches('/').to_string();
                normalized.set_path(if path.is_empty() { "/" } else { &path });
                normalized.to_string()
            })
            .collect()
    }

    /// Normalize URL by adding https:// if no scheme is present
    fn normalize_url(url: &str) -> String {
        if url.starts_with("http://") || url.starts_with("https://") {
            url.to_string()
        } else {
            format!("https://{}", url)
        }
    }

    /// Check if a path should be skipped (assets, auth, etc.)
    fn is_skip_path(path: &str) -> bool {
        let skip_patterns = [
            "/wp-admin",
            "/wp-login",
            "/wp-content/uploads",
            "/login",
            "/logout",
            "/signin",
            "/signout",
            "/auth",
            "/api/",
            "/cdn-cgi/",
            "/feed",
            "/rss",
            "/sitemap",
            ".pdf",
            ".jpg",
            ".jpeg",
            ".png",
            ".gif",
            ".svg",
            ".css",
            ".js",
            ".xml",
            ".json",
        ];

        let path_lower = path.to_lowercase();
        skip_patterns
            .iter()
            .any(|pattern| path_lower.contains(pattern))
    }
}

impl Default for SimpleScraper {
    fn default() -> Self {
        Self::new().expect("Failed to create SimpleScraper")
    }
}

#[async_trait]
impl BaseWebScraper for SimpleScraper {
    async fn scrape(&self, url: &str) -> Result<ScrapeResult> {
        let url = Self::normalize_url(url);
        debug!(url = %url, "Scraping page");

        let html = self.fetch_html(&url).await?;
        let document = Html::parse_document(&html);

        let title = Self::extract_title(&document);
        let main_content = Self::extract_main_content(&document);
        let markdown = Self::html_to_markdown(&main_content);

        // Skip if no meaningful content
        if markdown.trim().len() < 100 {
            warn!(url = %url, "Page has minimal content");
        }

        Ok(ScrapeResult {
            url: url.to_string(),
            markdown,
            title,
        })
    }

    async fn crawl(
        &self,
        url: &str,
        max_depth: i32,
        max_pages: i32,
        delay_seconds: i32,
        priorities: Option<&LinkPriorities>,
    ) -> Result<CrawlResult> {
        let url = Self::normalize_url(url);
        let has_priorities = priorities.map(|p| !p.is_empty()).unwrap_or(false);
        info!(
            url = %url,
            max_depth = %max_depth,
            max_pages = %max_pages,
            delay_seconds = %delay_seconds,
            concurrency = MAX_CONCURRENT_REQUESTS,
            has_priorities = has_priorities,
            "Starting parallel crawl"
        );

        let base_url = Url::parse(&url).context("Invalid start URL")?;
        let delay = Duration::from_millis((delay_seconds as u64) * 200); // Shorter delay with concurrency

        // Semaphore to limit concurrent requests
        let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_REQUESTS));

        // Track visited URLs and collected pages
        let mut visited: HashSet<String> = HashSet::new();
        let mut pages: Vec<CrawledPage> = Vec::new();

        // Normalize start URL
        let start_url = {
            let mut u = base_url.clone();
            u.set_query(None);
            u.set_fragment(None);
            u.to_string()
        };

        // Process level by level (BFS with parallel fetching per level)
        let mut current_level: Vec<String> = vec![start_url.clone()];
        visited.insert(start_url);

        for depth in 0..=max_depth {
            if current_level.is_empty() || pages.len() >= max_pages as usize {
                break;
            }

            info!(
                depth = depth,
                urls_at_level = current_level.len(),
                pages_so_far = pages.len(),
                "Processing depth level"
            );

            // Fetch all URLs at this level in parallel
            let mut handles = Vec::new();

            for url_to_fetch in current_level.iter().cloned() {
                // Check if we've hit max pages
                if pages.len() + handles.len() >= max_pages as usize {
                    break;
                }

                let client = self.client.clone();
                let sem = semaphore.clone();
                let delay_clone = delay;

                let handle = tokio::spawn(async move {
                    // Acquire semaphore permit
                    let _permit = sem.acquire().await.ok()?;

                    // Small delay to be polite to servers
                    if delay_clone.as_millis() > 0 {
                        tokio::time::sleep(delay_clone).await;
                    }

                    // Fetch the page
                    debug!(url = %url_to_fetch, "Fetching page");
                    let response = client.get(&url_to_fetch).send().await.ok()?;
                    if !response.status().is_success() {
                        warn!(url = %url_to_fetch, status = %response.status(), "HTTP error");
                        return None;
                    }
                    let html = response.text().await.ok()?;

                    Some((url_to_fetch, html))
                });

                handles.push(handle);
            }

            // Collect results and discover new links
            let mut next_level: Vec<String> = Vec::new();

            for handle in handles {
                if let Ok(Some((fetched_url, html))) = handle.await {
                    let document = Html::parse_document(&html);
                    let title = Self::extract_title(&document);
                    let main_content = Self::extract_main_content(&document);
                    let markdown = Self::html_to_markdown(&main_content);

                    // Skip pages with minimal content
                    if markdown.trim().len() < 50 {
                        debug!(url = %fetched_url, "Skipping page with minimal content");
                        continue;
                    }

                    pages.push(CrawledPage {
                        url: fetched_url,
                        markdown,
                        title,
                    });

                    // Discover links for next depth level
                    if depth < max_depth {
                        let mut links = Self::extract_links(&document, &base_url);

                        // Apply priorities: filter skips and sort by score
                        if let Some(prio) = priorities {
                            links.retain(|link| !prio.should_skip(link));
                            links.sort_by_cached_key(|link| {
                                std::cmp::Reverse(prio.score_path(link))
                            });
                        }

                        for link in links {
                            if !visited.contains(&link) {
                                visited.insert(link.clone());
                                next_level.push(link);
                            }
                        }
                    }

                    // Check max pages limit
                    if pages.len() >= max_pages as usize {
                        info!("Reached max_pages limit ({})", max_pages);
                        break;
                    }
                }
            }

            current_level = next_level;
        }

        info!(
            url = %url,
            pages_crawled = %pages.len(),
            urls_discovered = %visited.len(),
            "Parallel crawl completed"
        );

        Ok(CrawlResult { pages })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_skip_path() {
        assert!(SimpleScraper::is_skip_path("/wp-admin/"));
        assert!(SimpleScraper::is_skip_path("/image.jpg"));
        assert!(SimpleScraper::is_skip_path("/api/users"));
        assert!(!SimpleScraper::is_skip_path("/about"));
        assert!(!SimpleScraper::is_skip_path("/services/help"));
    }

    #[test]
    fn test_extract_title() {
        let html = r#"<html><head><title>Test Page</title></head><body></body></html>"#;
        let document = Html::parse_document(html);
        assert_eq!(
            SimpleScraper::extract_title(&document),
            Some("Test Page".to_string())
        );
    }

    #[test]
    fn test_html_to_markdown() {
        let html = "<h1>Hello</h1><p>World</p>";
        let md = SimpleScraper::html_to_markdown(html);
        assert!(md.contains("Hello"));
        assert!(md.contains("World"));
    }

    #[test]
    fn test_normalize_url() {
        assert_eq!(
            SimpleScraper::normalize_url("example.com"),
            "https://example.com"
        );
        assert_eq!(
            SimpleScraper::normalize_url("https://example.com"),
            "https://example.com"
        );
        assert_eq!(
            SimpleScraper::normalize_url("http://example.com"),
            "http://example.com"
        );
    }
}
