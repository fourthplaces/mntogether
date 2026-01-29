use uuid::Uuid;

use crate::{
    commands::CrawlerCommand,
    events::{CrawlerEvent, FlagSource},
    new_types::*,
    traits::{CrawlerStorage, PageEvaluator},
};

/// Flagging effect handler (executes FlagPage command)
pub struct FlaggingEffect<S, E> {
    storage: S,
    evaluator: E,
}

impl<S, E> FlaggingEffect<S, E>
where
    S: CrawlerStorage<PageId = Uuid>, // ✅ Constrain to Uuid
    E: PageEvaluator,
{
    pub fn new(storage: S, evaluator: E) -> Self {
        Self { storage, evaluator }
    }

    pub async fn execute(
        &self,
        cmd: CrawlerCommand,
    ) -> Result<Vec<CrawlerEvent>, Box<dyn std::error::Error + Send + Sync>> {
        match cmd {
            CrawlerCommand::FlagPage { page_id } => self.flag_page(page_id).await,
            _ => Ok(vec![]),
        }
    }

    async fn flag_page(
        &self,
        page_id: Uuid,
    ) -> Result<Vec<CrawlerEvent>, Box<dyn std::error::Error + Send + Sync>> {
        let page = self
            .storage
            .get_page(page_id)
            .await?
            .ok_or("Page not found")?;

        // Pre-filter check (before moving page data)
        let should_evaluate = self.evaluator.pre_filter(
            &page.url,
            &page.markdown[..page.markdown.len().min(2000)],
        );

        if !should_evaluate {
            let result = FlagResult {
                status: FlagStatus::Unflagged,
                source: Some(FlagSource::Rule),
                confidence: Some(0.0),
                reason: Some("Pre-filter rejected".to_string()),
            };

            self.storage.update_page_flag(page_id, result, None).await?;

            let url = page.url.clone();
            return Ok(vec![CrawlerEvent::PageUnflagged {
                page_id,
                url,
                reason: "Pre-filter rejected".to_string(),
            }]);
        }

        // Create content for AI evaluation
        let content = PageContent {
            url: page.url.clone(),
            html: page.html,
            markdown: page.markdown,
            content_hash: page.content_hash,
        };

        // AI evaluation
        let decision = match self.evaluator.should_flag(&content).await {
            Ok(d) => d,
            Err(e) => {
                // Emit failure event
                return Ok(vec![CrawlerEvent::PageFlaggingFailed {
                    page_id,
                    error: e.to_string(),
                }]);
            }
        };

        if decision.should_flag && decision.confidence >= 0.4 {
            let result = FlagResult {
                status: FlagStatus::Flagged,
                source: Some(decision.source),
                confidence: Some(decision.confidence),
                reason: Some(decision.reason.clone()),
            };

            self.storage.update_page_flag(page_id, result, None).await?;

            Ok(vec![CrawlerEvent::PageFlagged {
                page_id,
                url: page.url,
                flagged_by: decision.source,
                confidence: decision.confidence,
                reason: decision.reason,
            }])
        } else {
            let result = FlagResult {
                status: FlagStatus::Unflagged,
                source: Some(decision.source),
                confidence: Some(decision.confidence),
                reason: Some(decision.reason.clone()),
            };

            self.storage.update_page_flag(page_id, result, None).await?;

            Ok(vec![CrawlerEvent::PageUnflagged {
                page_id,
                url: page.url,
                reason: decision.reason,
            }])
        }
    }
}
