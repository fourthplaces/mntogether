//! Scrape social media sources and return structured post data for extraction.

use anyhow::{Context, Result};
use chrono::Utc;
use extraction::types::page::CachedPage;
use tracing::info;
use uuid::Uuid;

use crate::common::SourceId;
use crate::domains::source::models::{SocialSource, Source};
use crate::kernel::ServerDeps;

/// Raw social media post data before LLM processing.
/// Preserves the original caption verbatim for use as post description.
#[derive(Debug, Clone)]
pub struct ScrapedSocialPost {
    /// Original caption/text — becomes the post description verbatim
    pub caption: String,
    /// URL of the individual social media post
    pub source_url: String,
    /// Profile URL (instagram.com/handle, facebook page, etc.)
    pub profile_url: String,
    /// Platform: "instagram", "facebook", "x"
    pub platform: String,
    /// Author name or handle
    pub author: Option<String>,
    /// Location tagged on the post (Instagram location_name)
    pub location: Option<String>,
    /// When the post was published (ISO string or human-readable)
    pub posted_at: Option<String>,
}

impl ScrapedSocialPost {
    /// Convert to CachedPage for downstream consumers (e.g. note extraction).
    /// Wraps the caption with platform metadata in markdown format.
    pub fn to_cached_page(&self) -> CachedPage {
        let mut content = String::new();
        let platform_display = capitalize(&self.platform);
        content.push_str(&format!("# {} Post\n\n", platform_display));
        content.push_str(&self.caption);
        content.push_str("\n\n---\n\n");
        if let Some(loc) = &self.location {
            content.push_str(&format!("**Location**: {}\n", loc));
        }
        if let Some(ts) = &self.posted_at {
            content.push_str(&format!("**Posted**: {}\n", ts));
        }
        CachedPage::new(&self.source_url, &self.profile_url, content)
    }
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

/// Scrape a social source via Apify and return structured post data.
pub async fn scrape_social_source(
    source_id: Uuid,
    deps: &ServerDeps,
) -> Result<Vec<ScrapedSocialPost>> {
    let pool = &deps.db_pool;
    let source = Source::find_by_id(SourceId::from_uuid(source_id), pool)
        .await
        .context("Failed to load source")?;

    let social = SocialSource::find_by_source_id(SourceId::from_uuid(source_id), pool)
        .await
        .context("Failed to load social source")?;

    let profile_url = source
        .url
        .clone()
        .unwrap_or_else(|| format!("https://www.instagram.com/{}/", social.handle));

    match source.source_type.as_str() {
        "instagram" => scrape_instagram(&social.handle, &profile_url, deps).await,
        "facebook" => scrape_facebook(&profile_url, deps).await,
        "x" | "twitter" => scrape_x(&social.handle, &profile_url, deps).await,
        other => anyhow::bail!("Social source type '{}' is not yet supported for regeneration", other),
    }
}

/// Scrape Instagram posts via Apify and return structured post data.
async fn scrape_instagram(
    handle: &str,
    profile_url: &str,
    deps: &ServerDeps,
) -> Result<Vec<ScrapedSocialPost>> {
    let apify = deps
        .apify_client
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Apify client not configured"))?;

    info!(handle = %handle, "Scraping Instagram posts via Apify");

    let posts = apify
        .scrape_instagram_posts(handle, 50)
        .await
        .map_err(|e| anyhow::anyhow!("Apify scrape failed: {}", e))?;

    info!(handle = %handle, total_posts = posts.len(), "Instagram scrape complete");

    let thirty_days_ago = Utc::now() - chrono::Duration::days(30);
    let mut scraped = Vec::new();

    for ig_post in &posts {
        // Filter: only last 30 days
        if let Some(ts) = ig_post.timestamp {
            if ts < thirty_days_ago {
                continue;
            }
        }

        // Filter: must have a caption
        let caption = match &ig_post.caption {
            Some(c) if !c.trim().is_empty() => c.clone(),
            _ => continue,
        };

        scraped.push(ScrapedSocialPost {
            caption,
            source_url: ig_post.url.clone(),
            profile_url: profile_url.to_string(),
            platform: "instagram".to_string(),
            author: Some(handle.to_string()),
            location: ig_post.location_name.clone(),
            posted_at: ig_post.timestamp.map(|ts| ts.format("%B %d, %Y").to_string()),
        });
    }

    info!(
        handle = %handle,
        total_scraped = posts.len(),
        posts_created = scraped.len(),
        "Scraped Instagram posts"
    );

    Ok(scraped)
}

/// Scrape Facebook page posts via Apify and return structured post data.
async fn scrape_facebook(
    page_url: &str,
    deps: &ServerDeps,
) -> Result<Vec<ScrapedSocialPost>> {
    let apify = deps
        .apify_client
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Apify client not configured"))?;

    info!(page_url = %page_url, "Scraping Facebook posts via Apify");

    let posts = apify
        .scrape_facebook_posts(page_url, 50)
        .await
        .map_err(|e| anyhow::anyhow!("Apify scrape failed: {}", e))?;

    info!(page_url = %page_url, total_posts = posts.len(), "Facebook scrape complete");

    let mut scraped = Vec::new();

    for fb_post in &posts {
        let text = match &fb_post.text {
            Some(t) if !t.trim().is_empty() => t.clone(),
            _ => continue,
        };

        let post_url = match &fb_post.url {
            Some(u) => u.clone(),
            None => continue,
        };

        scraped.push(ScrapedSocialPost {
            caption: text,
            source_url: post_url,
            profile_url: page_url.to_string(),
            platform: "facebook".to_string(),
            author: fb_post.page_name.clone(),
            location: None,
            posted_at: fb_post.time.clone(),
        });
    }

    info!(
        page_url = %page_url,
        total_scraped = posts.len(),
        posts_created = scraped.len(),
        "Scraped Facebook posts"
    );

    Ok(scraped)
}

/// Scrape X/Twitter posts via Apify and return structured post data.
async fn scrape_x(
    handle: &str,
    profile_url: &str,
    deps: &ServerDeps,
) -> Result<Vec<ScrapedSocialPost>> {
    let apify = deps
        .apify_client
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Apify client not configured"))?;

    info!(handle = %handle, "Scraping X/Twitter posts via Apify");

    let tweets = apify
        .scrape_x_posts(handle, 50)
        .await
        .map_err(|e| anyhow::anyhow!("Apify scrape failed: {}", e))?;

    info!(handle = %handle, total_posts = tweets.len(), "X/Twitter scrape complete");

    let mut scraped = Vec::new();

    for tweet in &tweets {
        let text = match tweet.content() {
            Some(t) if !t.trim().is_empty() => t.to_string(),
            _ => continue,
        };

        let tweet_url = match &tweet.url {
            Some(u) => u.clone(),
            None => continue,
        };

        let author_display = tweet
            .author
            .as_ref()
            .and_then(|a| a.name.as_deref())
            .map(|s| s.to_string())
            .or_else(|| Some(handle.to_string()));

        scraped.push(ScrapedSocialPost {
            caption: text,
            source_url: tweet_url,
            profile_url: profile_url.to_string(),
            platform: "x".to_string(),
            author: author_display,
            location: None,
            posted_at: tweet.created_at.clone(),
        });
    }

    info!(
        handle = %handle,
        total_scraped = tweets.len(),
        posts_created = scraped.len(),
        "Scraped X/Twitter posts"
    );

    Ok(scraped)
}
