//! Resource link workflow
//!
//! Durable workflow that processes a submitted resource link:
//! 1. Scrape URL (Firecrawl/HTTP)
//! 2. AI extraction with PII scrubbing
//! 3. Create post records in DB

use restate_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::common::JobId;
use crate::impl_restate_serde;
use crate::kernel::ServerDeps;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLinkRequest {
    pub job_id: Uuid,
    pub url: String,
    pub submitter_contact: Option<String>,
}

impl_restate_serde!(ResourceLinkRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLinkResult {
    pub posts_created: usize,
    pub status: String,
}

impl_restate_serde!(ResourceLinkResult);

#[restate_sdk::workflow]
pub trait ResourceLinkWorkflow {
    async fn run(request: ResourceLinkRequest) -> Result<ResourceLinkResult, HandlerError>;
}

pub struct ResourceLinkWorkflowImpl {
    deps: std::sync::Arc<ServerDeps>,
}

impl ResourceLinkWorkflowImpl {
    pub fn with_deps(deps: std::sync::Arc<ServerDeps>) -> Self {
        Self { deps }
    }
}

impl ResourceLinkWorkflow for ResourceLinkWorkflowImpl {
    async fn run(
        &self,
        ctx: WorkflowContext<'_>,
        request: ResourceLinkRequest,
    ) -> Result<ResourceLinkResult, HandlerError> {
        let job_id = JobId::from(request.job_id);

        tracing::info!(
            job_id = %job_id,
            url = %request.url,
            "Starting resource link workflow"
        );

        // Single durable block: scrape → extract → create posts
        let result = ctx
            .run(|| async {
                use crate::domains::posts::activities::resource_link_creation::create_posts_from_resource_link;
                use crate::domains::posts::activities::resource_link_extraction::extract_posts_from_resource_link;
                use crate::domains::posts::activities::resource_link_scraping::scrape_resource_link;

                // Step 1: Scrape
                let scrape = scrape_resource_link(
                    job_id,
                    request.url.clone(),
                    None,
                    request.submitter_contact.clone(),
                    &self.deps,
                )
                .await?;

                // Step 2: AI extraction
                let extraction = extract_posts_from_resource_link(
                    job_id,
                    request.url.clone(),
                    scrape.content,
                    scrape.context,
                    scrape.submitter_contact,
                    &self.deps,
                )
                .await?;

                let posts_count = extraction.posts.len();

                // Step 3: Create posts
                create_posts_from_resource_link(
                    job_id,
                    request.url.clone(),
                    extraction.posts,
                    extraction.context,
                    extraction.submitter_contact,
                    &self.deps,
                )
                .await?;

                Ok(ResourceLinkResult {
                    posts_created: posts_count,
                    status: "completed".to_string(),
                })
            })
            .await?;

        Ok(result)
    }
}
