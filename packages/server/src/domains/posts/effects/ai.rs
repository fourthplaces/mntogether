//! AI effect handlers - handle AI extraction request events
//!
//! These handlers emit events directly and are called from the composite effect.

use anyhow::Result;
use seesaw_core::EffectContext;

use super::{post_extraction, ServerDeps};
use crate::common::AppState;
use crate::common::{JobId, WebsiteId};
use crate::domains::posts::events::PostEvent;
use crate::domains::website::models::Website;

// ============================================================================
// Handler actions - emit events directly
// ============================================================================

pub async fn handle_extract_posts(
    source_id: WebsiteId,
    job_id: JobId,
    organization_name: String,
    content: String,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<()> {
    tracing::info!(
        source_id = %source_id,
        job_id = %job_id,
        organization_name = %organization_name,
        content_length = content.len(),
        "Starting AI listing extraction"
    );

    // Get source for domain info
    let source = match Website::find_by_id(source_id, &ctx.deps().db_pool).await {
        Ok(s) => {
            tracing::info!(source_id = %source_id, domain = %s.domain, "Source found for extraction");
            s
        }
        Err(e) => {
            tracing::error!(
                source_id = %source_id,
                error = %e,
                "Failed to find source for extraction"
            );
            return Err(anyhow::anyhow!("Failed to find source: {}", e));
        }
    };

    tracing::info!(
        source_id = %source_id,
        domain = %source.domain,
        "Calling AI service to extract listings"
    );

    // Delegate to domain function with PII scrubbing
    let extracted_posts = match post_extraction::extract_posts_with_pii_scrub(
        ctx.deps().ai.as_ref(),
        ctx.deps().pii_detector.as_ref(),
        &organization_name,
        &content,
        &source.domain,
    )
    .await
    {
        Ok(listings) => {
            tracing::info!(
                source_id = %source_id,
                listings_count = listings.len(),
                "AI extraction completed successfully"
            );
            listings
        }
        Err(e) => {
            tracing::error!(
                source_id = %source_id,
                error = %e,
                "AI extraction failed"
            );
            return Err(anyhow::anyhow!("AI extraction failed: {}", e));
        }
    };

    // Emit fact event
    tracing::info!(
        source_id = %source_id,
        job_id = %job_id,
        listings_count = extracted_posts.len(),
        "Emitting PostsExtracted event"
    );
    ctx.emit(PostEvent::PostsExtracted {
        source_id,
        job_id,
        posts: extracted_posts,
    });
    Ok(())
}

pub async fn handle_extract_posts_from_resource_link(
    job_id: JobId,
    url: String,
    content: String,
    context: Option<String>,
    submitter_contact: Option<String>,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<()> {
    tracing::info!(
        job_id = %job_id,
        url = %url,
        content_length = content.len(),
        has_context = context.is_some(),
        "Starting AI listing extraction from resource link"
    );

    // Try to extract organization name from URL (domain)
    let organization_name = url
        .split("//")
        .nth(1)
        .and_then(|s| s.split('/').next())
        .unwrap_or("Unknown Organization")
        .to_string();

    tracing::info!(
        job_id = %job_id,
        organization_name = %organization_name,
        "Extracted organization name from URL"
    );

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
        ctx.deps().ai.as_ref(),
        ctx.deps().pii_detector.as_ref(),
        &organization_name,
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

    // Emit fact event
    tracing::info!(
        job_id = %job_id,
        url = %url,
        listings_count = extracted_posts.len(),
        "Emitting ResourceLinkPostsExtracted event"
    );
    ctx.emit(PostEvent::ResourceLinkPostsExtracted {
        job_id,
        url,
        posts: extracted_posts,
        context,
        submitter_contact,
    });
    Ok(())
}
