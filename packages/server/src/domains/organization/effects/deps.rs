use sqlx::PgPool;
use std::sync::Arc;
use twilio::TwilioService;

use crate::common::auth::HasAuthContext;
use crate::kernel::{BaseAI, BaseEmbeddingService, BasePushNotificationService, BaseWebScraper};

/// Server dependencies accessible to effects (using traits for testability)
pub struct ServerDeps {
    pub db_pool: PgPool,
    pub web_scraper: Arc<dyn BaseWebScraper>,
    pub ai: Arc<dyn BaseAI>,
    pub embedding_service: Arc<dyn BaseEmbeddingService>,
    pub push_service: Arc<dyn BasePushNotificationService>,
    pub twilio: Arc<TwilioService>,
    pub test_identifier_enabled: bool,
    pub admin_identifiers: Vec<String>,
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
        test_identifier_enabled: bool,
        admin_identifiers: Vec<String>,
    ) -> Self {
        Self {
            db_pool,
            web_scraper,
            ai,
            embedding_service,
            push_service,
            twilio,
            test_identifier_enabled,
            admin_identifiers,
        }
    }
}

/// Implement HasAuthContext for ServerDeps to enable authorization checks
impl HasAuthContext for ServerDeps {
    fn admin_identifiers(&self) -> &[String] {
        &self.admin_identifiers
    }

    fn test_identifier_enabled(&self) -> bool {
        self.test_identifier_enabled
    }
}
