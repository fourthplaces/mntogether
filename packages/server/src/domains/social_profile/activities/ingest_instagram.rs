use anyhow::Result;
use chrono::Utc;

use crate::common::PostId;
use crate::domains::social_profile::models::SocialProfile;
use crate::kernel::ServerDeps;

/// Keywords that suggest an Instagram post contains an actionable listing.
const ACTIONABLE_KEYWORDS: &[&str] = &[
    "volunteer",
    "donate",
    "join",
    "register",
    "sign up",
    "signup",
    "help needed",
    "seeking",
    "looking for",
    "free",
    "open to",
    "apply",
    "enroll",
    "rsvp",
    "call for",
    "hiring",
    "accepting",
    "available",
];

/// Scrape an Instagram profile via Apify and create posts from actionable content.
pub async fn ingest_instagram_profile(
    profile: &SocialProfile,
    deps: &ServerDeps,
) -> Result<Vec<PostId>> {
    let apify = deps
        .apify_client
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Apify client not configured"))?;

    tracing::info!(
        profile_id = %profile.id,
        handle = %profile.handle,
        platform = %profile.platform,
        "Ingesting Instagram profile"
    );

    let posts = apify
        .scrape_instagram_posts(&profile.handle, 50)
        .await
        .map_err(|e| anyhow::anyhow!("Apify scrape failed: {}", e))?;

    let thirty_days_ago = Utc::now() - chrono::Duration::days(30);
    let mut created_ids = Vec::new();

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

        // Filter: must contain actionable keywords
        let caption_lower = caption.to_lowercase();
        let is_actionable = ACTIONABLE_KEYWORDS
            .iter()
            .any(|kw| caption_lower.contains(kw));
        if !is_actionable {
            continue;
        }

        // Map to post fields
        let title = extract_title(caption);
        let source_url = &ig_post.url;

        tracing::debug!(
            title = %title,
            source_url = %source_url,
            "Creating post from Instagram content"
        );

        // Create the post via SQL
        let post_id = sqlx::query_scalar::<_, PostId>(
            "INSERT INTO posts (title, description, post_type, category, status, submission_type, source_url, source_language, published_at)
             VALUES ($1, $2, 'service', 'general', 'pending_approval', 'scraped', $3, 'en', $4)
             RETURNING id",
        )
        .bind(&title)
        .bind(caption)
        .bind(source_url)
        .bind(ig_post.timestamp)
        .fetch_one(&deps.db_pool)
        .await?;

        // Link to social profile source via post_sources
        use crate::domains::posts::models::PostSource;
        if let Err(e) = PostSource::create(
            post_id,
            &profile.platform,
            profile.id.into_uuid(),
            Some(source_url),
            &deps.db_pool,
        )
        .await
        {
            tracing::warn!(
                post_id = %post_id,
                error = %e,
                "Failed to create post source link for Instagram post"
            );
        }

        created_ids.push(post_id);
    }

    // Update last_scraped_at
    SocialProfile::update_last_scraped(profile.id, &deps.db_pool).await?;

    tracing::info!(
        profile_id = %profile.id,
        total_scraped = posts.len(),
        posts_created = created_ids.len(),
        "Instagram ingestion complete"
    );

    Ok(created_ids)
}

/// Extract a title from a caption: first sentence, truncated to 100 chars.
fn extract_title(caption: &str) -> String {
    let first_line = caption.lines().next().unwrap_or(caption);

    // Find first sentence boundary
    let end = first_line
        .find(". ")
        .or_else(|| first_line.find("! "))
        .or_else(|| first_line.find("? "))
        .map(|i| i + 1)
        .unwrap_or(first_line.len());

    let sentence = &first_line[..end];
    if sentence.len() > 100 {
        format!("{}...", &sentence[..97])
    } else {
        sentence.to_string()
    }
}
