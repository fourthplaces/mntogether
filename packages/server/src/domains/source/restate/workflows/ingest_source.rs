//! Unified source ingestion workflow
//!
//! Ingestion-only workflow that works for all source types (website, social).
//! Determines source type and dispatches to the appropriate ingestion activity.
//! Does NOT extract posts — that happens at the org level via ExtractOrgPostsWorkflow.
//!
//! Flow:
//! 1. Load source, determine type
//! 2. Website: ingest via Firecrawl/HTTP → extraction_pages
//! 3. Social: ingest via platform Ingestor (Apify) → extraction_pages
//! 4. Update last_scraped_at

use restate_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, warn};
use uuid::Uuid;

use crate::common::{EmptyRequest, SourceId};
use crate::domains::crawling::activities::ingest_website;
use crate::domains::source::activities::ingest_social::ingest_social_source;
use crate::domains::source::models::Source;
use crate::impl_restate_serde;
use crate::kernel::ServerDeps;

// =============================================================================
// Request / Response types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestSourceRequest {
    pub source_id: Uuid,
}

impl_restate_serde!(IngestSourceRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestSourceResult {
    pub pages_stored: usize,
    pub status: String,
}

impl_restate_serde!(IngestSourceResult);

// =============================================================================
// Workflow definition
// =============================================================================

#[restate_sdk::workflow]
#[name = "IngestSourceWorkflow"]
pub trait IngestSourceWorkflow {
    async fn run(req: IngestSourceRequest) -> Result<IngestSourceResult, HandlerError>;

    #[shared]
    async fn get_status(req: EmptyRequest) -> Result<String, HandlerError>;
}

pub struct IngestSourceWorkflowImpl {
    deps: Arc<ServerDeps>,
}

impl IngestSourceWorkflowImpl {
    pub fn with_deps(deps: Arc<ServerDeps>) -> Self {
        Self { deps }
    }
}

impl IngestSourceWorkflow for IngestSourceWorkflowImpl {
    async fn run(
        &self,
        ctx: WorkflowContext<'_>,
        req: IngestSourceRequest,
    ) -> Result<IngestSourceResult, HandlerError> {
        info!(source_id = %req.source_id, "Starting ingest source workflow");

        // Load source to determine type (durable)
        ctx.set("status", "Loading source...".to_string());

        let source_type: String = ctx
            .run(|| async {
                let source = Source::find_by_id(
                    SourceId::from_uuid(req.source_id),
                    &self.deps.db_pool,
                )
                .await
                .map_err(|e| restate_sdk::errors::TerminalError::new(e.to_string()))?;
                Ok(source.source_type)
            })
            .await?;

        // Dispatch based on source type (durable)
        let result = match source_type.as_str() {
            "website" => {
                ctx.set("status", "Ingesting website pages...".to_string());

                let pages = ctx
                    .run(|| async {
                        let r = ingest_website(
                            req.source_id,
                            Uuid::nil(), // system-initiated
                            true,        // admin action
                            &self.deps,
                        )
                        .await
                        .map_err(|e| restate_sdk::errors::TerminalError::new(e.to_string()))?;
                        Ok(r.pages_crawled as u64)
                    })
                    .await;

                match pages {
                    Ok(count) => {
                        let count = count as usize;
                        let msg = format!("Completed: {} pages ingested", count);
                        ctx.set("status", msg);
                        info!(source_id = %req.source_id, pages = count, "Website ingestion completed");
                        IngestSourceResult {
                            pages_stored: count,
                            status: "completed".to_string(),
                        }
                    }
                    Err(e) => {
                        let msg = format!("Failed to ingest website: {}", e);
                        warn!(source_id = %req.source_id, error = %e, "Website ingestion failed");
                        ctx.set("status", msg);
                        IngestSourceResult {
                            pages_stored: 0,
                            status: "failed".to_string(),
                        }
                    }
                }
            }
            "instagram" | "facebook" | "x" | "tiktok" => {
                ctx.set("status", "Scraping social media posts...".to_string());

                let pages = ctx
                    .run(|| async {
                        let r = ingest_social_source(req.source_id, &self.deps)
                            .await
                            .map_err(|e| restate_sdk::errors::TerminalError::new(e.to_string()))?;
                        Ok(r.pages_stored as u64)
                    })
                    .await;

                match pages {
                    Ok(count) => {
                        let count = count as usize;
                        let msg = format!("Completed: {} pages stored in extraction_pages", count);
                        ctx.set("status", msg);
                        info!(source_id = %req.source_id, pages = count, "Social ingestion completed");
                        IngestSourceResult {
                            pages_stored: count,
                            status: "completed".to_string(),
                        }
                    }
                    Err(e) => {
                        let msg = format!("Failed to ingest social source: {}", e);
                        warn!(source_id = %req.source_id, error = %e, "Social ingestion failed");
                        ctx.set("status", msg);
                        IngestSourceResult {
                            pages_stored: 0,
                            status: "failed".to_string(),
                        }
                    }
                }
            }
            other => {
                let msg = format!("Unsupported source type for ingestion: {}", other);
                warn!(source_id = %req.source_id, source_type = %other, "Unsupported source type");
                ctx.set("status", msg);
                IngestSourceResult {
                    pages_stored: 0,
                    status: "failed".to_string(),
                }
            }
        };

        Ok(result)
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
