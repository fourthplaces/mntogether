//! Organization extraction from crawled website pages
//!
//! After a website is crawled, this activity extracts organization-level info
//! (name, description, social media links) from the already-fetched pages.

use anyhow::Result;
use regex::Regex;
use schemars::JsonSchema;
use serde::Deserialize;
use std::sync::LazyLock;
use tracing::{info, warn};
use uuid::Uuid;

use crate::common::{OrganizationId, WebsiteId};
use crate::domains::crawling::models::ExtractionPage;
use crate::domains::organization::models::Organization;
use crate::domains::social_profile::models::SocialProfile;
use crate::domains::website::models::Website;
use crate::kernel::ServerDeps;

// =============================================================================
// LLM Response Types
// =============================================================================

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct ExtractedOrganization {
    /// The organization's official name
    pub name: String,
    /// Brief description or mission statement (1-3 sentences)
    pub description: Option<String>,
    /// Social media profiles found on the website
    pub social_links: Vec<ExtractedSocialLink>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct ExtractedSocialLink {
    /// Normalized platform name: "instagram", "facebook", "tiktok", "twitter", "linkedin", "youtube"
    pub platform: String,
    /// Handle/username (without @ prefix)
    pub handle: String,
    /// Full URL if available
    pub url: Option<String>,
}

// =============================================================================
// LLM Prompt
// =============================================================================

const ORG_EXTRACTION_PROMPT: &str = r#"You are extracting organization information from a website's pages.

From the provided page content, extract:

1. **name** — The organization's official name. Look in headers, page titles, about pages, and footer content. Use the most formal/official name you find.

2. **description** — A brief 1-3 sentence description or mission statement. Look on the about page, homepage hero section, or meta descriptions.

3. **social_links** — Return an empty array (social profiles are extracted separately).

## Rules

- If you can't determine the organization name, return your best guess based on the domain name and content
"#;

// =============================================================================
// Page Selection
// =============================================================================

/// Maximum content size per LLM call
const MAX_CONTENT_CHARS: usize = 50_000;

/// Select the most relevant pages for organization extraction.
/// Prioritizes homepage, about page, and contact page.
fn select_org_pages(pages: &[(Uuid, String, String)]) -> Vec<&(Uuid, String, String)> {
    let mut priority_pages = Vec::new();
    let mut other_pages = Vec::new();

    for page in pages {
        let url_lower = page.1.to_lowercase();
        // Strip protocol and domain to get path
        let path = if let Some(idx) = url_lower.find("://") {
            let after_proto = &url_lower[idx + 3..];
            if let Some(slash_idx) = after_proto.find('/') {
                after_proto[slash_idx..].trim_end_matches('/').to_string()
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        let is_priority = path.is_empty()
            || path == "/"
            || url_lower.contains("about")
            || url_lower.contains("contact")
            || url_lower.contains("mission");

        if is_priority {
            priority_pages.push(page);
        } else {
            other_pages.push(page);
        }
    }

    // Start with priority pages, add others up to limit
    let mut selected = priority_pages;
    if selected.is_empty() {
        // Fallback: first 3 pages
        selected = other_pages.iter().take(3).copied().collect();
    }

    selected
}

/// Build the user prompt from selected pages, capped at MAX_CONTENT_CHARS.
fn build_content(pages: &[&(Uuid, String, String)], domain: &str) -> String {
    let mut content = format!("## Website: {}\n\n", domain);
    let mut total_chars = content.len();

    for page in pages {
        let header = format!("### Page: {}\n\n", page.1);
        let page_content = &page.2;
        let entry_size = header.len() + page_content.len() + 10;

        if total_chars + entry_size > MAX_CONTENT_CHARS {
            // Truncate this page's content to fit
            let remaining = MAX_CONTENT_CHARS.saturating_sub(total_chars + header.len() + 10);
            if remaining > 200 {
                content.push_str(&header);
                content.push_str(&page_content[..remaining.min(page_content.len())]);
                content.push_str("\n\n---\n\n");
            }
            break;
        }

        content.push_str(&header);
        content.push_str(page_content);
        content.push_str("\n\n---\n\n");
        total_chars += entry_size;
    }

    content
}

// =============================================================================
// Validation
// =============================================================================

/// Generic/garbage org names to reject
const INVALID_ORG_NAMES: &[&str] = &[
    "home",
    "home page",
    "about",
    "about us",
    "contact",
    "contact us",
    "n/a",
    "na",
    "none",
    "unknown",
    "unknown organization",
    "website",
    "untitled",
];

fn is_valid_org_name(name: &str) -> bool {
    let trimmed = name.trim();
    if trimmed.len() < 2 || trimmed.len() > 200 {
        return false;
    }
    !INVALID_ORG_NAMES.contains(&trimmed.to_lowercase().as_str())
}

// =============================================================================
// Handle Normalization
// =============================================================================

/// Normalize a social media handle: strip @, lowercase, extract from URLs.
fn normalize_handle(handle: &str, _platform: &str) -> String {
    let mut h = handle.trim().to_string();

    // Extract handle from full URLs
    let url_patterns = [
        "instagram.com/",
        "facebook.com/",
        "tiktok.com/@",
        "tiktok.com/",
        "twitter.com/",
        "x.com/",
        "linkedin.com/company/",
        "linkedin.com/in/",
        "youtube.com/@",
        "youtube.com/c/",
        "youtube.com/channel/",
    ];
    for pattern in &url_patterns {
        if let Some(idx) = h.to_lowercase().find(pattern) {
            h = h[idx + pattern.len()..].to_string();
            // Remove trailing path segments
            if let Some(slash) = h.find('/') {
                h = h[..slash].to_string();
            }
            // Remove query params
            if let Some(q) = h.find('?') {
                h = h[..q].to_string();
            }
            break;
        }
    }

    // Strip @ prefix
    h = h.trim_start_matches('@').to_string();

    // Lowercase
    h.to_lowercase()
}

/// Validate and normalize platform name
fn normalize_platform(platform: &str) -> Option<&'static str> {
    match platform.to_lowercase().as_str() {
        "instagram" | "ig" | "insta" => Some("instagram"),
        "facebook" | "fb" => Some("facebook"),
        "tiktok" | "tik tok" => Some("tiktok"),
        "twitter" | "x" => Some("twitter"),
        "linkedin" => Some("linkedin"),
        "youtube" | "yt" => Some("youtube"),
        _ => None,
    }
}

// =============================================================================
// Social Profile Extraction (regex-based)
// =============================================================================

static RE_INSTAGRAM: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?:https?://)?(?:www\.)?instagram\.com/([A-Za-z0-9_.]+)").unwrap()
});
static RE_FACEBOOK: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?:https?://)?(?:www\.)?facebook\.com/([A-Za-z0-9_.]+)").unwrap()
});
static RE_TWITTER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?:https?://)?(?:www\.)?(?:twitter|x)\.com/([A-Za-z0-9_]+)").unwrap()
});

/// Non-profile path segments to skip per platform.
const INSTAGRAM_SKIP: &[&str] = &[
    "p", "reel", "reels", "stories", "explore", "accounts", "tv", "s", "share",
];
const FACEBOOK_SKIP: &[&str] = &[
    "photo",
    "photos",
    "sharer",
    "share",
    "events",
    "groups",
    "watch",
    "marketplace",
    "login",
    "dialog",
    "plugins",
];
const TWITTER_SKIP: &[&str] = &["intent", "share", "hashtag", "search", "i", "home"];

struct SocialPattern {
    platform: &'static str,
    regex: &'static LazyLock<Regex>,
    skip_segments: &'static [&'static str],
}

const SOCIAL_PATTERNS: &[SocialPattern] = &[
    SocialPattern {
        platform: "instagram",
        regex: &RE_INSTAGRAM,
        skip_segments: INSTAGRAM_SKIP,
    },
    SocialPattern {
        platform: "facebook",
        regex: &RE_FACEBOOK,
        skip_segments: FACEBOOK_SKIP,
    },
    SocialPattern {
        platform: "twitter",
        regex: &RE_TWITTER,
        skip_segments: TWITTER_SKIP,
    },
];

/// Scan all page content for social media profile URLs using regex.
/// Returns deduplicated profiles found across all pages.
fn scan_social_profiles(pages: &[(Uuid, String, String)]) -> Vec<ExtractedSocialLink> {
    let mut seen = std::collections::HashSet::new();
    let mut profiles = Vec::new();

    for page in pages {
        let content = &page.2;

        for pattern in SOCIAL_PATTERNS {
            for cap in pattern.regex.captures_iter(content) {
                let handle = cap[1].to_lowercase();

                // Skip non-profile path segments
                if pattern.skip_segments.contains(&handle.as_str()) {
                    continue;
                }

                let key = (pattern.platform.to_string(), handle.clone());
                if seen.insert(key) {
                    // Reconstruct the matched URL
                    let url = cap.get(0).map(|m| m.as_str().to_string());

                    profiles.push(ExtractedSocialLink {
                        platform: pattern.platform.to_string(),
                        handle: handle.clone(),
                        url,
                    });
                }
            }
        }
    }

    profiles
}

// =============================================================================
// Main Activity
// =============================================================================

/// Extracted organization info (name, description, social links) without any DB writes.
///
/// Used by both `extract_and_create_organization` (new orgs) and the regenerate endpoint
/// (update-in-place).
pub async fn extract_organization_info(
    website_id: WebsiteId,
    deps: &ServerDeps,
) -> Result<(String, Option<String>, Vec<ExtractedSocialLink>)> {
    let pool = &deps.db_pool;

    // Load website
    let website = Website::find_by_id(website_id, pool).await?;

    // Load extraction pages
    let pages = ExtractionPage::find_by_domain(&website.domain, pool).await?;
    if pages.is_empty() {
        anyhow::bail!("No extraction pages found for domain {}", website.domain);
    }

    // Select relevant pages for org extraction (name/description)
    let selected = select_org_pages(&pages);
    let org_content = build_content(&selected, &website.domain);

    info!(
        website_id = %website_id,
        domain = %website.domain,
        pages_selected = selected.len(),
        content_chars = org_content.len(),
        "Extracting organization info"
    );

    // LLM extraction: org name + description (priority pages only)
    let extracted: ExtractedOrganization = deps
        .ai
        .extract(
            crate::kernel::GPT_5_MINI,
            ORG_EXTRACTION_PROMPT,
            &org_content,
        )
        .await
        .map_err(|e| anyhow::anyhow!("Organization extraction failed: {}", e))?;

    // Validate org name
    if !is_valid_org_name(&extracted.name) {
        anyhow::bail!(
            "Extracted org name '{}' is invalid or generic",
            extracted.name
        );
    }

    let org_name = extracted.name.trim().to_string();
    let org_desc = extracted
        .description
        .as_deref()
        .map(|d| d.trim().to_string());

    // Regex extraction: social profiles across ALL pages
    info!(
        total_pages = pages.len(),
        "REGEX SCAN: scanning all pages for social media URLs"
    );
    let social_result = scan_social_profiles(&pages);
    info!(found = social_result.len(), "REGEX SCAN: complete");
    for p in &social_result {
        info!(
            platform = %p.platform,
            handle = %p.handle,
            url = ?p.url,
            "Found social profile"
        );
    }

    info!(
        website_id = %website_id,
        org_name = %org_name,
        social_links = social_result.len(),
        "Organization info extracted"
    );

    Ok((org_name, org_desc, social_result))
}

/// Normalize and persist extracted social links for an organization.
pub async fn create_social_profiles_for_org(
    org_id: OrganizationId,
    social_links: &[ExtractedSocialLink],
    pool: &sqlx::PgPool,
) {
    for link in social_links {
        let platform = match normalize_platform(&link.platform) {
            Some(p) => p,
            None => {
                warn!(platform = %link.platform, "Skipping unsupported platform");
                continue;
            }
        };

        let handle = normalize_handle(&link.handle, platform);
        if handle.is_empty() {
            warn!(platform = %platform, "Skipping empty handle");
            continue;
        }

        match SocialProfile::find_or_create(org_id, platform, &handle, link.url.as_deref(), pool)
            .await
        {
            Ok(profile) => {
                info!(
                    org_id = %org_id,
                    platform = %platform,
                    handle = %handle,
                    profile_id = %profile.id,
                    "Social profile created/found"
                );
            }
            Err(e) => {
                warn!(
                    org_id = %org_id,
                    platform = %platform,
                    handle = %handle,
                    error = %e,
                    "Failed to create social profile, continuing"
                );
            }
        }
    }
}

/// Extract organization info from crawled pages and create Organization + SocialProfiles.
///
/// This is a best-effort activity — failures are logged but don't propagate.
/// Callable from both the crawl pipeline and the backfill endpoint.
pub async fn extract_and_create_organization(
    website_id: WebsiteId,
    deps: &ServerDeps,
) -> Result<OrganizationId> {
    let pool = &deps.db_pool;

    // Load website
    let website = Website::find_by_id(website_id, pool).await?;

    // Skip if already has an organization
    if website.organization_id.is_some() {
        info!(website_id = %website_id, "Website already has organization, skipping");
        return Ok(website.organization_id.unwrap());
    }

    // Extract org info (LLM + regex)
    let (org_name, org_desc, social_links) = extract_organization_info(website_id, deps).await?;

    // Create or find organization
    let org = Organization::find_or_create_by_name(&org_name, org_desc.as_deref(), pool).await?;

    // Link website to organization
    Website::set_organization_id(website_id, org.id, pool).await?;

    // Create social profiles
    create_social_profiles_for_org(org.id, &social_links, pool).await;

    info!(
        website_id = %website_id,
        org_id = %org.id,
        org_name = %org_name,
        "Organization created and linked to website"
    );

    Ok(org.id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_social_profiles_markdown_links() {
        let pages = vec![(
            Uuid::nil(),
            "https://www.canmn.org".to_string(),
            r#"
Some content here.
[instagram-unauth](https://www.instagram.com/communityaidnetworkmn/)
[facebook-unauth](https://www.facebook.com/CommunityAidNetworkMN)
Follow us on [Twitter](https://x.com/canaboretum)
[Some post](https://www.instagram.com/p/DUPMIPgkVkr/)
            "#
            .to_string(),
        )];

        let profiles = scan_social_profiles(&pages);

        let platforms: Vec<&str> = profiles.iter().map(|p| p.platform.as_str()).collect();
        let handles: Vec<&str> = profiles.iter().map(|p| p.handle.as_str()).collect();

        println!("Found profiles: {:?}", profiles);

        assert!(platforms.contains(&"instagram"), "Should find instagram");
        assert!(
            handles.contains(&"communityaidnetworkmn"),
            "Should find communityaidnetworkmn handle"
        );

        assert!(platforms.contains(&"facebook"), "Should find facebook");
        assert!(
            handles.contains(&"communityaidnetworkmn"),
            "Should find CommunityAidNetworkMN handle (lowercased)"
        );

        assert!(platforms.contains(&"twitter"), "Should find twitter/x");

        // Should NOT include post URLs
        assert!(
            !handles.contains(&"p"),
            "Should skip instagram.com/p/ (post URL)"
        );
    }

    #[test]
    fn test_scan_social_profiles_plain_urls() {
        let pages = vec![(
            Uuid::nil(),
            "https://example.org".to_string(),
            r#"
Visit us at https://instagram.com/myhandle and https://facebook.com/mypage
            "#
            .to_string(),
        )];

        let profiles = scan_social_profiles(&pages);
        assert_eq!(profiles.len(), 2);
        assert_eq!(profiles[0].handle, "myhandle");
        assert_eq!(profiles[1].handle, "mypage");
    }

    #[test]
    fn test_scan_deduplicates_across_pages() {
        let pages = vec![
            (
                Uuid::nil(),
                "https://example.org".to_string(),
                "https://instagram.com/myhandle".to_string(),
            ),
            (
                Uuid::nil(),
                "https://example.org/about".to_string(),
                "https://instagram.com/myhandle".to_string(),
            ),
        ];

        let profiles = scan_social_profiles(&pages);
        assert_eq!(
            profiles.len(),
            1,
            "Should deduplicate same handle across pages"
        );
    }
}
