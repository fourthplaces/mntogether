// ServerKernel - core infrastructure with all dependencies
//
// The ServerKernel holds all server dependencies (database, APIs, services)
// and provides access via traits for testability.
//
// NOTE: In seesaw 0.6.0, EventBus is removed. Engine is used directly with
// engine.activate(state) pattern. ServerKernel is kept for scheduled tasks
// and background services that need access to dependencies.

use openai_client::OpenAIClient;
use sqlx::PgPool;
use std::sync::Arc;

use super::{
    job_queue::JobQueue, BaseEmbeddingService, BasePiiDetector, BasePushNotificationService,
};

// Import from extraction library
use extraction::{Ingestor, WebSearcher};

/// ServerKernel holds all server dependencies
///
/// NOTE: EventBus is removed in seesaw 0.6.0. Scheduled tasks should use
/// a reference to the Engine and call engine.activate() to emit events.
pub struct ServerKernel {
    pub db_pool: PgPool,
    /// Ingestor for crawling/scraping (from extraction library)
    pub ingestor: Arc<dyn Ingestor>,
    /// OpenAI client for LLM operations
    pub ai: Arc<OpenAIClient>,
    pub embedding_service: Arc<dyn BaseEmbeddingService>,
    pub push_service: Arc<dyn BasePushNotificationService>,
    /// Web searcher for discovery (from extraction library)
    pub web_searcher: Arc<dyn WebSearcher>,
    pub pii_detector: Arc<dyn BasePiiDetector>,
    /// Job queue for background command execution
    pub job_queue: Arc<dyn JobQueue>,
}

impl ServerKernel {
    /// Creates a new ServerKernel with the given dependencies
    pub fn new(
        db_pool: PgPool,
        ingestor: Arc<dyn Ingestor>,
        ai: Arc<OpenAIClient>,
        embedding_service: Arc<dyn BaseEmbeddingService>,
        push_service: Arc<dyn BasePushNotificationService>,
        web_searcher: Arc<dyn WebSearcher>,
        pii_detector: Arc<dyn BasePiiDetector>,
        job_queue: Arc<dyn JobQueue>,
    ) -> Self {
        Self {
            db_pool,
            ingestor,
            ai,
            embedding_service,
            push_service,
            web_searcher,
            pii_detector,
            job_queue,
        }
    }
}
