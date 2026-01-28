// TestDependencies - mock implementations for testing
//
// Provides mock services that can be injected into ServerKernel for tests.

use anyhow::Result;
use async_trait::async_trait;
use seesaw::testing::SpyJobQueue;
use seesaw::EventBus;
use sqlx::PgPool;
use std::sync::{Arc, Mutex};

use super::{
    BaseAI, BaseEmbeddingService, BasePushNotificationService, BaseWebScraper, ScrapeResult,
    ServerKernel,
};

// =============================================================================
// Mock Web Scraper
// =============================================================================

pub struct MockWebScraper {
    responses: Arc<Mutex<Vec<ScrapeResult>>>,
}

impl MockWebScraper {
    pub fn new() -> Self {
        Self {
            responses: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn with_response(self, markdown: &str) -> Self {
        let response = ScrapeResult {
            url: "https://example.org".to_string(),
            markdown: markdown.to_string(),
            title: Some("Test Page".to_string()),
        };
        self.responses.lock().unwrap().push(response);
        self
    }
}

#[async_trait]
impl BaseWebScraper for MockWebScraper {
    async fn scrape(&self, url: &str) -> Result<ScrapeResult> {
        let mut responses = self.responses.lock().unwrap();
        if !responses.is_empty() {
            Ok(responses.remove(0))
        } else {
            Ok(ScrapeResult {
                url: url.to_string(),
                markdown: "# Mock Content\n\nThis is mock scraped content.".to_string(),
                title: Some("Mock Page".to_string()),
            })
        }
    }
}

// =============================================================================
// Mock AI (Generic LLM capabilities)
// =============================================================================

pub struct MockAI {
    responses: Arc<Mutex<Vec<String>>>,
}

impl MockAI {
    pub fn new() -> Self {
        Self {
            responses: Arc::new(Mutex::new(Vec::new())),
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
}

#[async_trait]
impl BaseAI for MockAI {
    async fn complete(&self, _prompt: &str) -> Result<String> {
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
    // Returns a fixed embedding vector for all inputs
    fixed_embedding: Vec<f32>,
}

impl MockEmbeddingService {
    pub fn new() -> Self {
        // Return a simple 1536-dimensional vector for testing
        Self {
            fixed_embedding: vec![0.1; 1536],
        }
    }

    pub fn with_embedding(mut self, embedding: Vec<f32>) -> Self {
        self.fixed_embedding = embedding;
        self
    }
}

#[async_trait]
impl BaseEmbeddingService for MockEmbeddingService {
    async fn generate(&self, _text: &str) -> Result<Vec<f32>> {
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
// TestDependencies - Builder for test dependencies
// =============================================================================

#[derive(Clone)]
pub struct TestDependencies {
    pub web_scraper: Arc<MockWebScraper>,
    pub ai: Arc<MockAI>,
    pub embedding_service: Arc<MockEmbeddingService>,
    pub push_service: Arc<MockPushNotificationService>,
    pub job_queue: Arc<SpyJobQueue>,
}

impl TestDependencies {
    pub fn new() -> Self {
        Self {
            web_scraper: Arc::new(MockWebScraper::new()),
            ai: Arc::new(MockAI::new()),
            embedding_service: Arc::new(MockEmbeddingService::new()),
            push_service: Arc::new(MockPushNotificationService::new()),
            job_queue: Arc::new(SpyJobQueue::new()),
        }
    }

    /// Set a mock web scraper
    pub fn mock_scraper(mut self, scraper: MockWebScraper) -> Self {
        self.web_scraper = Arc::new(scraper);
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

    /// Convert into a ServerKernel for testing
    pub fn into_kernel(self, db_pool: PgPool) -> Arc<ServerKernel> {
        Arc::new(ServerKernel::new(
            db_pool,
            self.web_scraper,
            self.ai,
            self.embedding_service,
            self.push_service,
            EventBus::new(),
            self.job_queue,
        ))
    }
}

impl Default for TestDependencies {
    fn default() -> Self {
        Self::new()
    }
}
