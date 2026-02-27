// PII Detection Service Implementations

use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

use crate::common::pii::{
    detect_pii_contextual, redact_pii, DetectionContext, PiiFindings, RedactionStrategy,
};
use crate::kernel::traits::{BasePiiDetector, PiiScrubResult};

// =============================================================================
// Regex-only PII Detector
// =============================================================================

/// Fast regex-based PII detector
/// Detects structured PII: emails, phones, SSNs, credit cards, IPs
pub struct RegexPiiDetector;

impl RegexPiiDetector {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl BasePiiDetector for RegexPiiDetector {
    async fn detect(&self, text: &str, context: DetectionContext) -> Result<PiiFindings> {
        Ok(detect_pii_contextual(text, context))
    }

    async fn scrub(
        &self,
        text: &str,
        context: DetectionContext,
        strategy: RedactionStrategy,
    ) -> Result<PiiScrubResult> {
        let findings = detect_pii_contextual(text, context);
        let pii_detected = !findings.is_empty();
        let clean_text = redact_pii(text, &findings, strategy);

        Ok(PiiScrubResult {
            clean_text,
            findings,
            pii_detected,
        })
    }
}

// =============================================================================
// No-op Detector (for testing or when scrubbing is disabled)
// =============================================================================

/// No-op PII detector that never detects PII
/// Used when PII scrubbing is disabled via config
pub struct NoopPiiDetector;

impl NoopPiiDetector {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl BasePiiDetector for NoopPiiDetector {
    async fn detect(&self, _text: &str, _context: DetectionContext) -> Result<PiiFindings> {
        Ok(PiiFindings::new())
    }

    async fn scrub(
        &self,
        text: &str,
        _context: DetectionContext,
        _strategy: RedactionStrategy,
    ) -> Result<PiiScrubResult> {
        Ok(PiiScrubResult {
            clean_text: text.to_string(),
            findings: PiiFindings::new(),
            pii_detected: false,
        })
    }
}

// =============================================================================
// Factory function
// =============================================================================

/// Create PII detector based on configuration
pub fn create_pii_detector(enabled: bool) -> Arc<dyn BasePiiDetector> {
    if !enabled {
        tracing::info!("PII scrubbing disabled");
        return Arc::new(NoopPiiDetector::new());
    }

    tracing::info!("PII scrubbing enabled (regex detection)");
    Arc::new(RegexPiiDetector::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_regex_detector() {
        let detector = RegexPiiDetector::new();

        let result = detector
            .scrub(
                "Email me at john@example.com",
                DetectionContext::PersonalMessage,
                RedactionStrategy::PartialMask,
            )
            .await
            .unwrap();

        assert!(result.pii_detected);
        assert!(result.clean_text.contains("j***@example.com"));
    }

    #[tokio::test]
    async fn test_noop_detector() {
        let detector = NoopPiiDetector::new();

        let result = detector
            .scrub(
                "Email me at john@example.com",
                DetectionContext::PersonalMessage,
                RedactionStrategy::PartialMask,
            )
            .await
            .unwrap();

        assert!(!result.pii_detected);
        assert_eq!(result.clean_text, "Email me at john@example.com");
    }

    #[tokio::test]
    async fn test_factory_disabled() {
        let detector = create_pii_detector(false);

        let result = detector
            .scrub(
                "Email me at john@example.com",
                DetectionContext::PersonalMessage,
                RedactionStrategy::PartialMask,
            )
            .await
            .unwrap();

        assert!(!result.pii_detected);
    }

    #[tokio::test]
    async fn test_factory_enabled() {
        let detector = create_pii_detector(true);

        let result = detector
            .scrub(
                "Email me at john@example.com",
                DetectionContext::PersonalMessage,
                RedactionStrategy::PartialMask,
            )
            .await
            .unwrap();

        assert!(result.pii_detected);
    }
}
