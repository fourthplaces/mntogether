// Effect executor - wires up intelligent-crawler effects with domain adapters
//
// This is the glue layer that connects:
// - intelligent-crawler's infrastructure (FlaggingEffect, ExtractionEffect)
// - Our domain logic (MultiTypeListingEvaluator, ListingAdapter)

use anyhow::Result;
use intelligent_crawler::{
    CrawlerCommand, CrawlerEvent, FlaggingEffect, ExtractionEffect,
    traits::CrawlerStorage as IntelligentCrawlerStorage,
};
use sqlx::PgPool;
use std::sync::Arc;
use tracing::{info, warn};

use crate::kernel::BaseAI;

use super::{ListingAdapter, MultiTypeListingEvaluator};

/// Effect executor that runs intelligent-crawler effects and adapts results
pub struct CrawlerEffectExecutor<AI, Storage>
where
    AI: BaseAI + Send + Sync,
    Storage: IntelligentCrawlerStorage<
        PageId = uuid::Uuid,
        ExtractionRunId = uuid::Uuid,
        ExtractionId = uuid::Uuid,
    >,
{
    flagging_effect: FlaggingEffect<Storage, MultiTypeListingEvaluator<AI>>,
    extraction_effect: ExtractionEffect<Storage, MultiTypeListingEvaluator<AI>>,
    listing_adapter: Arc<ListingAdapter>,
}

impl<AI, Storage> CrawlerEffectExecutor<AI, Storage>
where
    AI: BaseAI + Send + Sync + Clone,
    Storage: IntelligentCrawlerStorage<
        PageId = uuid::Uuid,
        ExtractionRunId = uuid::Uuid,
        ExtractionId = uuid::Uuid,
    > + Clone,
{
    pub fn new(
        ai: AI,
        storage: Storage,
        listings_pool: PgPool,
    ) -> Self {
        // Create separate evaluators for each effect since they take ownership
        let flagging_evaluator = MultiTypeListingEvaluator::new(ai.clone());
        let extraction_evaluator = MultiTypeListingEvaluator::new(ai);

        let flagging_effect = FlaggingEffect::new(
            storage.clone(),
            flagging_evaluator,
        );

        let extraction_effect = ExtractionEffect::new(
            storage,
            extraction_evaluator,
            "v1.0.0".to_string(), // extractor_version
            "v1.0.0".to_string(), // prompt_version
            "gpt-4-turbo".to_string(), // model
        );

        let listing_adapter = Arc::new(ListingAdapter::new(listings_pool));

        Self {
            flagging_effect,
            extraction_effect,
            listing_adapter,
        }
    }

    /// Execute a command and return resulting events
    ///
    /// This method:
    /// 1. Routes command to appropriate effect
    /// 2. Executes the effect
    /// 3. Processes any DataExtracted events through ListingAdapter
    /// 4. Returns all events for coordinator
    pub async fn execute_command(&self, cmd: CrawlerCommand) -> Result<Vec<CrawlerEvent>> {
        let mut events = match &cmd {
            CrawlerCommand::FlagPage { .. } => {
                self.flagging_effect.execute(cmd).await
                    .map_err(|e| anyhow::anyhow!("Flagging effect failed: {}", e))?
            }
            CrawlerCommand::ExtractFromPage { .. } => {
                self.extraction_effect.execute(cmd).await
                    .map_err(|e| anyhow::anyhow!("Extraction effect failed: {}", e))?
            }
            _ => {
                // Other commands not handled by these effects
                return Ok(vec![]);
            }
        };

        // Process DataExtracted events through ListingAdapter
        for event in &events {
            if let CrawlerEvent::DataExtracted {
                page_id,
                extraction_run_id,
                data,
                confidence,
                fingerprint_hint,
                ..
            } = event
            {
                // Convert to RawExtraction and process
                let raw_extraction = intelligent_crawler::RawExtraction {
                    extraction_run_id: *extraction_run_id,
                    page_id: *page_id,
                    page_url: url::Url::parse("https://placeholder.com").unwrap(), // Will be filled by effect
                    data: data.clone(),
                    confidence: *confidence,
                    fingerprint_hint: fingerprint_hint.clone(),
                };

                match self.listing_adapter.process_extraction(&raw_extraction).await {
                    Ok(listing_id) => {
                        info!(
                            extraction_run_id = %extraction_run_id,
                            listing_id = %listing_id,
                            "Successfully created listing from extraction"
                        );
                    }
                    Err(e) => {
                        warn!(
                            extraction_run_id = %extraction_run_id,
                            error = %e,
                            "Failed to create listing from extraction"
                        );
                    }
                }
            }
        }

        Ok(events)
    }

    /// Execute multiple commands in sequence
    pub async fn execute_commands(&self, commands: Vec<CrawlerCommand>) -> Result<Vec<CrawlerEvent>> {
        let mut all_events = Vec::new();

        for cmd in commands {
            let events = self.execute_command(cmd).await?;
            all_events.extend(events);
        }

        Ok(all_events)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: Integration tests would require:
    // - Mock AI implementation
    // - Mock CrawlerStorage implementation
    // - Test database for ListingAdapter
    //
    // These should be in integration tests rather than unit tests.
}
