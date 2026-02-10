//! Scrape social media sources and convert posts to CachedPages for LLM extraction.

use anyhow::{Context, Result};
use chrono::Utc;
use extraction::types::page::CachedPage;
use tracing::info;
use uuid::Uuid;

use crate::common::SourceId;
use crate::domains::source::models::{SocialSource, Source};
use crate::kernel::ServerDeps;

/// Scrape a social source via Apify and return posts as CachedPages for the extraction pipeline.
pub async fn scrape_social_source(
    source_id: Uuid,
    deps: &ServerDeps,
) -> Result<Vec<CachedPage>> {
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

/// Scrape Instagram posts via Apify and convert to CachedPages.
async fn scrape_instagram(
    handle: &str,
    profile_url: &str,
    deps: &ServerDeps,
) -> Result<Vec<CachedPage>> {
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
    let mut pages = Vec::new();

    for ig_post in &posts {
        // Filter: only last 30 days
        if let Some(ts) = ig_post.timestamp {
            if ts < thirty_days_ago {
                continue;
            }
        }

        // Filter: must have a caption
        let caption = match &ig_post.caption {
            Some(c) if !c.trim().is_empty() => c,
            _ => continue,
        };

        // Build markdown content with all available metadata
        let mut content = String::new();

        content.push_str(&format!("# Instagram Post by @{}\n\n", handle));
        content.push_str(caption);
        content.push_str("\n\n---\n\n");

        if let Some(location) = &ig_post.location_name {
            content.push_str(&format!("**Location**: {}\n", location));
        }

        if let Some(ts) = ig_post.timestamp {
            content.push_str(&format!("**Posted**: {}\n", ts.format("%B %d, %Y")));
        }

        if let Some(likes) = ig_post.likes_count {
            content.push_str(&format!("**Likes**: {}\n", likes));
        }

        if let Some(comments) = ig_post.comments_count {
            content.push_str(&format!("**Comments**: {}\n", comments));
        }

        if let Some(mentions) = &ig_post.mentions {
            if !mentions.is_empty() {
                content.push_str(&format!("**Mentions**: {}\n", mentions.join(", ")));
            }
        }

        let page = CachedPage::new(&ig_post.url, profile_url, content);
        pages.push(page);
    }

    info!(
        handle = %handle,
        total_scraped = posts.len(),
        pages_created = pages.len(),
        "Converted Instagram posts to CachedPages"
    );

    Ok(pages)
}

/// Scrape Facebook page posts via Apify and convert to CachedPages.
async fn scrape_facebook(
    page_url: &str,
    deps: &ServerDeps,
) -> Result<Vec<CachedPage>> {
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

    let mut pages = Vec::new();

    for fb_post in &posts {
        let text = match &fb_post.text {
            Some(t) if !t.trim().is_empty() => t,
            _ => continue,
        };

        let post_url = match &fb_post.url {
            Some(u) => u.as_str(),
            None => continue,
        };

        let mut content = String::new();

        if let Some(page_name) = &fb_post.page_name {
            content.push_str(&format!("# Facebook Post by {}\n\n", page_name));
        } else {
            content.push_str("# Facebook Post\n\n");
        }

        content.push_str(text);
        content.push_str("\n\n---\n\n");

        if let Some(time) = &fb_post.time {
            content.push_str(&format!("**Posted**: {}\n", time));
        }

        if let Some(likes) = fb_post.likes {
            content.push_str(&format!("**Likes**: {}\n", likes));
        }

        if let Some(comments) = fb_post.comments {
            content.push_str(&format!("**Comments**: {}\n", comments));
        }

        if let Some(shares) = fb_post.shares {
            content.push_str(&format!("**Shares**: {}\n", shares));
        }

        let page = CachedPage::new(post_url, page_url, content);
        pages.push(page);
    }

    info!(
        page_url = %page_url,
        total_scraped = posts.len(),
        pages_created = pages.len(),
        "Converted Facebook posts to CachedPages"
    );

    Ok(pages)
}

/// Scrape X/Twitter posts via Apify and convert to CachedPages.
async fn scrape_x(
    handle: &str,
    profile_url: &str,
    deps: &ServerDeps,
) -> Result<Vec<CachedPage>> {
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

    let mut pages = Vec::new();

    for tweet in &tweets {
        let text = match tweet.content() {
            Some(t) if !t.trim().is_empty() => t,
            _ => continue,
        };

        let tweet_url = match &tweet.url {
            Some(u) => u.as_str(),
            None => continue,
        };

        let mut content = String::new();

        let author_display = tweet
            .author
            .as_ref()
            .and_then(|a| a.name.as_deref())
            .unwrap_or(handle);

        content.push_str(&format!("# Post by @{} ({})\n\n", handle, author_display));
        content.push_str(text);
        content.push_str("\n\n---\n\n");

        if let Some(created_at) = &tweet.created_at {
            content.push_str(&format!("**Posted**: {}\n", created_at));
        }

        if let Some(likes) = tweet.like_count {
            content.push_str(&format!("**Likes**: {}\n", likes));
        }

        if let Some(retweets) = tweet.retweet_count {
            content.push_str(&format!("**Retweets**: {}\n", retweets));
        }

        if let Some(replies) = tweet.reply_count {
            content.push_str(&format!("**Replies**: {}\n", replies));
        }

        let page = CachedPage::new(tweet_url, profile_url, content);
        pages.push(page);
    }

    info!(
        handle = %handle,
        total_scraped = tweets.len(),
        pages_created = pages.len(),
        "Converted X/Twitter posts to CachedPages"
    );

    Ok(pages)
}
