//! Server dependencies for effects (using traits for testability)
//!
//! This module provides the central dependency container used by all domain effects.
//! All external services use trait abstractions to enable testing.

use ai_client::OpenAi;
use anyhow::Result;
use async_trait::async_trait;
use sqlx::PgPool;
use std::sync::Arc;
use twilio::TwilioService;

use crate::common::auth::HasAuthContext;
use crate::domains::auth::JwtService;
use crate::domains::memo::MemoBuilder;
use crate::kernel::{
    stream_hub::StreamHub, BaseEmbeddingService, BasePiiDetector, BaseTwilioService,
};

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
    /// AI client for all LLM operations. Callers pass specific model constants
    /// (GPT_5_MINI, GPT_5, "gpt-4o") to select the model per-call.
    pub ai: Arc<OpenAi>,
    pub embedding_service: Arc<dyn BaseEmbeddingService>,
    pub twilio: Arc<dyn BaseTwilioService>,
    pub pii_detector: Arc<dyn BasePiiDetector>,
    /// JWT service for token creation
    pub jwt_service: Arc<JwtService>,
    /// In-process pub/sub hub for real-time streaming to SSE endpoints
    pub stream_hub: StreamHub,
    pub test_identifier_enabled: bool,
    pub admin_identifiers: Vec<String>,
}

impl ServerDeps {
    /// Create new ServerDeps with the given dependencies
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        db_pool: PgPool,
        ai: Arc<OpenAi>,
        embedding_service: Arc<dyn BaseEmbeddingService>,
        twilio: Arc<dyn BaseTwilioService>,
        pii_detector: Arc<dyn BasePiiDetector>,
        jwt_service: Arc<JwtService>,
        stream_hub: StreamHub,
        test_identifier_enabled: bool,
        admin_identifiers: Vec<String>,
    ) -> Self {
        Self {
            db_pool,
            ai,
            embedding_service,
            twilio,
            pii_detector,
            jwt_service,
            stream_hub,
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
