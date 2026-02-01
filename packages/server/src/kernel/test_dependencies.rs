// TestDependencies - mock implementations for testing
//
// Provides mock services that can be injected into ServerKernel for tests.

use anyhow::Result;
use async_trait::async_trait;
use seesaw_core::EventBus;
use seesaw_testing::SpyJobQueue;
use sqlx::PgPool;
use std::sync::{Arc, Mutex};

use super::{
    BaseAI, BaseEmbeddingService, BasePiiDetector, BasePushNotificationService, BaseSearchService,
    BaseWebScraper, CrawlResult, CrawledPage, LinkPriorities, PiiScrubResult, ScrapeResult,
    SearchResult, ServerKernel,
};
use crate::common::pii::{DetectionContext, PiiFindings, RedactionStrategy};

// =============================================================================
// Mock Web Scraper
// =============================================================================

/// Arguments captured from a crawl call
#[derive(Debug, Clone)]
pub struct CrawlCallArgs {
    pub url: String,
    pub max_depth: i32,
    pub max_pages: i32,
    pub delay_seconds: i32,
    pub priorities: Option<LinkPriorities>,
}

pub struct MockWebScraper {
    responses: Arc<Mutex<Vec<ScrapeResult>>>,
    crawl_responses: Arc<Mutex<Vec<CrawlResult>>>,
    scrape_calls: Arc<Mutex<Vec<String>>>,
    crawl_calls: Arc<Mutex<Vec<CrawlCallArgs>>>,
}

impl MockWebScraper {
    pub fn new() -> Self {
        Self {
            responses: Arc::new(Mutex::new(Vec::new())),
            crawl_responses: Arc::new(Mutex::new(Vec::new())),
            scrape_calls: Arc::new(Mutex::new(Vec::new())),
            crawl_calls: Arc::new(Mutex::new(Vec::new())),
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

    /// Add a crawl result to be returned
    pub fn with_crawl_result(self, result: CrawlResult) -> Self {
        self.crawl_responses.lock().unwrap().push(result);
        self
    }

    /// Add a crawl result from (url, markdown) pairs
    pub fn with_crawl_pages(self, pages: Vec<(&str, &str)>) -> Self {
        let crawled_pages: Vec<CrawledPage> = pages
            .into_iter()
            .map(|(url, markdown)| CrawledPage {
                url: url.to_string(),
                markdown: markdown.to_string(),
                title: Some(format!("Page: {}", url)),
            })
            .collect();
        self.crawl_responses
            .lock()
            .unwrap()
            .push(CrawlResult { pages: crawled_pages });
        self
    }

    /// Get all URLs that were scraped
    pub fn scrape_calls(&self) -> Vec<String> {
        self.scrape_calls.lock().unwrap().clone()
    }

    /// Get all crawl calls with their arguments
    pub fn crawl_calls(&self) -> Vec<CrawlCallArgs> {
        self.crawl_calls.lock().unwrap().clone()
    }

    /// Check if a URL was scraped
    pub fn was_scraped(&self, url: &str) -> bool {
        self.scrape_calls.lock().unwrap().iter().any(|u| u == url)
    }

    /// Check if a URL was crawled
    pub fn was_crawled(&self, url: &str) -> bool {
        self.crawl_calls
            .lock()
            .unwrap()
            .iter()
            .any(|c| c.url == url)
    }
}

#[async_trait]
impl BaseWebScraper for MockWebScraper {
    async fn scrape(&self, url: &str) -> Result<ScrapeResult> {
        // Record the call
        self.scrape_calls.lock().unwrap().push(url.to_string());

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

    async fn crawl(
        &self,
        url: &str,
        max_depth: i32,
        max_pages: i32,
        delay_seconds: i32,
        priorities: Option<&LinkPriorities>,
    ) -> Result<CrawlResult> {
        // Record the call with all arguments
        self.crawl_calls.lock().unwrap().push(CrawlCallArgs {
            url: url.to_string(),
            max_depth,
            max_pages,
            delay_seconds,
            priorities: priorities.cloned(),
        });

        // Check for queued crawl responses first
        let mut crawl_responses = self.crawl_responses.lock().unwrap();
        if !crawl_responses.is_empty() {
            return Ok(crawl_responses.remove(0));
        }
        drop(crawl_responses);

        // Fall back to scrape responses converted to crawl pages
        let mut responses = self.responses.lock().unwrap();
        let pages: Vec<CrawledPage> = if !responses.is_empty() {
            // Use queued responses as pages
            responses
                .drain(..)
                .take(max_pages as usize)
                .map(|r| CrawledPage {
                    url: r.url,
                    markdown: r.markdown,
                    title: r.title,
                })
                .collect()
        } else {
            // Return default mock pages
            vec![
                CrawledPage {
                    url: url.to_string(),
                    markdown: "# Homepage\n\nThis is the mock homepage.".to_string(),
                    title: Some("Homepage".to_string()),
                },
                CrawledPage {
                    url: format!("{}/about", url),
                    markdown: "# About\n\nThis is the mock about page.".to_string(),
                    title: Some("About".to_string()),
                },
            ]
        };

        Ok(CrawlResult { pages })
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
        self.calls
            .lock()
            .unwrap()
            .iter()
            .any(|p| p.contains(text))
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
// Mock Search Service
// =============================================================================

pub struct MockSearchService {
    responses: Arc<Mutex<Vec<Vec<SearchResult>>>>,
}

impl MockSearchService {
    pub fn new() -> Self {
        Self {
            responses: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn with_results(self, results: Vec<SearchResult>) -> Self {
        self.responses.lock().unwrap().push(results);
        self
    }
}

#[async_trait]
impl BaseSearchService for MockSearchService {
    async fn search(
        &self,
        _query: &str,
        _max_results: Option<usize>,
        _search_depth: Option<&str>,
        _days: Option<i32>,
    ) -> Result<Vec<SearchResult>> {
        let mut responses = self.responses.lock().unwrap();
        if !responses.is_empty() {
            Ok(responses.remove(0))
        } else {
            // Return empty results by default
            Ok(vec![])
        }
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
    pub web_scraper: Arc<MockWebScraper>,
    pub ai: Arc<MockAI>,
    pub embedding_service: Arc<MockEmbeddingService>,
    pub push_service: Arc<MockPushNotificationService>,
    pub search_service: Arc<MockSearchService>,
    pub pii_detector: Arc<MockPiiDetector>,
    pub job_queue: Arc<SpyJobQueue>,
}

impl TestDependencies {
    pub fn new() -> Self {
        Self {
            web_scraper: Arc::new(MockWebScraper::new()),
            ai: Arc::new(MockAI::new()),
            embedding_service: Arc::new(MockEmbeddingService::new()),
            push_service: Arc::new(MockPushNotificationService::new()),
            search_service: Arc::new(MockSearchService::new()),
            pii_detector: Arc::new(MockPiiDetector::new()),
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

    /// Set a mock search service
    pub fn mock_search(mut self, service: MockSearchService) -> Self {
        self.search_service = Arc::new(service);
        self
    }

    /// Set a mock PII detector
    pub fn mock_pii(mut self, detector: MockPiiDetector) -> Self {
        self.pii_detector = Arc::new(detector);
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
            self.search_service,
            self.pii_detector,
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
