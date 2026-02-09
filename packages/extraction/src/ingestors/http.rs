//! HTTP-based ingestor implementation.
//!
//! Provides basic HTTP crawling with link following and rate limiting.

use async_trait::async_trait;
use chrono::Utc;
use std::collections::{HashMap, HashSet, VecDeque};
use tracing::{debug, info, warn};
use url::Url;

use crate::error::{CrawlError, CrawlResult};
use crate::traits::ingestor::{DiscoverConfig, Ingestor, RawPage};

/// HTTP ingestor that fetches pages via HTTP and follows links.
///
/// This is a basic implementation suitable for simple websites.
/// For JavaScript-heavy sites, use `FirecrawlIngestor` instead.
///
/// # Example
///
/// ```rust,ignore
/// use extraction::ingestors::{HttpIngestor, ValidatedIngestor, DiscoverConfig};
///
/// let ingestor = ValidatedIngestor::new(HttpIngestor::new());
/// let config = DiscoverConfig::new("https://example.com").with_limit(10);
/// let pages = ingestor.discover(&config).await?;
/// ```
pub struct HttpIngestor {
    client: reqwest::Client,
    user_agent: String,
    rate_limit_ms: u64,
}

impl Default for HttpIngestor {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpIngestor {
    /// Create a new HTTP ingestor with default settings.
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .expect("Failed to create HTTP client"),
            user_agent: "ExtractionBot/1.0".to_string(),
            rate_limit_ms: 100,
        }
    }

    /// Set a custom user agent.
    pub fn with_user_agent(mut self, user_agent: impl Into<String>) -> Self {
        self.user_agent = user_agent.into();
        self
    }

    /// Set a custom HTTP client.
    pub fn with_client(mut self, client: reqwest::Client) -> Self {
        self.client = client;
        self
    }

    /// Set rate limiting delay between requests (milliseconds).
    pub fn with_rate_limit(mut self, ms: u64) -> Self {
        self.rate_limit_ms = ms;
        self
    }

    /// Fetch a single URL and return a RawPage plus the raw HTML and final URL (after redirects).
    async fn fetch_url_with_html(&self, url: &str) -> CrawlResult<(RawPage, String, Url)> {
        debug!(url = %url, "HTTP fetch starting");
        let response = self
            .client
            .get(url)
            .header("User-Agent", &self.user_agent)
            .send()
            .await
            .map_err(|e| {
                warn!(url = %url, error = %e, "HTTP request failed");
                CrawlError::Http(Box::new(e))
            })?;

        let status = response.status();
        if !status.is_success() {
            return Err(CrawlError::Http(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("HTTP {}", status),
            ))));
        }

        // Capture final URL after redirects
        let final_url = response.url().clone();

        // Extract content type from headers
        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        // Collect headers as metadata
        let mut metadata: HashMap<String, String> = response
            .headers()
            .iter()
            .filter_map(|(k, v)| {
                v.to_str()
                    .ok()
                    .map(|v| (format!("http_{}", k.as_str()), v.to_string()))
            })
            .collect();

        metadata.insert("http_status".to_string(), status.as_u16().to_string());

        let html = response
            .text()
            .await
            .map_err(|e| CrawlError::Http(Box::new(e)))?;

        let title = self.extract_title(&html);
        let content = self.html_to_markdown(&html);

        let mut page = RawPage::new(url, content).with_fetched_at(Utc::now());

        if let Some(title) = title {
            page = page.with_title(title);
        }
        if let Some(ct) = content_type {
            page = page.with_content_type(ct);
        }
        page.metadata = metadata;

        Ok((page, html, final_url))
    }

    /// Extract links from HTML content.
    fn extract_links(&self, base_url: &Url, html: &str) -> Vec<String> {
        let mut links = Vec::new();

        // Match href attributes
        let href_pattern = regex::Regex::new(r#"href\s*=\s*["']([^"']+)["']"#).unwrap();

        for cap in href_pattern.captures_iter(html) {
            if let Some(href) = cap.get(1) {
                let href = href.as_str();

                // Skip anchors, javascript, mailto
                if href.starts_with('#')
                    || href.starts_with("javascript:")
                    || href.starts_with("mailto:")
                    || href.starts_with("tel:")
                {
                    continue;
                }

                // Resolve relative URLs
                if let Ok(resolved) = base_url.join(href) {
                    links.push(resolved.to_string());
                }
            }
        }

        links
    }

    /// Check if a URL should be crawled based on config.
    fn should_crawl(&self, url: &Url, base_url: &Url, config: &DiscoverConfig) -> bool {
        // Must be same host
        let base_host = base_url.host_str().unwrap_or("");
        let url_host = url.host_str().unwrap_or("");

        if url_host != base_host {
            return false;
        }

        // Check include patterns
        if !config.include_patterns.is_empty() {
            let path = url.path();
            let matches = config.include_patterns.iter().any(|p| path.contains(p));
            if !matches {
                return false;
            }
        }

        // Check exclude patterns
        if !config.exclude_patterns.is_empty() {
            let path = url.path();
            let excluded = config.exclude_patterns.iter().any(|p| path.contains(p));
            if excluded {
                return false;
            }
        }

        true
    }

    /// Convert HTML to markdown (simplified).
    fn html_to_markdown(&self, html: &str) -> String {
        let mut text = html.to_string();

        // Remove scripts and styles
        let script_pattern = regex::Regex::new(r"(?s)<script[^>]*>.*?</script>").unwrap();
        let style_pattern = regex::Regex::new(r"(?s)<style[^>]*>.*?</style>").unwrap();
        text = script_pattern.replace_all(&text, "").to_string();
        text = style_pattern.replace_all(&text, "").to_string();

        // Convert headers
        let h1_pattern = regex::Regex::new(r"<h1[^>]*>(.*?)</h1>").unwrap();
        let h2_pattern = regex::Regex::new(r"<h2[^>]*>(.*?)</h2>").unwrap();
        let h3_pattern = regex::Regex::new(r"<h3[^>]*>(.*?)</h3>").unwrap();
        text = h1_pattern.replace_all(&text, "# $1\n").to_string();
        text = h2_pattern.replace_all(&text, "## $1\n").to_string();
        text = h3_pattern.replace_all(&text, "### $1\n").to_string();

        // Convert paragraphs and line breaks
        let p_pattern = regex::Regex::new(r"<p[^>]*>(.*?)</p>").unwrap();
        let br_pattern = regex::Regex::new(r"<br\s*/?>").unwrap();
        text = p_pattern.replace_all(&text, "$1\n\n").to_string();
        text = br_pattern.replace_all(&text, "\n").to_string();

        // Convert links
        let link_pattern =
            regex::Regex::new(r#"<a[^>]*href=["']([^"']+)["'][^>]*>(.*?)</a>"#).unwrap();
        text = link_pattern.replace_all(&text, "[$2]($1)").to_string();

        // Convert lists
        let li_pattern = regex::Regex::new(r"<li[^>]*>(.*?)</li>").unwrap();
        text = li_pattern.replace_all(&text, "- $1\n").to_string();

        // Remove remaining tags
        let tag_pattern = regex::Regex::new(r"<[^>]+>").unwrap();
        text = tag_pattern.replace_all(&text, "").to_string();

        // Clean up whitespace
        let multi_newline = regex::Regex::new(r"\n{3,}").unwrap();
        text = multi_newline.replace_all(&text, "\n\n").to_string();

        // Decode HTML entities
        text = text
            .replace("&nbsp;", " ")
            .replace("&amp;", "&")
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("&#39;", "'");

        text.trim().to_string()
    }

    /// Extract title from HTML.
    fn extract_title(&self, html: &str) -> Option<String> {
        let title_pattern = regex::Regex::new(r"<title[^>]*>(.*?)</title>").ok()?;
        title_pattern
            .captures(html)
            .and_then(|cap| cap.get(1))
            .map(|m| m.as_str().trim().to_string())
    }
}

#[async_trait]
impl Ingestor for HttpIngestor {
    async fn discover(&self, config: &DiscoverConfig) -> CrawlResult<Vec<RawPage>> {
        info!(
            url = %config.url,
            limit = config.limit,
            max_depth = config.max_depth,
            "HttpIngestor.discover() starting"
        );

        let mut base_url = Url::parse(&config.url).map_err(|_| CrawlError::InvalidUrl {
            url: config.url.clone(),
        })?;

        let mut visited: HashSet<String> = HashSet::new();
        let mut queue: VecDeque<(String, usize)> = VecDeque::new();
        let mut pages: Vec<RawPage> = Vec::new();
        let mut base_resolved = false;

        queue.push_back((config.url.clone(), 0));
        debug!(url = %config.url, "Starting BFS crawl");

        while let Some((url, depth)) = queue.pop_front() {
            // Check limits
            if pages.len() >= config.limit {
                break;
            }
            if depth > config.max_depth {
                continue;
            }
            if visited.contains(&url) {
                continue;
            }

            visited.insert(url.clone());

            debug!(url = %url, depth = depth, pages_so_far = pages.len(), "Fetching page");

            // Fetch page (returns raw HTML + final URL after redirects)
            match self.fetch_url_with_html(&url).await {
                Ok((page, html, final_url)) => {
                    // On first fetch, update base_url to the final URL after redirects
                    // This handles cases like canmn.org -> www.canmn.org
                    if !base_resolved {
                        if final_url.host_str() != base_url.host_str() {
                            info!(
                                original = %base_url,
                                resolved = %final_url,
                                "Base URL resolved after redirect"
                            );
                            base_url = final_url.clone();
                        }
                        base_resolved = true;
                    }

                    debug!(
                        url = %url,
                        content_length = page.content.len(),
                        "Page fetched successfully"
                    );

                    // Extract links from the already-fetched HTML (no double-fetch)
                    if depth < config.max_depth {
                        let links = self.extract_links(&final_url, &html);
                        let new_links: Vec<_> = links
                            .into_iter()
                            .filter(|link| {
                                if let Ok(link_url) = Url::parse(link) {
                                    self.should_crawl(&link_url, &base_url, config)
                                        && !visited.contains(link)
                                } else {
                                    false
                                }
                            })
                            .collect();
                        debug!(url = %url, new_links_count = new_links.len(), "Extracted links");
                        for link in new_links {
                            queue.push_back((link, depth + 1));
                        }
                    }

                    pages.push(page);
                }
                Err(e) => {
                    warn!(url = %url, error = %e, "Failed to fetch page");
                }
            }

            // Rate limiting
            if self.rate_limit_ms > 0 {
                tokio::time::sleep(std::time::Duration::from_millis(self.rate_limit_ms)).await;
            }
        }

        info!(
            base_url = %config.url,
            pages_crawled = pages.len(),
            urls_visited = visited.len(),
            "HttpIngestor.discover() completed"
        );

        Ok(pages)
    }

    async fn fetch_specific(&self, urls: &[String]) -> CrawlResult<Vec<RawPage>> {
        let mut pages = Vec::with_capacity(urls.len());

        for url in urls {
            match self.fetch_url_with_html(url).await {
                Ok((page, _html, _final_url)) => pages.push(page),
                Err(e) => {
                    tracing::warn!("Failed to fetch {}: {}", url, e);
                }
            }

            // Rate limiting
            if self.rate_limit_ms > 0 {
                tokio::time::sleep(std::time::Duration::from_millis(self.rate_limit_ms)).await;
            }
        }

        Ok(pages)
    }

    fn name(&self) -> &str {
        "http"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_links() {
        let ingestor = HttpIngestor::new();
        let base_url = Url::parse("https://example.com/page").unwrap();

        let html = r##"
            <a href="/about">About</a>
            <a href="https://example.com/contact">Contact</a>
            <a href="#section">Anchor</a>
            <a href="javascript:void(0)">JS</a>
        "##;

        let links = ingestor.extract_links(&base_url, html);

        assert!(links.contains(&"https://example.com/about".to_string()));
        assert!(links.contains(&"https://example.com/contact".to_string()));
        assert!(!links.iter().any(|l| l.contains('#')));
        assert!(!links.iter().any(|l| l.contains("javascript")));
    }

    #[test]
    fn test_html_to_markdown() {
        let ingestor = HttpIngestor::new();

        let html = r#"
            <h1>Title</h1>
            <p>Paragraph text.</p>
            <a href="https://example.com">Link</a>
        "#;

        let md = ingestor.html_to_markdown(html);

        assert!(md.contains("# Title"));
        assert!(md.contains("Paragraph text."));
        assert!(md.contains("[Link](https://example.com)"));
    }

    #[test]
    fn test_extract_title() {
        let ingestor = HttpIngestor::new();

        let html = "<html><head><title>Page Title</title></head></html>";
        assert_eq!(ingestor.extract_title(html), Some("Page Title".to_string()));

        let html_no_title = "<html><body>No title</body></html>";
        assert_eq!(ingestor.extract_title(html_no_title), None);
    }

    #[test]
    fn test_should_crawl_same_host() {
        let ingestor = HttpIngestor::new();
        let base = Url::parse("https://example.com").unwrap();
        let config = DiscoverConfig::new("https://example.com");

        let same_host = Url::parse("https://example.com/page").unwrap();
        let different_host = Url::parse("https://other.com/page").unwrap();

        assert!(ingestor.should_crawl(&same_host, &base, &config));
        assert!(!ingestor.should_crawl(&different_host, &base, &config));
    }

    #[test]
    fn test_should_crawl_with_patterns() {
        let ingestor = HttpIngestor::new();
        let base = Url::parse("https://example.com").unwrap();

        let config = DiscoverConfig::new("https://example.com")
            .include("/blog")
            .exclude("/admin");

        let blog = Url::parse("https://example.com/blog/post").unwrap();
        let admin = Url::parse("https://example.com/admin/settings").unwrap();
        let other = Url::parse("https://example.com/about").unwrap();

        assert!(ingestor.should_crawl(&blog, &base, &config));
        assert!(!ingestor.should_crawl(&admin, &base, &config));
        assert!(!ingestor.should_crawl(&other, &base, &config)); // Not in include pattern
    }
}
