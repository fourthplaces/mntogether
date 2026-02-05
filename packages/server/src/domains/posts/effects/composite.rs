//! Post composite effect - watches FACT events and calls cascade handlers
//!
//! Architecture (seesaw 0.7.2 direct-call pattern):
//!   GraphQL → process(action) → returns event → Effect watches facts → calls handlers
//!
//! NO *Requested events. Effects watch FACT events and cascade directly.
//!
//! Cascade flows:
//!   WebsiteCreatedFromLink → handle_scrape_resource_link → extract → create (chained internally)
//!   ResourceLinkScraped → extract → create (chained internally)
//!   ResourceLinkPostsExtracted → create posts

use seesaw_core::effect::{self, EffectContext};
use tracing::{error, info};

use crate::common::AppState;
use crate::domains::posts::events::PostEvent;
use crate::kernel::ServerDeps;

use super::ai::handle_extract_posts_from_resource_link;
use super::post::handle_create_posts_from_resource_link;
use super::scraper::handle_scrape_resource_link;

/// Build the post composite effect handler using the 0.7.2 builder pattern.
///
/// This composite effect watches FACT events and calls cascade handlers.
/// Entry-point handlers are called directly from GraphQL via process().
pub fn post_composite_effect() -> seesaw_core::effect::Effect<AppState, ServerDeps> {
    effect::on::<PostEvent>().then(
        |event, ctx: EffectContext<AppState, ServerDeps>| async move {
            match event.as_ref() {
                // =================================================================
                // Cascade: WebsiteCreatedFromLink → scrape → extract → create
                // =================================================================
                PostEvent::WebsiteCreatedFromLink {
                    job_id,
                    url,
                    submitter_contact,
                    ..
                } => {
                    // Step 1: Scrape the resource link
                    let scraped_event = match handle_scrape_resource_link(
                        *job_id,
                        url.clone(),
                        None, // context is not available in this event
                        submitter_contact.clone(),
                        ctx.deps(),
                    )
                    .await
                    {
                        Ok(e) => e,
                        Err(e) => {
                            error!(error = %e, url = %url, "Scraping resource link failed");
                            return Ok(());
                        }
                    };

                    // Step 2: Extract posts from scraped content
                    if let PostEvent::ResourceLinkScraped {
                        job_id,
                        url,
                        content,
                        context,
                        submitter_contact,
                        ..
                    } = scraped_event
                    {
                        let extracted_event = match handle_extract_posts_from_resource_link(
                            job_id,
                            url.clone(),
                            content,
                            context.clone(),
                            submitter_contact.clone(),
                            ctx.deps(),
                        )
                        .await
                        {
                            Ok(e) => e,
                            Err(e) => {
                                error!(error = %e, url = %url, "Extracting posts failed");
                                return Ok(());
                            }
                        };

                        // Step 3: Create posts from extracted data
                        if let PostEvent::ResourceLinkPostsExtracted {
                            job_id,
                            url,
                            posts,
                            context,
                            submitter_contact,
                        } = extracted_event
                        {
                            match handle_create_posts_from_resource_link(
                                job_id,
                                url.clone(),
                                posts,
                                context,
                                submitter_contact,
                                ctx.deps(),
                            )
                            .await
                            {
                                Ok(PostEvent::PostEntryCreated { title, .. }) => {
                                    info!(url = %url, title = %title, "Posts created from resource link");
                                }
                                Ok(_) => {}
                                Err(e) => {
                                    error!(error = %e, url = %url, "Creating posts failed");
                                }
                            }
                        }
                    }
                    Ok(())
                }

                // =================================================================
                // Cascade: ResourceLinkScraped → extract → create
                // =================================================================
                PostEvent::ResourceLinkScraped {
                    job_id,
                    url,
                    content,
                    context,
                    submitter_contact,
                    ..
                } => {
                    // Step 1: Extract posts from scraped content
                    let extracted_event = match handle_extract_posts_from_resource_link(
                        *job_id,
                        url.clone(),
                        content.clone(),
                        context.clone(),
                        submitter_contact.clone(),
                        ctx.deps(),
                    )
                    .await
                    {
                        Ok(e) => e,
                        Err(e) => {
                            error!(error = %e, url = %url, "Extracting posts failed");
                            return Ok(());
                        }
                    };

                    // Step 2: Create posts from extracted data
                    if let PostEvent::ResourceLinkPostsExtracted {
                        job_id,
                        url,
                        posts,
                        context,
                        submitter_contact,
                    } = extracted_event
                    {
                        match handle_create_posts_from_resource_link(
                            job_id,
                            url.clone(),
                            posts,
                            context,
                            submitter_contact,
                            ctx.deps(),
                        )
                        .await
                        {
                            Ok(PostEvent::PostEntryCreated { title, .. }) => {
                                info!(url = %url, title = %title, "Posts created from resource link");
                            }
                            Ok(_) => {}
                            Err(e) => {
                                error!(error = %e, url = %url, "Creating posts failed");
                            }
                        }
                    }
                    Ok(())
                }

                // =================================================================
                // Cascade: ResourceLinkPostsExtracted → create posts
                // =================================================================
                PostEvent::ResourceLinkPostsExtracted {
                    job_id,
                    url,
                    posts,
                    context,
                    submitter_contact,
                } => {
                    match handle_create_posts_from_resource_link(
                        *job_id,
                        url.clone(),
                        posts.clone(),
                        context.clone(),
                        submitter_contact.clone(),
                        ctx.deps(),
                    )
                    .await
                    {
                        Ok(PostEvent::PostEntryCreated { title, .. }) => {
                            info!(url = %url, title = %title, "Posts created from resource link");
                        }
                        Ok(_) => {}
                        Err(e) => {
                            error!(error = %e, url = %url, "Creating posts failed");
                        }
                    }
                    Ok(())
                }

                // =================================================================
                // Terminal events - no cascade needed
                // =================================================================
                PostEvent::PostEntryCreated { .. }
                | PostEvent::PostApproved { .. }
                | PostEvent::PostRejected { .. }
                | PostEvent::PostExpired { .. }
                | PostEvent::PostArchived { .. }
                | PostEvent::PostViewed { .. }
                | PostEvent::PostClicked { .. }
                | PostEvent::PostDeleted { .. }
                | PostEvent::PostReported { .. }
                | PostEvent::ReportResolved { .. }
                | PostEvent::ReportDismissed { .. }
                | PostEvent::PostEmbeddingGenerated { .. }
                | PostEvent::PostsDeduplicated { .. }
                | PostEvent::WebsitePendingApproval { .. } => Ok(()),
            }
        },
    )
}
