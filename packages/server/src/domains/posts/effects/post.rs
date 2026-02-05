//! Post effect cascade handlers
//!
//! These handlers respond to fact events and are called from the composite effect.
//! Entry-point actions live in `actions/`, not here.

use anyhow::Result;
use tracing::{info, warn};

use crate::common::{ExtractedPost, JobId, WebsiteId};
use crate::domains::posts::events::PostEvent;
use crate::kernel::ServerDeps;

/// Extract domain from URL (e.g., "https://example.org/path" -> "example.org")
pub fn extract_domain(url: &str) -> Option<String> {
    url::Url::parse(url).ok().and_then(|parsed| {
        parsed
            .host_str()
            .map(|host| host.strip_prefix("www.").unwrap_or(host).to_lowercase())
    })
}

/// Cascade handler: ResourceLinkPostsExtracted → create posts from resource link
/// Returns the PostEntryCreated event with summary info.
pub async fn handle_create_posts_from_resource_link(
    _job_id: JobId,
    url: String,
    posts: Vec<ExtractedPost>,
    context: Option<String>,
    _submitter_contact: Option<String>,
    deps: &ServerDeps,
) -> Result<PostEvent> {
    use crate::domains::posts::models::{CreatePost, Post};
    use crate::domains::website::models::Website;

    let organization_name = context
        .clone()
        .unwrap_or_else(|| "Submitted Resource".to_string());

    let source = Website::find_by_domain(&url, &deps.db_pool)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Website source not found for URL: {}", url))?;

    let source_id = source.id;

    Website::update_last_scraped(source_id, &deps.db_pool).await?;

    let mut created_count = 0;

    for extracted_post in posts {
        let _contact_json = extracted_post
            .contact
            .and_then(|c| serde_json::to_value(c).ok());

        match Post::create(
            CreatePost::builder()
                .organization_name(organization_name.clone())
                .title(extracted_post.title.clone())
                .description(extracted_post.description.clone())
                .tldr(Some(extracted_post.tldr))
                .capacity_status(Some("accepting".to_string()))
                .urgency(extracted_post.urgency)
                .submission_type(Some("user_submitted".to_string()))
                .website_id(Some(WebsiteId::from_uuid(source_id.into_uuid())))
                .source_url(Some(url.clone()))
                .build(),
            &deps.db_pool,
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

    Ok(PostEvent::PostEntryCreated {
        post_id: crate::common::PostId::new(),
        organization_name: "Resource Link".to_string(),
        title: format!("{} listings created", created_count),
        submission_type: "user_submitted".to_string(),
    })
}
