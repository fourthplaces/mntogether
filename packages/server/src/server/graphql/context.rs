use crate::domains::organization::effects::{FirecrawlClient, NeedExtractor};
use sqlx::PgPool;

/// GraphQL request context
///
/// Contains shared resources available to all resolvers
pub struct GraphQLContext {
    pub pool: PgPool,
    pub firecrawl_client: FirecrawlClient,
    pub need_extractor: NeedExtractor,
    // TODO: Add auth (Clerk verification)
}

impl juniper::Context for GraphQLContext {}

impl GraphQLContext {
    pub fn new(
        pool: PgPool,
        firecrawl_api_key: String,
        openai_api_key: String,
    ) -> Self {
        Self {
            pool,
            firecrawl_client: FirecrawlClient::new(firecrawl_api_key),
            need_extractor: NeedExtractor::new(openai_api_key),
        }
    }
}
