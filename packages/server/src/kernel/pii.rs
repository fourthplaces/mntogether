// PII Detection Service Implementations

use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

use crate::common::pii::llm_detector::detect_pii_hybrid_with_ai;
use crate::common::pii::{
    detect_pii_contextual, redact_pii, DetectionContext, PiiFindings, RedactionStrategy,
};
use crate::kernel::traits::{BasePiiDetector, PiiScrubResult};

// =============================================================================
// Regex-only PII Detector (Fast, No AI)
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
// Hybrid PII Detector (Regex + GPT)
// =============================================================================

/// Hybrid PII detector using regex + LLM
/// Detects structured PII (regex) + unstructured PII (names, addresses via LLM)
pub struct HybridPiiDetector {
    ai: Arc<ai_client::OpenAi>,
}

impl HybridPiiDetector {
    pub fn new(ai: Arc<ai_client::OpenAi>) -> Self {
        Self { ai }
    }
}

#[async_trait]
impl BasePiiDetector for HybridPiiDetector {
    async fn detect(&self, text: &str, context: DetectionContext) -> Result<PiiFindings> {
        // IMPORTANT: Redact PII BEFORE sending to LLM
        // Step 1: Run regex detection first (fast, catches obvious PII)
        let regex_findings = detect_pii_contextual(text, context);

        // Step 2: Redact detected PII before sending to LLM
        let redacted_text = redact_pii(text, &regex_findings, RedactionStrategy::TokenReplacement);

        // Step 3: Send redacted text to LLM for additional contextual detection
        // The LLM will see "[EMAIL]" instead of actual emails
        // For now, only use GPT for personal messages to reduce costs
        if context == DetectionContext::PersonalMessage && !text.is_empty() {
            // Use hybrid detection on the REDACTED text
            match detect_pii_hybrid_with_ai(&redacted_text, &self.ai).await {
                Ok(llm_findings) => {
                    // Combine regex and LLM findings
                    let mut combined = regex_findings;
                    for llm_match in llm_findings.matches {
                        // Check if this overlaps with existing matches
                        let overlaps = combined.matches.iter().any(|existing| {
                            (llm_match.start >= existing.start && llm_match.start < existing.end)
                                || (llm_match.end > existing.start && llm_match.end <= existing.end)
                                || (llm_match.start <= existing.start
                                    && llm_match.end >= existing.end)
                        });

                        if !overlaps {
                            combined.matches.push(llm_match);
                        }
                    }
                    Ok(combined)
                }
                Err(e) => {
                    tracing::warn!(error = %e, "LLM PII detection failed, falling back to regex only");
                    Ok(regex_findings)
                }
            }
        } else {
            // Public content: use regex only
            Ok(regex_findings)
        }
    }

    async fn scrub(
        &self,
        text: &str,
        context: DetectionContext,
        strategy: RedactionStrategy,
    ) -> Result<PiiScrubResult> {
        let findings = self.detect(text, context).await?;
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
pub fn create_pii_detector(
    enabled: bool,
    use_llm: bool,
    ai: Option<Arc<ai_client::OpenAi>>,
) -> Arc<dyn BasePiiDetector> {
    if !enabled {
        tracing::info!("PII scrubbing disabled");
        return Arc::new(NoopPiiDetector::new());
    }

    if use_llm {
        match ai {
            Some(ai_client) => {
                tracing::info!("PII scrubbing enabled with hybrid detection (regex + LLM)");
                Arc::new(HybridPiiDetector::new(ai_client))
            }
            None => {
                tracing::warn!(
                    "PII_USE_GPT_DETECTION=true but no AI client provided, falling back to regex-only"
                );
                Arc::new(RegexPiiDetector::new())
            }
        }
    } else {
        tracing::info!("PII scrubbing enabled with regex-only detection");
        Arc::new(RegexPiiDetector::new())
    }
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
        let detector = create_pii_detector(false, false, None);

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
    async fn test_factory_regex_only() {
        let detector = create_pii_detector(true, false, None);

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
