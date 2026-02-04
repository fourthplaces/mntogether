//! Robots.txt parser and checker.

use std::collections::HashMap;
use std::time::Duration;

/// Parsed robots.txt rules.
#[derive(Debug, Clone, Default)]
pub struct RobotsTxt {
    /// Rules per user-agent (lowercase)
    rules: HashMap<String, AgentRules>,

    /// Default rules (for *)
    default_rules: AgentRules,

    /// Crawl delay in seconds
    crawl_delay: Option<f64>,

    /// Sitemaps listed
    sitemaps: Vec<String>,
}

/// Rules for a specific user-agent.
#[derive(Debug, Clone, Default)]
pub struct AgentRules {
    /// Disallowed path prefixes
    disallow: Vec<String>,

    /// Allowed path prefixes (override disallow)
    allow: Vec<String>,

    /// Crawl delay for this agent
    crawl_delay: Option<f64>,
}

impl RobotsTxt {
    /// Parse robots.txt content.
    pub fn parse(content: &str) -> Self {
        let mut robots = Self::default();
        let mut current_agents: Vec<String> = Vec::new();
        let mut current_rules = AgentRules::default();

        for line in content.lines() {
            let line = line.trim();

            // Skip comments and empty lines
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Parse directive
            if let Some((directive, value)) = line.split_once(':') {
                let directive = directive.trim().to_lowercase();
                let value = value.trim();

                match directive.as_str() {
                    "user-agent" => {
                        // Save previous rules if any
                        if !current_agents.is_empty() {
                            for agent in &current_agents {
                                if agent == "*" {
                                    robots.default_rules = current_rules.clone();
                                } else {
                                    robots.rules.insert(agent.clone(), current_rules.clone());
                                }
                            }
                            current_rules = AgentRules::default();
                            current_agents.clear();
                        }

                        current_agents.push(value.to_lowercase());
                    }
                    "disallow" => {
                        if !value.is_empty() {
                            current_rules.disallow.push(value.to_string());
                        }
                    }
                    "allow" => {
                        if !value.is_empty() {
                            current_rules.allow.push(value.to_string());
                        }
                    }
                    "crawl-delay" => {
                        if let Ok(delay) = value.parse::<f64>() {
                            current_rules.crawl_delay = Some(delay);
                            if robots.crawl_delay.is_none() {
                                robots.crawl_delay = Some(delay);
                            }
                        }
                    }
                    "sitemap" => {
                        robots.sitemaps.push(value.to_string());
                    }
                    _ => {}
                }
            }
        }

        // Save final rules
        for agent in current_agents {
            if agent == "*" {
                robots.default_rules = current_rules.clone();
            } else {
                robots.rules.insert(agent, current_rules.clone());
            }
        }

        robots
    }

    /// Check if a path is allowed for a user-agent.
    pub fn is_allowed(&self, user_agent: &str, path: &str) -> bool {
        let agent_lower = user_agent.to_lowercase();

        // Find matching rules
        let rules = self
            .rules
            .get(&agent_lower)
            .or_else(|| {
                // Check for partial matches
                self.rules
                    .iter()
                    .find(|(k, _)| agent_lower.contains(k.as_str()))
                    .map(|(_, v)| v)
            })
            .unwrap_or(&self.default_rules);

        // Check allow rules first (they take precedence)
        for allow in &rules.allow {
            if path.starts_with(allow) {
                return true;
            }
        }

        // Check disallow rules
        for disallow in &rules.disallow {
            if disallow == "/" {
                return false; // Disallow all
            }
            if path.starts_with(disallow) {
                return false;
            }
        }

        true
    }

    /// Get crawl delay for a user-agent.
    pub fn crawl_delay(&self, user_agent: &str) -> Option<Duration> {
        let agent_lower = user_agent.to_lowercase();

        let delay = self
            .rules
            .get(&agent_lower)
            .and_then(|r| r.crawl_delay)
            .or(self.crawl_delay);

        delay.map(|d| Duration::from_secs_f64(d))
    }

    /// Get listed sitemaps.
    pub fn sitemaps(&self) -> &[String] {
        &self.sitemaps
    }

    /// Check if robots.txt disallows all crawling.
    pub fn disallows_all(&self, user_agent: &str) -> bool {
        !self.is_allowed(user_agent, "/")
    }
}

/// Fetch and parse robots.txt for a site.
pub async fn fetch_robots_txt(
    client: &reqwest::Client,
    site_url: &str,
) -> Result<RobotsTxt, reqwest::Error> {
    let url = format!("{}/robots.txt", site_url.trim_end_matches('/'));

    match client.get(&url).send().await {
        Ok(response) if response.status().is_success() => {
            let content = response.text().await?;
            Ok(RobotsTxt::parse(&content))
        }
        Ok(_) => {
            // No robots.txt or error - allow all
            Ok(RobotsTxt::default())
        }
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_basic() {
        let content = r#"
User-agent: *
Disallow: /private/
Disallow: /admin/
Allow: /public/
Crawl-delay: 2

Sitemap: https://example.com/sitemap.xml
        "#;

        let robots = RobotsTxt::parse(content);

        assert!(robots.is_allowed("TestBot", "/public/page"));
        assert!(!robots.is_allowed("TestBot", "/private/page"));
        assert!(!robots.is_allowed("TestBot", "/admin/"));
        assert!(robots.is_allowed("TestBot", "/other/page"));

        assert_eq!(robots.crawl_delay("TestBot"), Some(Duration::from_secs(2)));
        assert_eq!(robots.sitemaps().len(), 1);
    }

    #[test]
    fn test_specific_user_agent() {
        let content = r#"
User-agent: *
Disallow: /

User-agent: goodbot
Disallow:
Allow: /
        "#;

        let robots = RobotsTxt::parse(content);

        assert!(!robots.is_allowed("BadBot", "/page"));
        assert!(robots.is_allowed("GoodBot", "/page"));
    }

    #[test]
    fn test_allow_overrides_disallow() {
        let content = r#"
User-agent: *
Disallow: /private/
Allow: /private/public/
        "#;

        let robots = RobotsTxt::parse(content);

        assert!(!robots.is_allowed("Bot", "/private/secret"));
        assert!(robots.is_allowed("Bot", "/private/public/page"));
    }

    #[test]
    fn test_empty_robots() {
        let robots = RobotsTxt::parse("");

        assert!(robots.is_allowed("AnyBot", "/any/path"));
        assert!(robots.crawl_delay("AnyBot").is_none());
    }

    #[test]
    fn test_disallow_all() {
        let content = r#"
User-agent: *
Disallow: /
        "#;

        let robots = RobotsTxt::parse(content);

        assert!(robots.disallows_all("Bot"));
        assert!(!robots.is_allowed("Bot", "/anything"));
    }
}
