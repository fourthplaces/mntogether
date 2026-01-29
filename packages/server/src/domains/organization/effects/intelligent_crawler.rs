use anyhow::Result;
use async_trait::async_trait;
use seesaw::{Effect, EffectContext};

use super::deps::ServerDeps;
use crate::common::JobId;
use crate::domains::organization::commands::OrganizationCommand;
use crate::domains::organization::events::OrganizationEvent;
use intelligent_crawler::{
    crawler, detector, extractor, relationships, DetectionConfig, Heuristic, RelationshipRule,
    Storage,
};

/// Intelligent Crawler Effect - Handles intelligent web crawling commands
///
/// This effect orchestrates the intelligent crawler library for
/// crawl → detect → extract → relate workflows
pub struct IntelligentCrawlerEffect;

#[async_trait]
impl Effect<OrganizationCommand, ServerDeps> for IntelligentCrawlerEffect {
    type Event = OrganizationEvent;

    async fn execute(
        &self,
        cmd: OrganizationCommand,
        ctx: EffectContext<ServerDeps>,
    ) -> Result<OrganizationEvent> {
        match cmd {
            OrganizationCommand::CrawlSite {
                url,
                job_id,
                page_limit,
            } => handle_crawl_site(url, job_id, page_limit, &ctx).await,
            OrganizationCommand::DetectInformation {
                snapshot_ids,
                job_id,
                detection_kind,
            } => handle_detect_information(snapshot_ids, job_id, detection_kind, &ctx).await,
            OrganizationCommand::ExtractData {
                detection_ids,
                job_id,
                schema_id,
            } => handle_extract_data(detection_ids, job_id, schema_id, &ctx).await,
            OrganizationCommand::ResolveRelationships {
                extraction_ids,
                job_id,
            } => handle_resolve_relationships(extraction_ids, job_id, &ctx).await,
            _ => anyhow::bail!("IntelligentCrawlerEffect: Unexpected command"),
        }
    }
}

// ============================================================================
// Handler functions
// ============================================================================

async fn handle_crawl_site(
    url: String,
    job_id: JobId,
    page_limit: Option<usize>,
    ctx: &EffectContext<ServerDeps>,
) -> Result<OrganizationEvent> {
    tracing::info!(
        url = %url,
        job_id = %job_id,
        page_limit = ?page_limit,
        "Starting intelligent site crawl"
    );

    // Create a simple adapter for the firecrawl client
    struct FirecrawlAdapter {
        client: std::sync::Arc<dyn crate::kernel::BaseWebScraper>,
    }

    #[async_trait]
    impl crawler::WebCrawler for FirecrawlAdapter {
        async fn crawl(&self, url: &str, _page_limit: Option<usize>) -> Result<Vec<crawler::CrawlPage>> {
            // For now, use single-page scraping
            // TODO: Implement multi-page crawling with Firecrawl's crawl_url
            let result = self.client.scrape(url).await?;

            Ok(vec![crawler::CrawlPage {
                url: result.url,
                html: String::new(), // Firecrawl doesn't return HTML
                markdown: Some(result.markdown),
                title: result.title,
            }])
        }
    }

    let crawler = FirecrawlAdapter {
        client: ctx.deps().web_scraper.clone(),
    };

    // Crawl the site
    tracing::info!(url = %url, "Crawling site via Firecrawl");

    match crawler::crawl_site(
        &url,
        &crawler,
        ctx.deps().intelligent_crawler.as_ref(),
        page_limit,
    )
    .await
    {
        Ok(snapshot_ids) => {
            tracing::info!(
                url = %url,
                job_id = %job_id,
                snapshot_count = snapshot_ids.len(),
                "Intelligent crawl completed - stored page snapshots"
            );

            Ok(OrganizationEvent::SiteCrawled {
                url,
                job_id,
                snapshot_ids: snapshot_ids.iter().map(|id| id.0).collect(),
            })
        }
        Err(e) => {
            tracing::error!(
                url = %url,
                job_id = %job_id,
                error = %e,
                "Intelligent crawl failed"
            );

            Ok(OrganizationEvent::SiteCrawlFailed {
                url,
                job_id,
                reason: e.to_string(),
            })
        }
    }
}

async fn handle_detect_information(
    snapshot_ids: Vec<uuid::Uuid>,
    job_id: JobId,
    detection_kind: String,
    ctx: &EffectContext<ServerDeps>,
) -> Result<OrganizationEvent> {
    tracing::info!(
        job_id = %job_id,
        snapshot_count = snapshot_ids.len(),
        detection_kind = %detection_kind,
        "Starting intelligent information detection (heuristic + AI)"
    );

    // Create detection config based on kind
    let config = create_detection_config(&detection_kind);

    // TODO: Implement AI detector adapter
    let mut detection_ids = Vec::new();
    let snapshot_count = snapshot_ids.len();

    for snapshot_id in snapshot_ids {
        // Get snapshot from storage
        let snapshot_id = intelligent_crawler::PageSnapshotId(snapshot_id);
        if let Some(snapshot) = ctx
            .deps()
            .intelligent_crawler
            .get_page_snapshot(snapshot_id)
            .await?
        {
            // Run detection (without AI for now)
            if let Some(detection) =
                detector::detect_information(&snapshot, &config, None::<&NoAIDetector>).await?
            {
                // Save detection
                ctx.deps()
                    .intelligent_crawler
                    .save_detection(&detection)
                    .await?;
                detection_ids.push(detection.id.0);
            }
        }
    }

    if detection_ids.is_empty() {
        tracing::warn!(
            job_id = %job_id,
            snapshot_count,
            "No information detected in any snapshots - pages may not contain relevant content"
        );
    } else {
        tracing::info!(
            job_id = %job_id,
            detection_count = detection_ids.len(),
            snapshot_count,
            detection_kind = %detection_kind,
            "Information detection completed successfully"
        );
    }

    Ok(OrganizationEvent::InformationDetected {
        job_id,
        detection_ids,
    })
}

async fn handle_extract_data(
    detection_ids: Vec<uuid::Uuid>,
    job_id: JobId,
    schema_id: uuid::Uuid,
    ctx: &EffectContext<ServerDeps>,
) -> Result<OrganizationEvent> {
    tracing::info!(
        job_id = %job_id,
        detection_count = detection_ids.len(),
        schema_id = %schema_id,
        "Starting data extraction"
    );

    // Get schema
    let schema_id = intelligent_crawler::SchemaId(schema_id);
    let _schema = match ctx
        .deps()
        .intelligent_crawler
        .get_schema(schema_id)
        .await?
    {
        Some(s) => s,
        None => {
            return Ok(OrganizationEvent::DataExtractionFailed {
                job_id,
                reason: format!("Schema not found: {}", schema_id.0),
            });
        }
    };

    // TODO: Implement AI extractor adapter
    let extraction_ids = Vec::new();

    for detection_id in detection_ids {
        let detection_id = intelligent_crawler::DetectionId(detection_id);

        // Get detection
        if let Some(_detection) = ctx
            .deps()
            .intelligent_crawler
            .get_detection(detection_id)
            .await?
        {
            // Get snapshot
            if let Some(_snapshot) = ctx
                .deps()
                .intelligent_crawler
                .get_page_snapshot(_detection.page_snapshot_id)
                .await?
            {
                // Extract data (placeholder - needs AI adapter)
                // For now, skip actual extraction
                tracing::warn!(
                    detection_id = %detection_id.0,
                    "Skipping extraction - AI adapter not implemented"
                );
            }
        }
    }

    tracing::info!(
        job_id = %job_id,
        extraction_count = extraction_ids.len(),
        "Data extraction completed"
    );

    Ok(OrganizationEvent::DataExtracted {
        job_id,
        extraction_ids,
    })
}

async fn handle_resolve_relationships(
    extraction_ids: Vec<uuid::Uuid>,
    job_id: JobId,
    ctx: &EffectContext<ServerDeps>,
) -> Result<OrganizationEvent> {
    tracing::info!(
        job_id = %job_id,
        extraction_count = extraction_ids.len(),
        "Starting relationship resolution"
    );

    // Get extractions
    let mut extractions = Vec::new();
    for extraction_id in extraction_ids {
        let extraction_id = intelligent_crawler::ExtractionId(extraction_id);
        if let Some(extraction) = ctx
            .deps()
            .intelligent_crawler
            .get_extraction(extraction_id)
            .await?
        {
            extractions.push(extraction);
        }
    }

    // Create relationship rules (example rules)
    let rules = vec![
        RelationshipRule::new(
            "organization".to_string(),
            "volunteer_opportunity".to_string(),
            "offers".to_string(),
        )
        .same_page()
        .with_threshold(0.7),
    ];

    // Resolve relationships (without AI for now)
    let relationships =
        relationships::resolve_relationships(&extractions, &rules, None::<&NoAIResolver>).await?;

    // Save relationships
    for relationship in &relationships {
        ctx.deps()
            .intelligent_crawler
            .save_relationship(relationship)
            .await?;
    }

    let relationship_ids: Vec<uuid::Uuid> = relationships.iter().map(|r| r.id.0).collect();

    tracing::info!(
        job_id = %job_id,
        relationship_count = relationship_ids.len(),
        "Relationship resolution completed"
    );

    Ok(OrganizationEvent::RelationshipsResolved {
        job_id,
        relationship_ids,
    })
}

// ============================================================================
// Helper functions
// ============================================================================

fn create_detection_config(kind: &str) -> DetectionConfig {
    match kind {
        "volunteer_opportunity" => DetectionConfig::new(kind.to_string())
            .with_heuristic(Heuristic::Keywords {
                words: vec![
                    "volunteer".to_string(),
                    "volunteers".to_string(),
                    "volunteering".to_string(),
                ],
            })
            .with_heuristic(Heuristic::UrlPattern {
                pattern: "volunteer".to_string(),
            })
            .with_threshold(0.6),
        _ => DetectionConfig::new(kind.to_string()).with_threshold(0.5),
    }
}

// Placeholder structs for AI integration (to be implemented)
struct NoAIDetector;

#[async_trait]
impl detector::AIDetector for NoAIDetector {
    async fn detect(
        &self,
        _content: &str,
        _prompt: &str,
    ) -> Result<(bool, f32, String)> {
        Ok((false, 0.0, String::new()))
    }
}

struct NoAIResolver;

#[async_trait]
impl relationships::AIRelationshipResolver for NoAIResolver {
    async fn resolve(
        &self,
        _from_data: &serde_json::Value,
        _to_data: &serde_json::Value,
        _relationship_type: &str,
    ) -> Result<(bool, f32, String)> {
        Ok((false, 0.0, String::new()))
    }
}
