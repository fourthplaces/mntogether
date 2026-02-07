//! Resource link AI extraction activity
//!
//! Extracts posts from a scraped resource link using AI with PII scrubbing.

use anyhow::Result;

use super::post_extraction;
use crate::common::{ExtractedPost, JobId};
use crate::kernel::ServerDeps;

/// Result of AI extraction from a resource link
#[derive(Debug, Clone)]
pub struct ExtractionResult {
    pub posts: Vec<ExtractedPost>,
    pub context: Option<String>,
    pub submitter_contact: Option<String>,
}

/// Extract posts from a scraped resource link using AI with PII scrubbing.
pub async fn extract_posts_from_resource_link(
    job_id: JobId,
    url: String,
    content: String,
    context: Option<String>,
    submitter_contact: Option<String>,
    deps: &ServerDeps,
) -> Result<ExtractionResult> {
    tracing::info!(
        job_id = %job_id,
        url = %url,
        content_length = content.len(),
        has_context = context.is_some(),
        "Starting AI listing extraction from resource link"
    );

    // Extract domain from URL for context
    let domain = url
        .split("//")
        .nth(1)
        .and_then(|s| s.split('/').next())
        .unwrap_or("unknown")
        .to_string();

    // Add context to the content if provided
    let content_with_context = if let Some(ref ctx_text) = context {
        tracing::info!(job_id = %job_id, "Adding user context to content");
        format!("Context: {}\n\n{}", ctx_text, content)
    } else {
        content
    };

    tracing::info!(job_id = %job_id, "Calling AI service to extract listings from resource link");

    // Delegate to domain function with PII scrubbing
    let extracted_posts = match post_extraction::extract_posts_with_pii_scrub(
        deps.ai.as_ref(),
        deps.pii_detector.as_ref(),
        &domain,
        &content_with_context,
        &url,
    )
    .await
    {
        Ok(listings) => {
            tracing::info!(
                job_id = %job_id,
                listings_count = listings.len(),
                "AI extraction from resource link completed successfully"
            );
            listings
        }
        Err(e) => {
            tracing::error!(
                job_id = %job_id,
                url = %url,
                error = %e,
                "AI extraction from resource link failed"
            );
            return Err(anyhow::anyhow!(
                "AI extraction from resource link failed: {}",
                e
            ));
        }
    };

    tracing::info!(
        job_id = %job_id,
        url = %url,
        listings_count = extracted_posts.len(),
        "Resource link extraction complete"
    );

    Ok(ExtractionResult {
        posts: extracted_posts,
        context,
        submitter_contact,
    })
}
