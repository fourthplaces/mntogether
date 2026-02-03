// ServerKernel - core infrastructure with all dependencies
//
// The ServerKernel holds all server dependencies (database, APIs, services)
// and provides access via traits for testability.
//
// NOTE: In seesaw 0.6.0, EventBus is removed. Engine is used directly with
// engine.activate(state) pattern. ServerKernel is kept for scheduled tasks
// and background services that need access to dependencies.

use sqlx::PgPool;
use std::sync::Arc;

use super::{
    job_queue::JobQueue, BaseAI, BaseEmbeddingService, BasePiiDetector,
    BasePushNotificationService, BaseSearchService, BaseWebScraper,
};

/// ServerKernel holds all server dependencies
///
/// NOTE: EventBus is removed in seesaw 0.6.0. Scheduled tasks should use
/// a reference to the Engine and call engine.activate() to emit events.
pub struct ServerKernel {
    pub db_pool: PgPool,
    pub web_scraper: Arc<dyn BaseWebScraper>,
    pub ai: Arc<dyn BaseAI>,
    pub embedding_service: Arc<dyn BaseEmbeddingService>,
    pub push_service: Arc<dyn BasePushNotificationService>,
    pub search_service: Arc<dyn BaseSearchService>,
    pub pii_detector: Arc<dyn BasePiiDetector>,
    /// Job queue for background command execution
    pub job_queue: Arc<dyn JobQueue>,
}

impl ServerKernel {
    /// Creates a new ServerKernel with the given dependencies
    pub fn new(
        db_pool: PgPool,
        web_scraper: Arc<dyn BaseWebScraper>,
        ai: Arc<dyn BaseAI>,
        embedding_service: Arc<dyn BaseEmbeddingService>,
        push_service: Arc<dyn BasePushNotificationService>,
        search_service: Arc<dyn BaseSearchService>,
        pii_detector: Arc<dyn BasePiiDetector>,
        job_queue: Arc<dyn JobQueue>,
    ) -> Self {
        Self {
            db_pool,
            web_scraper,
            ai,
            embedding_service,
            push_service,
            search_service,
            pii_detector,
            job_queue,
        }
    }
}
