//! Crawler trait and URL validation for SSRF protection.

use async_trait::async_trait;
use std::collections::HashSet;
use std::net::IpAddr;

use crate::error::{CrawlError, CrawlResult, SecurityError, SecurityResult};
use crate::types::{config::CrawlConfig, page::CrawledPage};

/// Crawler trait for fetching pages from websites.
#[async_trait]
pub trait Crawler: Send + Sync {
    /// Discover and fetch pages from a site.
    async fn crawl(&self, config: &CrawlConfig) -> CrawlResult<Vec<CrawledPage>>;

    /// Fetch a single page by URL.
    async fn fetch(&self, url: &str) -> CrawlResult<CrawledPage>;

    /// Fetch multiple pages by URL.
    async fn fetch_pages(&self, urls: &[&str]) -> CrawlResult<Vec<CrawledPage>> {
        let mut pages = Vec::with_capacity(urls.len());
        for url in urls {
            match self.fetch(url).await {
                Ok(page) => pages.push(page),
                Err(e) => {
                    tracing::warn!("Failed to fetch {}: {}", url, e);
                }
            }
        }
        Ok(pages)
    }
}

/// URL validator for SSRF protection.
///
/// Validates URLs before crawling to prevent:
/// - Access to internal services (localhost, 127.0.0.1)
/// - Access to private IP ranges (10.x, 172.16.x, 192.168.x)
/// - Access to cloud metadata services (169.254.x)
/// - Non-HTTP(S) schemes (file://, ftp://)
#[derive(Debug, Clone)]
pub struct UrlValidator {
    /// Allowed URL schemes
    allowed_schemes: HashSet<String>,

    /// Blocked hostnames
    blocked_hosts: HashSet<String>,

    /// Blocked CIDR ranges
    blocked_cidrs: Vec<ipnet::IpNet>,

    /// Additional allowed hosts (bypass normal validation)
    allowed_hosts: HashSet<String>,
}

impl Default for UrlValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl UrlValidator {
    /// Create a new URL validator with default security rules.
    pub fn new() -> Self {
        Self {
            allowed_schemes: ["http", "https"]
                .into_iter()
                .map(String::from)
                .collect(),
            blocked_hosts: [
                "localhost",
                "127.0.0.1",
                "::1",
                "[::1]",
                "0.0.0.0",
                "metadata.google.internal",
                "metadata.gke.internal",
                "instance-data",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
            blocked_cidrs: vec![
                "10.0.0.0/8".parse().unwrap(),
                "172.16.0.0/12".parse().unwrap(),
                "192.168.0.0/16".parse().unwrap(),
                "169.254.0.0/16".parse().unwrap(), // Link-local / cloud metadata
                "127.0.0.0/8".parse().unwrap(),    // Loopback
                "::1/128".parse().unwrap(),        // IPv6 loopback
                "fc00::/7".parse().unwrap(),       // IPv6 private
                "fe80::/10".parse().unwrap(),      // IPv6 link-local
            ],
            allowed_hosts: HashSet::new(),
        }
    }

    /// Add an allowed host (bypasses validation).
    pub fn allow_host(mut self, host: impl Into<String>) -> Self {
        self.allowed_hosts.insert(host.into());
        self
    }

    /// Block an additional host.
    pub fn block_host(mut self, host: impl Into<String>) -> Self {
        self.blocked_hosts.insert(host.into());
        self
    }

    /// Block an additional CIDR range.
    pub fn block_cidr(mut self, cidr: ipnet::IpNet) -> Self {
        self.blocked_cidrs.push(cidr);
        self
    }

    /// Validate a URL for safety.
    pub fn validate(&self, url: &str) -> SecurityResult<()> {
        let parsed = url::Url::parse(url)?;

        // Check scheme
        if !self.allowed_schemes.contains(parsed.scheme()) {
            return Err(SecurityError::DisallowedScheme(parsed.scheme().to_string()));
        }

        // Get host
        let host = parsed.host_str().ok_or(SecurityError::NoHost)?;

        // Check allowed hosts first (bypass other checks)
        if self.allowed_hosts.contains(host) {
            return Ok(());
        }

        // Check blocked hosts
        if self.blocked_hosts.contains(host) {
            return Err(SecurityError::BlockedHost(host.to_string()));
        }

        // Check blocked CIDRs for IP addresses
        if let Ok(ip) = host.parse::<IpAddr>() {
            for cidr in &self.blocked_cidrs {
                if cidr.contains(&ip) {
                    return Err(SecurityError::BlockedCidr(ip.to_string()));
                }
            }
        }

        Ok(())
    }

    /// Validate a URL and resolve DNS to check the actual IP.
    ///
    /// This catches DNS rebinding attacks where a hostname resolves
    /// to an internal IP.
    pub async fn validate_with_dns(&self, url: &str) -> SecurityResult<()> {
        // First do basic validation
        self.validate(url)?;

        let parsed = url::Url::parse(url)?;
        let host = parsed.host_str().ok_or(SecurityError::NoHost)?;

        // Skip DNS check for allowed hosts
        if self.allowed_hosts.contains(host) {
            return Ok(());
        }

        // Skip DNS check for IP addresses (already checked in validate)
        if host.parse::<IpAddr>().is_ok() {
            return Ok(());
        }

        // Resolve DNS and check IPs
        let port = parsed.port().unwrap_or(match parsed.scheme() {
            "https" => 443,
            _ => 80,
        });

        let addrs = tokio::net::lookup_host(format!("{}:{}", host, port))
            .await
            .map_err(|e| SecurityError::DnsResolution(e.to_string()))?;

        for addr in addrs {
            let ip = addr.ip();
            for cidr in &self.blocked_cidrs {
                if cidr.contains(&ip) {
                    return Err(SecurityError::BlockedCidr(format!(
                        "DNS for {} resolved to blocked IP {}",
                        host, ip
                    )));
                }
            }
        }

        Ok(())
    }
}

/// A crawler that validates URLs before fetching.
pub struct ValidatedCrawler<C: Crawler> {
    inner: C,
    validator: UrlValidator,
}

impl<C: Crawler> ValidatedCrawler<C> {
    /// Create a new validated crawler.
    pub fn new(crawler: C) -> Self {
        Self {
            inner: crawler,
            validator: UrlValidator::new(),
        }
    }

    /// Create with a custom validator.
    pub fn with_validator(crawler: C, validator: UrlValidator) -> Self {
        Self {
            inner: crawler,
            validator,
        }
    }
}

#[async_trait]
impl<C: Crawler> Crawler for ValidatedCrawler<C> {
    async fn crawl(&self, config: &CrawlConfig) -> CrawlResult<Vec<CrawledPage>> {
        // Validate the starting URL
        self.validator
            .validate_with_dns(&config.url)
            .await
            .map_err(CrawlError::Security)?;

        // Crawl and filter results
        let pages = self.inner.crawl(config).await?;

        // Validate each crawled URL (in case of redirects)
        let validated: Vec<_> = pages
            .into_iter()
            .filter(|p| self.validator.validate(&p.url).is_ok())
            .collect();

        Ok(validated)
    }

    async fn fetch(&self, url: &str) -> CrawlResult<CrawledPage> {
        self.validator
            .validate_with_dns(url)
            .await
            .map_err(CrawlError::Security)?;

        self.inner.fetch(url).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blocks_localhost() {
        let validator = UrlValidator::new();
        assert!(validator.validate("http://localhost/").is_err());
        assert!(validator.validate("http://127.0.0.1/").is_err());
        assert!(validator.validate("http://[::1]/").is_err());
    }

    #[test]
    fn test_blocks_private_ips() {
        let validator = UrlValidator::new();
        assert!(validator.validate("http://10.0.0.1/").is_err());
        assert!(validator.validate("http://172.16.0.1/").is_err());
        assert!(validator.validate("http://192.168.1.1/").is_err());
    }

    #[test]
    fn test_blocks_metadata_services() {
        let validator = UrlValidator::new();
        assert!(validator.validate("http://169.254.169.254/").is_err());
        assert!(validator.validate("http://metadata.google.internal/").is_err());
    }

    #[test]
    fn test_blocks_non_http() {
        let validator = UrlValidator::new();
        assert!(validator.validate("file:///etc/passwd").is_err());
        assert!(validator.validate("ftp://example.com/").is_err());
    }

    #[test]
    fn test_allows_public_urls() {
        let validator = UrlValidator::new();
        assert!(validator.validate("https://example.com/").is_ok());
        assert!(validator.validate("http://google.com/").is_ok());
    }

    #[test]
    fn test_allowed_hosts_bypass() {
        let validator = UrlValidator::new().allow_host("localhost");
        assert!(validator.validate("http://localhost/").is_ok());
    }
}
