//! Post effect cascade handlers
//!
//! These handlers respond to fact events and are called from the composite effect.
//! Entry-point actions live in `actions/`, not here.

use anyhow::Result;
use seesaw_core::EffectContext;
use tracing::{info, warn};

use crate::common::AppState;
use crate::common::{ExtractedPost, JobId, WebsiteId};
use crate::domains::posts::events::PostEvent;
use crate::kernel::ServerDeps;

/// Extract domain from URL (e.g., "https://example.org/path" -> "example.org")
pub fn extract_domain(url: &str) -> Option<String> {
    url::Url::parse(url).ok().and_then(|parsed| {
        parsed.host_str().map(|host| {
            host.strip_prefix("www.").unwrap_or(host).to_lowercase()
        })
    })
}

/// Cascade handler: ResourceLinkPostsExtracted → create posts from resource link
pub async fn handle_create_posts_from_resource_link(
    _job_id: JobId,
    url: String,
    posts: Vec<ExtractedPost>,
    context: Option<String>,
    _submitter_contact: Option<String>,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<()> {
    use crate::domains::posts::models::Post;
    use crate::domains::website::models::Website;

    let organization_name = context
        .clone()
        .unwrap_or_else(|| "Submitted Resource".to_string());

    let source = Website::find_by_domain(&url, &ctx.deps().db_pool)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Website source not found for URL: {}", url))?;

    let source_id = source.id;

    Website::update_last_scraped(source_id, &ctx.deps().db_pool).await?;

    let mut created_count = 0;

    for extracted_post in posts {
        let _contact_json = extracted_post
            .contact
            .and_then(|c| serde_json::to_value(c).ok());

        match Post::create(
            organization_name.clone(),
            extracted_post.title.clone(),
            extracted_post.description.clone(),
            Some(extracted_post.tldr),
            "opportunity".to_string(),
            "general".to_string(),
            Some("accepting".to_string()),
            extracted_post.urgency,
            None,
            "pending_approval".to_string(),
            "en".to_string(),
            Some("user_submitted".to_string()),
            None,
            Some(WebsiteId::from_uuid(source_id.into_uuid())),
            Some(url.clone()),
            None,
            &ctx.deps().db_pool,
        )
        .await
        {
            Ok(new_post) => {
                created_count += 1;
                info!(
                    post_id = %new_post.id,
                    org = %new_post.organization_name,
                    title = %new_post.title,
                    "Created listing from resource link"
                );
            }
            Err(e) => {
                warn!(
                    error = %e,
                    title = %extracted_post.title,
                    "Failed to create listing from resource link"
                );
            }
        }
    }

    info!(created_count = %created_count, "Created listings from resource link");

    ctx.emit(PostEvent::PostEntryCreated {
        post_id: crate::common::PostId::new(),
        organization_name: "Resource Link".to_string(),
        title: format!("{} listings created", created_count),
        submission_type: "user_submitted".to_string(),
    });
    Ok(())
}
