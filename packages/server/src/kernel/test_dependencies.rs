// TestDependencies - mock implementations for testing
//
// Provides mock services that can be injected into ServerKernel for tests.

use anyhow::Result;
use async_trait::async_trait;
use openai_client::OpenAIClient;
use sqlx::PgPool;
use std::sync::{Arc, Mutex};

use super::{
    jobs::SpyJobQueue, BaseEmbeddingService, BasePiiDetector, BasePushNotificationService,
    PiiScrubResult, ServerKernel,
};
use crate::common::pii::{DetectionContext, PiiFindings, RedactionStrategy};

// Import from extraction library
use extraction::{MockIngestor, MockWebSearcher};

// =============================================================================
// Mock AI Client (for testing)
// =============================================================================

/// Create a mock OpenAIClient for testing.
///
/// This creates a real OpenAIClient with a dummy API key. In tests, you should
/// use mockito or wiremock to intercept HTTP requests, or use integration tests
/// with a real API key (ignored by default).
///
/// For most unit tests, the AI calls should be abstracted behind service boundaries
/// that can be mocked at a higher level.
pub fn mock_openai_client() -> Arc<OpenAIClient> {
    Arc::new(OpenAIClient::new("sk-test-mock-key-for-testing"))
}

/// Legacy MockAI for test compatibility.
///
/// DEPRECATED: This struct exists only for backwards compatibility with existing tests.
/// Tests should be migrated to use HTTP-level mocking (mockito/wiremock) for the
/// OpenAI API endpoints instead.
///
/// The with_response() calls are now no-ops - tests using this will need to be
/// marked as #[ignore] until they're updated.
#[derive(Clone)]
pub struct MockAI {
    responses: Vec<String>,
}

impl MockAI {
    pub fn new() -> Self {
        Self {
            responses: Vec::new(),
        }
    }

    /// Add a canned response (no-op in new architecture - use HTTP mocking instead)
    pub fn with_response(mut self, response: impl Into<String>) -> Self {
        self.responses.push(response.into());
        self
    }
}

impl Default for MockAI {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Mock Embedding Service
// =============================================================================

pub struct MockEmbeddingService {
    // Returns a fixed embedding vector for all inputs by default
    fixed_embedding: Vec<f32>,
    // Map of text patterns to embeddings for deduplication testing
    pattern_embeddings: Arc<Mutex<Vec<(String, Vec<f32>)>>>,
    // Track all texts that embeddings were generated for
    calls: Arc<Mutex<Vec<String>>>,
}

impl MockEmbeddingService {
    pub fn new() -> Self {
        // Return a simple 1536-dimensional vector for testing
        Self {
            fixed_embedding: vec![0.1; 1536],
            pattern_embeddings: Arc::new(Mutex::new(Vec::new())),
            calls: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn with_embedding(mut self, embedding: Vec<f32>) -> Self {
        self.fixed_embedding = embedding;
        self
    }

    /// Add a pattern-based embedding: when text contains the pattern, return this embedding
    pub fn with_pattern_embedding(self, pattern: &str, embedding: Vec<f32>) -> Self {
        self.pattern_embeddings
            .lock()
            .unwrap()
            .push((pattern.to_string(), embedding));
        self
    }

    /// Create embeddings that will make two texts appear similar (for dedup testing)
    /// Returns embeddings with high cosine similarity (>0.90)
    pub fn with_similar_texts(self, text1_pattern: &str, text2_pattern: &str) -> Self {
        // Create two very similar embeddings (cosine similarity ~0.95)
        let base: Vec<f32> = (0..1536).map(|i| (i as f32 * 0.01).sin()).collect();
        let similar: Vec<f32> = base.iter().map(|v| v + 0.01).collect();

        self.with_pattern_embedding(text1_pattern, base)
            .with_pattern_embedding(text2_pattern, similar)
    }

    /// Create embeddings that will make texts appear different (for non-dedup testing)
    pub fn with_different_texts(self, patterns: Vec<&str>) -> Self {
        let mut result = self;
        for (i, pattern) in patterns.into_iter().enumerate() {
            // Create distinctly different embeddings
            let embedding: Vec<f32> = (0..1536)
                .map(|j| ((i * 100 + j) as f32 * 0.1).sin())
                .collect();
            result = result.with_pattern_embedding(pattern, embedding);
        }
        result
    }

    /// Get all texts that embeddings were generated for
    pub fn calls(&self) -> Vec<String> {
        self.calls.lock().unwrap().clone()
    }
}

#[async_trait]
impl BaseEmbeddingService for MockEmbeddingService {
    async fn generate(&self, text: &str) -> Result<Vec<f32>> {
        // Record the call
        self.calls.lock().unwrap().push(text.to_string());

        // Check for pattern match first
        let patterns = self.pattern_embeddings.lock().unwrap();
        for (pattern, embedding) in patterns.iter() {
            if text.to_lowercase().contains(&pattern.to_lowercase()) {
                return Ok(embedding.clone());
            }
        }
        drop(patterns);

        // Fall back to fixed embedding
        Ok(self.fixed_embedding.clone())
    }
}

// =============================================================================
// Mock Push Notification Service
// =============================================================================

pub struct MockPushNotificationService {
    sent_notifications: Arc<Mutex<Vec<(String, String, String, serde_json::Value)>>>,
}

impl MockPushNotificationService {
    pub fn new() -> Self {
        Self {
            sent_notifications: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Get all notifications that were sent
    pub fn sent_notifications(&self) -> Vec<(String, String, String, serde_json::Value)> {
        self.sent_notifications.lock().unwrap().clone()
    }

    /// Check if a notification was sent with the given title
    pub fn was_sent_with_title(&self, title: &str) -> bool {
        self.sent_notifications
            .lock()
            .unwrap()
            .iter()
            .any(|(_, t, _, _)| t == title)
    }
}

#[async_trait]
impl BasePushNotificationService for MockPushNotificationService {
    async fn send_notification(
        &self,
        push_token: &str,
        title: &str,
        body: &str,
        data: serde_json::Value,
    ) -> Result<()> {
        self.sent_notifications.lock().unwrap().push((
            push_token.to_string(),
            title.to_string(),
            body.to_string(),
            data,
        ));
        Ok(())
    }

    async fn send_batch(
        &self,
        notifications: Vec<(&str, &str, &str, serde_json::Value)>,
    ) -> Result<()> {
        for (token, title, body, data) in notifications {
            self.send_notification(token, title, body, data).await?;
        }
        Ok(())
    }
}

// =============================================================================
// Mock PII Detector
// =============================================================================

pub struct MockPiiDetector {
    scrub_enabled: bool,
}

impl MockPiiDetector {
    pub fn new() -> Self {
        Self {
            scrub_enabled: true,
        }
    }

    pub fn disabled() -> Self {
        Self {
            scrub_enabled: false,
        }
    }
}

#[async_trait]
impl BasePiiDetector for MockPiiDetector {
    async fn detect(&self, text: &str, context: DetectionContext) -> Result<PiiFindings> {
        if self.scrub_enabled {
            // Use real detection for tests
            Ok(crate::common::pii::detect_pii_contextual(text, context))
        } else {
            Ok(PiiFindings::new())
        }
    }

    async fn scrub(
        &self,
        text: &str,
        context: DetectionContext,
        strategy: RedactionStrategy,
    ) -> Result<PiiScrubResult> {
        if self.scrub_enabled {
            let findings = self.detect(text, context).await?;
            let pii_detected = !findings.is_empty();
            let clean_text = crate::common::pii::redact_pii(text, &findings, strategy);

            Ok(PiiScrubResult {
                clean_text,
                findings,
                pii_detected,
            })
        } else {
            Ok(PiiScrubResult {
                clean_text: text.to_string(),
                findings: PiiFindings::new(),
                pii_detected: false,
            })
        }
    }
}

// =============================================================================
// TestDependencies - Builder for test dependencies
// =============================================================================

#[derive(Clone)]
pub struct TestDependencies {
    pub ingestor: Arc<MockIngestor>,
    pub ai: Arc<OpenAIClient>,
    pub embedding_service: Arc<MockEmbeddingService>,
    pub push_service: Arc<MockPushNotificationService>,
    pub web_searcher: Arc<MockWebSearcher>,
    pub pii_detector: Arc<MockPiiDetector>,
    pub job_queue: Arc<SpyJobQueue>,
}

impl TestDependencies {
    pub fn new() -> Self {
        Self {
            ingestor: Arc::new(MockIngestor::new()),
            ai: mock_openai_client(),
            embedding_service: Arc::new(MockEmbeddingService::new()),
            push_service: Arc::new(MockPushNotificationService::new()),
            web_searcher: Arc::new(MockWebSearcher::new()),
            pii_detector: Arc::new(MockPiiDetector::new()),
            job_queue: Arc::new(SpyJobQueue::new()),
        }
    }

    /// Set a mock ingestor (for crawling/scraping)
    pub fn mock_ingestor(mut self, ingestor: MockIngestor) -> Self {
        self.ingestor = Arc::new(ingestor);
        self
    }

    /// Set an OpenAI client (can be configured with a test server URL)
    pub fn with_ai(mut self, ai: Arc<OpenAIClient>) -> Self {
        self.ai = ai;
        self
    }

    /// Legacy method for test compatibility.
    ///
    /// DEPRECATED: MockAI is a no-op stub. Tests using this should be marked as
    /// #[ignore] and migrated to use HTTP-level mocking instead.
    #[allow(unused_variables)]
    pub fn mock_ai(self, mock_ai: MockAI) -> Self {
        // MockAI is now a no-op - the OpenAI client is used directly
        // Tests relying on canned AI responses need to be updated to use
        // HTTP mocking (mockito/wiremock) for the OpenAI API
        self
    }

    /// Set a mock embedding service
    pub fn mock_embeddings(mut self, service: MockEmbeddingService) -> Self {
        self.embedding_service = Arc::new(service);
        self
    }

    /// Set a mock push notification service
    pub fn mock_push(mut self, service: MockPushNotificationService) -> Self {
        self.push_service = Arc::new(service);
        self
    }

    /// Set a mock web searcher
    pub fn mock_web_searcher(mut self, searcher: MockWebSearcher) -> Self {
        self.web_searcher = Arc::new(searcher);
        self
    }

    /// Set a mock PII detector
    pub fn mock_pii(mut self, detector: MockPiiDetector) -> Self {
        self.pii_detector = Arc::new(detector);
        self
    }

    /// Convert into a ServerKernel for testing
    ///
    /// NOTE: In seesaw 0.6.0, EventBus is removed. Tests should create
    /// an Engine and use engine.activate() to emit events.
    pub fn into_kernel(self, db_pool: PgPool) -> Arc<ServerKernel> {
        Arc::new(ServerKernel::new(
            db_pool,
            self.ingestor,
            self.ai,
            self.embedding_service,
            self.push_service,
            self.web_searcher,
            self.pii_detector,
            self.job_queue,
        ))
    }
}

impl Default for TestDependencies {
    fn default() -> Self {
        Self::new()
    }
}
