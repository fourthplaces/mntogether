//! Server dependencies for effects (using traits for testability)
//!
//! This module provides the central dependency container used by all domain effects.
//! All external services use trait abstractions to enable testing.

use ai_client::{Claude, OpenAi};
use anyhow::Result;
use apify_client::ApifyClient;
use async_trait::async_trait;
use sqlx::PgPool;
use std::sync::Arc;
use twilio::TwilioService;

use crate::common::auth::HasAuthContext;
use crate::domains::auth::JwtService;
use crate::domains::memo::MemoBuilder;
use crate::kernel::{
    extraction_service::OpenAIExtractionService, stream_hub::StreamHub, BaseEmbeddingService,
    BasePiiDetector, BasePushNotificationService, BaseTwilioService,
};

// Import from extraction library
use extraction::{Ingestor, WebSearcher};

// =============================================================================
// TwilioService Adapter (implements BaseTwilioService trait)
// =============================================================================

/// Wrapper around TwilioService that implements BaseTwilioService trait
pub struct TwilioAdapter(pub Arc<TwilioService>);

impl TwilioAdapter {
    pub fn new(service: Arc<TwilioService>) -> Self {
        Self(service)
    }
}

#[async_trait]
impl BaseTwilioService for TwilioAdapter {
    async fn send_otp(&self, phone_number: &str) -> Result<()> {
        self.0
            .send_otp(phone_number)
            .await
            .map(|_| ())
            .map_err(|e| anyhow::anyhow!("{}", e))
    }

    async fn verify_otp(&self, phone_number: &str, code: &str) -> Result<()> {
        self.0
            .verify_otp(phone_number, code)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))
    }
}

// =============================================================================
// ServerDeps
// =============================================================================

/// Server dependencies accessible to effects (using traits for testability)
#[derive(Clone)]
pub struct ServerDeps {
    pub db_pool: PgPool,
    /// DEPRECATED: Use ExtractionService methods with specific ingestors instead.
    /// This field is retained for backward compatibility with deprecated code paths.
    /// Use `extraction.ingest()` or `extraction.ingest_urls()` with FirecrawlIngestor/HttpIngestor.
    pub ingestor: Arc<dyn Ingestor>,
    /// AI client for all LLM operations. Callers pass specific model constants
    /// (GPT_5_MINI, GPT_5, "gpt-4o") to select the model per-call.
    pub ai: Arc<OpenAi>,
    /// Claude client for Anthropic models (optional — needs ANTHROPIC_API_KEY).
    pub claude: Option<Arc<Claude>>,
    pub embedding_service: Arc<dyn BaseEmbeddingService>,
    pub push_service: Arc<dyn BasePushNotificationService>,
    pub twilio: Arc<dyn BaseTwilioService>,
    /// Web searcher for discovery (from extraction library)
    pub web_searcher: Arc<dyn WebSearcher>,
    pub pii_detector: Arc<dyn BasePiiDetector>,
    /// Extraction service for query-driven content extraction (optional for tests)
    pub extraction: Option<Arc<OpenAIExtractionService>>,
    /// JWT service for token creation
    pub jwt_service: Arc<JwtService>,
    /// In-process pub/sub hub for real-time streaming to SSE endpoints
    pub stream_hub: StreamHub,
    /// Apify client for social media scraping (optional — not all envs need it)
    pub apify_client: Option<Arc<ApifyClient>>,
    pub test_identifier_enabled: bool,
    pub admin_identifiers: Vec<String>,
}

impl ServerDeps {
    /// Create new ServerDeps with the given dependencies
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        db_pool: PgPool,
        ingestor: Arc<dyn Ingestor>,
        ai: Arc<OpenAi>,
        claude: Option<Arc<Claude>>,
        embedding_service: Arc<dyn BaseEmbeddingService>,
        push_service: Arc<dyn BasePushNotificationService>,
        twilio: Arc<dyn BaseTwilioService>,
        web_searcher: Arc<dyn WebSearcher>,
        pii_detector: Arc<dyn BasePiiDetector>,
        extraction: Option<Arc<OpenAIExtractionService>>,
        jwt_service: Arc<JwtService>,
        stream_hub: StreamHub,
        apify_client: Option<Arc<ApifyClient>>,
        test_identifier_enabled: bool,
        admin_identifiers: Vec<String>,
    ) -> Self {
        Self {
            db_pool,
            ingestor,
            ai,
            claude,
            embedding_service,
            push_service,
            twilio,
            web_searcher,
            pii_detector,
            extraction,
            jwt_service,
            stream_hub,
            apify_client,
            test_identifier_enabled,
            admin_identifiers,
        }
    }
}

impl ServerDeps {
    /// Create a memoized computation builder.
    ///
    /// ```ignore
    /// let result: MyType = deps.memo("my_func_v1", &input)
    ///     .ttl(86_400_000) // optional, ms
    ///     .get_or(|| async { expensive_call().await })
    ///     .await?;
    /// ```
    pub fn memo<'a, K: serde::Serialize>(
        &'a self,
        function_name: &'a str,
        key: K,
    ) -> MemoBuilder<'a, K> {
        MemoBuilder::new(function_name, key, &self.db_pool)
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
