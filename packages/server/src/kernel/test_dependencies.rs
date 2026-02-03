// TestDependencies - mock implementations for testing
//
// Provides mock services that can be injected into ServerKernel for tests.

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

use super::{
    job_queue::{JobQueue, JobSpec},
    BaseAI, BaseEmbeddingService, BasePiiDetector, BasePushNotificationService, PiiScrubResult,
    ServerKernel,
};
use crate::common::pii::{DetectionContext, PiiFindings, RedactionStrategy};

// Import from extraction library
use extraction::{MockIngestor, MockWebSearcher};

// =============================================================================
// Spy Job Queue (for testing)
// =============================================================================

/// Spy job queue that records all enqueued jobs for testing
pub struct SpyJobQueue {
    jobs: Arc<Mutex<Vec<(serde_json::Value, JobSpec)>>>,
}

impl SpyJobQueue {
    pub fn new() -> Self {
        Self {
            jobs: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Get all enqueued jobs
    pub fn jobs(&self) -> Vec<(serde_json::Value, JobSpec)> {
        self.jobs.lock().unwrap().clone()
    }

    /// Check if a job of the given type was enqueued
    pub fn has_job(&self, job_type: &str) -> bool {
        self.jobs
            .lock()
            .unwrap()
            .iter()
            .any(|(_, spec)| spec.job_type == job_type)
    }
}

impl Default for SpyJobQueue {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl JobQueue for SpyJobQueue {
    async fn enqueue(&self, payload: serde_json::Value, spec: JobSpec) -> Result<Uuid> {
        self.jobs.lock().unwrap().push((payload, spec));
        Ok(Uuid::new_v4())
    }

    async fn schedule(
        &self,
        payload: serde_json::Value,
        spec: JobSpec,
        _run_at: DateTime<Utc>,
    ) -> Result<Uuid> {
        self.jobs.lock().unwrap().push((payload, spec));
        Ok(Uuid::new_v4())
    }
}

// =============================================================================
// Mock AI (Generic LLM capabilities)
// =============================================================================

pub struct MockAI {
    responses: Arc<Mutex<Vec<String>>>,
    calls: Arc<Mutex<Vec<String>>>,
}

impl MockAI {
    pub fn new() -> Self {
        Self {
            responses: Arc::new(Mutex::new(Vec::new())),
            calls: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Add a text response to the queue
    pub fn with_response(self, response: impl Into<String>) -> Self {
        self.responses.lock().unwrap().push(response.into());
        self
    }

    /// Add a JSON response to the queue (will be serialized)
    pub fn with_json_response<T: serde::Serialize>(self, data: &T) -> Self {
        let json = serde_json::to_string(data).expect("Failed to serialize mock response");
        self.responses.lock().unwrap().push(json);
        self
    }

    /// Get all prompts that were sent to the AI
    pub fn calls(&self) -> Vec<String> {
        self.calls.lock().unwrap().clone()
    }

    /// Get the last prompt sent to the AI
    pub fn last_prompt(&self) -> Option<String> {
        self.calls.lock().unwrap().last().cloned()
    }

    /// Check if a prompt containing the given text was sent
    pub fn was_called_with(&self, text: &str) -> bool {
        self.calls.lock().unwrap().iter().any(|p| p.contains(text))
    }

    /// Get the number of times the AI was called
    pub fn call_count(&self) -> usize {
        self.calls.lock().unwrap().len()
    }
}

#[async_trait]
impl BaseAI for MockAI {
    async fn complete(&self, prompt: &str) -> Result<String> {
        // Record the call
        self.calls.lock().unwrap().push(prompt.to_string());

        let mut responses = self.responses.lock().unwrap();
        if !responses.is_empty() {
            Ok(responses.remove(0))
        } else {
            // Return default mock response
            Ok("Mock AI response".to_string())
        }
    }

    async fn complete_json(&self, prompt: &str) -> Result<String> {
        // Same as complete - returns JSON string
        self.complete(prompt).await
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
    pub ai: Arc<MockAI>,
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
            ai: Arc::new(MockAI::new()),
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

    /// Set a mock AI
    pub fn mock_ai(mut self, ai: MockAI) -> Self {
        self.ai = Arc::new(ai);
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
