use sqlx::PgPool;

use crate::common::utils::{EmbeddingService, ExpoClient};
use super::utils::{FirecrawlClient, NeedExtractor};

/// Server dependencies accessible to effects
pub struct ServerDeps {
    pub db_pool: PgPool,
    pub firecrawl_client: FirecrawlClient,
    pub need_extractor: NeedExtractor,
    pub embedding_service: EmbeddingService,
    pub expo_client: ExpoClient,
}
