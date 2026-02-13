//! Resource link post creation activity
//!
//! Creates post records in the database from extracted posts.

use anyhow::Result;
use tracing::{info, warn};

use crate::common::{ExtractedPost, JobId};
use crate::kernel::ServerDeps;

/// Create posts from extracted resource link data.
/// Returns the count of posts created.
pub async fn create_posts_from_resource_link(
    _job_id: JobId,
    url: String,
    posts: Vec<ExtractedPost>,
    _context: Option<String>,
    _submitter_contact: Option<String>,
    deps: &ServerDeps,
) -> Result<usize> {
    use crate::domains::posts::models::{CreatePost, Post};
    use crate::domains::website::models::Website;

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
                .title(extracted_post.title.clone())
                .description(extracted_post.description.clone())
                .summary(Some(extracted_post.summary))
                .capacity_status(Some("accepting".to_string()))
                .urgency(extracted_post.urgency)
                .submission_type(Some("user_submitted".to_string()))
                .source_url(Some(url.clone()))
                .build(),
            &deps.db_pool,
        )
        .await
        {
            Ok(new_post) => {
                // Link to website source via post_sources
                use crate::domains::posts::models::PostSource;
                if let Err(e) = PostSource::create(
                    new_post.id,
                    "website",
                    source_id.into_uuid(),
                    Some(&url),
                    &deps.db_pool,
                )
                .await
                {
                    warn!(
                        post_id = %new_post.id,
                        error = %e,
                        "Failed to create post source link"
                    );
                }

                created_count += 1;
                info!(
                    post_id = %new_post.id,
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

    Ok(created_count)
}
