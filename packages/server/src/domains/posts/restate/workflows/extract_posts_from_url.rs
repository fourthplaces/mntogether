//! Extract posts from URL workflow
//!
//! Durable workflow that processes a submitted URL:
//! 1. Scrape URL (Firecrawl/HTTP)
//! 2. AI extraction with PII scrubbing
//! 3. Create post records in DB
//!
//! Each step is a separate ctx.run() block so Restate journals intermediate
//! results and won't re-execute completed steps on retry.

use restate_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::common::{EmptyRequest, JobId};
use crate::domains::posts::activities::resource_link_creation::create_posts_from_resource_link;
use crate::domains::posts::activities::resource_link_extraction::extract_posts_from_resource_link;
use crate::domains::posts::activities::resource_link_scraping::scrape_resource_link;
use crate::impl_restate_serde;
use crate::kernel::ServerDeps;

/// Wrapper for journaling post creation count between ctx.run() blocks
#[derive(Debug, Clone, Serialize, Deserialize)]
struct PostsCreated {
    count: usize,
}

impl_restate_serde!(PostsCreated);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractPostsFromUrlRequest {
    pub job_id: Uuid,
    pub url: String,
    pub submitter_contact: Option<String>,
}

impl_restate_serde!(ExtractPostsFromUrlRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractPostsFromUrlResult {
    pub posts_created: usize,
    pub status: String,
}

impl_restate_serde!(ExtractPostsFromUrlResult);

#[restate_sdk::workflow]
pub trait ExtractPostsFromUrlWorkflow {
    async fn run(request: ExtractPostsFromUrlRequest) -> Result<ExtractPostsFromUrlResult, HandlerError>;

    #[shared]
    async fn get_status(req: EmptyRequest) -> Result<String, HandlerError>;
}

pub struct ExtractPostsFromUrlWorkflowImpl {
    deps: std::sync::Arc<ServerDeps>,
}

impl ExtractPostsFromUrlWorkflowImpl {
    pub fn with_deps(deps: std::sync::Arc<ServerDeps>) -> Self {
        Self { deps }
    }
}

impl ExtractPostsFromUrlWorkflow for ExtractPostsFromUrlWorkflowImpl {
    async fn run(
        &self,
        ctx: WorkflowContext<'_>,
        request: ExtractPostsFromUrlRequest,
    ) -> Result<ExtractPostsFromUrlResult, HandlerError> {
        let job_id = JobId::from(request.job_id);

        tracing::info!(
            job_id = %job_id,
            url = %request.url,
            "Starting extract posts from URL workflow"
        );

        // Step 1: Scrape — journaled, won't re-run on replay
        ctx.set("status", "Scraping URL...".to_string());

        let scrape = ctx
            .run(|| async {
                scrape_resource_link(
                    job_id,
                    request.url.clone(),
                    None,
                    request.submitter_contact.clone(),
                    &self.deps,
                )
                .await
                .map_err(Into::into)
            })
            .await?;

        // Step 2: AI extraction — if this fails, step 1 is replayed from journal
        ctx.set("status", "Extracting posts...".to_string());

        let extraction = ctx
            .run(|| async {
                extract_posts_from_resource_link(
                    job_id,
                    request.url.clone(),
                    scrape.content,
                    scrape.context,
                    scrape.submitter_contact,
                    &self.deps,
                )
                .await
                .map_err(Into::into)
            })
            .await?;

        let posts_count = extraction.posts.len();

        // Step 3: Create posts — if this fails, steps 1+2 are replayed from journal
        ctx.set(
            "status",
            format!("Creating {} posts...", posts_count),
        );

        let created = ctx
            .run(|| async {
                create_posts_from_resource_link(
                    job_id,
                    request.url.clone(),
                    extraction.posts,
                    extraction.context,
                    extraction.submitter_contact,
                    &self.deps,
                )
                .await?;
                Ok(PostsCreated { count: posts_count })
            })
            .await?;

        ctx.set(
            "status",
            format!("Completed: {} created", created.count),
        );

        Ok(ExtractPostsFromUrlResult {
            posts_created: created.count,
            status: "completed".to_string(),
        })
    }

    async fn get_status(
        &self,
        ctx: SharedWorkflowContext<'_>,
        _req: EmptyRequest,
    ) -> Result<String, HandlerError> {
        Ok(ctx
            .get::<String>("status")
            .await?
            .unwrap_or_else(|| "pending".to_string()))
    }
}
