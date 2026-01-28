use sqlx::PgPool;
use std::sync::Arc;
use twilio::TwilioService;

use crate::kernel::{BaseAI, BaseEmbeddingService, BasePushNotificationService, BaseWebScraper};

/// Server dependencies accessible to effects (using traits for testability)
pub struct ServerDeps {
    pub db_pool: PgPool,
    pub web_scraper: Arc<dyn BaseWebScraper>,
    pub ai: Arc<dyn BaseAI>,
    pub embedding_service: Arc<dyn BaseEmbeddingService>,
    pub push_service: Arc<dyn BasePushNotificationService>,
    pub twilio: Arc<TwilioService>,
}

impl ServerDeps {
    /// Create new ServerDeps with the given dependencies
    pub fn new(
        db_pool: PgPool,
        web_scraper: Arc<dyn BaseWebScraper>,
        ai: Arc<dyn BaseAI>,
        embedding_service: Arc<dyn BaseEmbeddingService>,
        push_service: Arc<dyn BasePushNotificationService>,
        twilio: Arc<TwilioService>,
    ) -> Self {
        Self {
            db_pool,
            web_scraper,
            ai,
            embedding_service,
            push_service,
            twilio,
        }
    }
}
