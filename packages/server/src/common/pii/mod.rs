/// PII (Personally Identifiable Information) detection and redaction
///
/// This module provides automatic PII scrubbing for anonymous communication.
/// It detects and redacts emails, phone numbers, SSNs, credit cards, IPs, and more.
///
/// # Detection Methods
///
/// - **Regex-based**: Fast detection of structured PII (emails, phones, SSNs, credit cards, IPs)
/// - **LLM-based**: Context-aware detection of unstructured PII (names, addresses, medical info)
///
/// # Redaction Strategies
///
/// - `FullRemoval`: Replace PII with [REDACTED] token
/// - `PartialMask`: Partially mask while preserving readability (john@example.com → j***@example.com)
/// - `TokenReplacement`: Replace with typed tokens ([EMAIL], [PHONE], etc.)
///
/// # Examples
///
/// ```rust
/// use server_core::common::pii::{detect_structured_pii, redact_pii, RedactionStrategy};
///
/// let text = "Contact me at john@example.com or (555) 123-4567";
///
/// // Detect PII
/// let findings = detect_structured_pii(text);
///
/// // Redact with partial masking
/// let clean = redact_pii(text, &findings, RedactionStrategy::PartialMask);
/// // Result: "Contact me at j***@example.com or (555) 123-****"
/// ```
///
/// # Integration Points
///
/// This module is used in:
/// - Message creation (scrub user input)
/// - Web scraping (scrub scraped content)
/// - Error logging (scrub Sentry reports)

pub mod detector;
pub mod llm_detector;
pub mod redactor;

// Re-export main types and functions
pub use detector::{
    detect_pii_contextual, detect_structured_pii, DetectionContext, PiiFindings, PiiMatch, PiiType,
};
pub use llm_detector::{detect_pii_hybrid, detect_pii_with_gpt, entities_to_findings, PiiEntity};
pub use redactor::{redact_pii, RedactionStrategy};
