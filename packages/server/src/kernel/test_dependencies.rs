// TestDependencies - mock implementations for testing
//
// Provides mock services that can be injected into ServerDeps for tests.

use anyhow::Result;
use async_trait::async_trait;
use sqlx::PgPool;
use std::sync::Arc;

use super::{BasePiiDetector, PiiScrubResult};
use crate::common::pii::{DetectionContext, PiiFindings, RedactionStrategy};
use crate::domains::auth::JwtService;
use crate::kernel::{ServerDeps, StreamHub, TwilioAdapter};

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
    pub pii_detector: Arc<MockPiiDetector>,
}

impl TestDependencies {
    pub fn new() -> Self {
        Self {
            pii_detector: Arc::new(MockPiiDetector::new()),
        }
    }

    /// Set a mock PII detector
    pub fn mock_pii(mut self, detector: MockPiiDetector) -> Self {
        self.pii_detector = Arc::new(detector);
        self
    }

    /// Convert into ServerDeps for testing
    pub fn into_server_deps(self, db_pool: PgPool) -> ServerDeps {
        let twilio = Arc::new(twilio::TwilioService::new(twilio::TwilioOptions {
            account_sid: "test_account_sid".to_string(),
            auth_token: "test_auth_token".to_string(),
            service_id: "test_service_id".to_string(),
        }));
        let jwt_service = Arc::new(JwtService::new("test_secret", "test_issuer".to_string()));

        ServerDeps::new(
            db_pool,
            Arc::new(TwilioAdapter::new(twilio)),
            self.pii_detector,
            None, // storage — no S3 in tests
            jwt_service,
            StreamHub::new(),
            true,   // test_identifier_enabled
            vec![], // admin_identifiers
        )
    }
}

impl Default for TestDependencies {
    fn default() -> Self {
        Self::new()
    }
}
