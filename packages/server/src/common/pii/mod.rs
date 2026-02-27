/// PII (Personally Identifiable Information) detection and redaction
///
/// Regex-based detection of structured PII (emails, phones, SSNs, credit cards, IPs)
/// with context-aware filtering (preserves public organizational contact info).
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
pub mod detector;
pub mod redactor;

// Re-export main types and functions
pub use detector::{
    detect_pii_contextual, detect_structured_pii, DetectionContext, PiiFindings, PiiMatch, PiiType,
};
pub use redactor::{redact_pii, RedactionStrategy};
