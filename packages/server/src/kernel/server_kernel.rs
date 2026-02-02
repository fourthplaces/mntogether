// ServerKernel - core infrastructure with all dependencies
//
// The ServerKernel holds all server dependencies (database, APIs, services)
// and provides access via traits for testability.

use seesaw_core::EventBus;
use sqlx::PgPool;
use std::sync::Arc;

use super::{
    job_queue::JobQueue, BaseAI, BaseEmbeddingService, BasePiiDetector, BasePushNotificationService,
    BaseSearchService, BaseWebScraper,
};

/// ServerKernel holds all server dependencies
pub struct ServerKernel {
    pub db_pool: PgPool,
    pub web_scraper: Arc<dyn BaseWebScraper>,
    pub ai: Arc<dyn BaseAI>,
    pub embedding_service: Arc<dyn BaseEmbeddingService>,
    pub push_service: Arc<dyn BasePushNotificationService>,
    pub search_service: Arc<dyn BaseSearchService>,
    pub pii_detector: Arc<dyn BasePiiDetector>,
    /// Shared event bus for all engines and edges
    pub bus: EventBus,
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
        bus: EventBus,
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
            bus,
            job_queue,
        }
    }
}
