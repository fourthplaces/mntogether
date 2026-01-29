use uuid::Uuid;

use crate::{
    commands::CrawlerCommand,
    events::CrawlerEvent,
    new_types::*,
    traits::{CrawlerStorage, PageEvaluator},
};

/// Extraction effect handler (executes ExtractFromPage command)
pub struct ExtractionEffect<S, E> {
    storage: S,
    evaluator: E,
    extractor_version: String,
    prompt_version: String,
    model: String,
}

impl<S, E> ExtractionEffect<S, E>
where
    S: CrawlerStorage<PageId = Uuid, ExtractionRunId = Uuid, ExtractionId = Uuid>, // ✅ Constrain to Uuid
    E: PageEvaluator,
{
    pub fn new(
        storage: S,
        evaluator: E,
        extractor_version: String,
        prompt_version: String,
        model: String,
    ) -> Self {
        Self {
            storage,
            evaluator,
            extractor_version,
            prompt_version,
            model,
        }
    }

    pub async fn execute(
        &self,
        cmd: CrawlerCommand,
    ) -> Result<Vec<CrawlerEvent>, Box<dyn std::error::Error + Send + Sync>> {
        match cmd {
            CrawlerCommand::ExtractFromPage { page_id } => {
                self.extract_from_page(page_id).await
            }
            _ => Ok(vec![]),
        }
    }

    async fn extract_from_page(
        &self,
        page_id: Uuid,
    ) -> Result<Vec<CrawlerEvent>, Box<dyn std::error::Error + Send + Sync>> {
        let mut events = Vec::new();

        let page = self
            .storage
            .get_page(page_id)
            .await?
            .ok_or("Page not found")?;

        // Create extraction run
        let run = ExtractionRun {
            page_id,
            page_content_hash: page.content_hash.clone(),
            extractor_version: self.extractor_version.clone(),
            prompt_version: self.prompt_version.clone(),
            model: self.model.clone(),
        };

        let run_id = self.storage.create_extraction_run(run).await?;

        events.push(CrawlerEvent::ExtractionStarted {
            page_id,
            extraction_run_id: run_id,
        });

        // Extract data
        let content = PageContent {
            url: page.url.clone(),
            html: page.html,
            markdown: page.markdown,
            content_hash: page.content_hash,
        };

        let raw_extractions = match self.evaluator.extract_data(&content).await {
            Ok(extractions) => extractions,
            Err(e) => {
                events.push(CrawlerEvent::ExtractionFailed {
                    page_id,
                    extraction_run_id: run_id,
                    error: e.to_string(),
                });
                return Ok(events);
            }
        };

        // Store extractions and emit events
        for raw_extraction in &raw_extractions {
            let extraction_id = self
                .storage
                .insert_extraction(raw_extraction.clone(), run_id)
                .await?;

            events.push(CrawlerEvent::DataExtracted {
                page_id,
                extraction_run_id: run_id,
                extraction_id,
                data: raw_extraction.data.clone(),
                confidence: raw_extraction.confidence,
                fingerprint_hint: raw_extraction.fingerprint_hint.clone(),
            });
        }

        // Finish run
        let stats = ExtractionStats {
            items_found: raw_extractions.len(),
            items_created: raw_extractions.len(),
            items_updated: 0,
        };

        self.storage.finish_extraction_run(run_id, stats).await?;

        events.push(CrawlerEvent::ExtractionCompleted {
            page_id,
            extraction_run_id: run_id,
            items_found: raw_extractions.len(),
        });

        Ok(events)
    }
}
