//! Production-grade signal types with confidence and metadata.
//!
//! These types are **domain-agnostic**. The `signal_type` is a user-defined
//! string that the caller specifies through their extraction prompts.
//!
//! # Example
//!
//! ```rust,ignore
//! // E-commerce domain
//! ExtractedSignal::new("product", "iPhone 15 Pro")
//!     .with_subtype("electronics");
//!
//! // Real estate domain
//! ExtractedSignal::new("listing", "3BR apartment downtown")
//!     .with_subtype("rental");
//!
//! // Job board domain
//! ExtractedSignal::new("requirement", "5+ years Python experience");
//!
//! // Nonprofit domain (legacy compatibility)
//! ExtractedSignal::new("cta", "Sign up to volunteer");
//! ```

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A single extracted signal with production-grade metadata.
///
/// This struct is **domain-agnostic**:
/// - `signal_type` is a user-defined string (e.g., "product", "listing", "cta")
/// - `subtype` provides optional categorization within the type
///
/// # Production Features
/// - **Evidence grounding** via `source_id` (every signal traceable to source)
/// - **Confidence scoring** for filtering low-quality signals
/// - **Context snippets** for explainability (show where signal was found)
/// - **Group IDs** for deduplication across pages
/// - **Tags** for flexible categorization
///
/// Maps 1:1 to the `extraction_signals` table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedSignal {
    /// User-defined signal type (e.g., "product", "listing", "cta", "entity")
    ///
    /// The library does not enforce types - the caller defines them
    /// through their extraction prompts.
    pub signal_type: String,

    /// The signal value (e.g., "Sign up for our newsletter")
    pub value: String,

    /// Optional subtype for further classification.
    ///
    /// Examples:
    /// - signal_type="entity", subtype="contact"
    /// - signal_type="product", subtype="electronics"
    /// - signal_type="listing", subtype="rental"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtype: Option<String>,

    /// The source this signal was extracted from.
    ///
    /// Critical for the "Evidence-grounded" principle - every signal
    /// must be traceable back to its source page/summary.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_id: Option<Uuid>,

    /// AI confidence in this extraction (0.0-1.0).
    ///
    /// Higher confidence signals are more reliable.
    /// Default: 1.0 (fully confident)
    #[serde(default = "default_confidence")]
    pub confidence: f32,

    /// Supporting context showing where the signal was found.
    ///
    /// Useful for explainability and verification.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_snippet: Option<String>,

    /// Group ID for deduplication across pages.
    ///
    /// Signals with the same group_id refer to the same real-world thing
    /// (e.g., the same CTA appearing on multiple pages).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group_id: Option<Uuid>,

    /// Flexible tags for categorization.
    ///
    /// Examples: ["urgent", "seasonal", "verified", "primary"]
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

fn default_confidence() -> f32 {
    1.0
}

impl ExtractedSignal {
    /// Create a new signal with user-defined type and value.
    ///
    /// # Example
    /// ```rust,ignore
    /// // Any domain - you define the types
    /// ExtractedSignal::new("product", "MacBook Pro M3");
    /// ExtractedSignal::new("job_requirement", "AWS certification");
    /// ExtractedSignal::new("amenity", "In-unit washer/dryer");
    /// ```
    pub fn new(signal_type: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            signal_type: signal_type.into(),
            value: value.into(),
            subtype: None,
            source_id: None,
            confidence: 1.0,
            context_snippet: None,
            group_id: None,
            tags: Vec::new(),
        }
    }

    /// Create a signal with type, value, and subtype.
    pub fn with_type_and_subtype(
        signal_type: impl Into<String>,
        value: impl Into<String>,
        subtype: impl Into<String>,
    ) -> Self {
        Self {
            subtype: Some(subtype.into()),
            ..Self::new(signal_type, value)
        }
    }

    // =========================================================================
    // Legacy convenience constructors (backwards compatibility)
    // These use the old domain-specific types as strings
    // =========================================================================

    /// Create a CTA signal (legacy compatibility).
    #[deprecated(note = "Use ExtractedSignal::new(\"cta\", value) for clarity")]
    pub fn cta(value: impl Into<String>) -> Self {
        Self::new("cta", value)
    }

    /// Create an Offer signal (legacy compatibility).
    #[deprecated(note = "Use ExtractedSignal::new(\"offer\", value) for clarity")]
    pub fn offer(value: impl Into<String>) -> Self {
        Self::new("offer", value)
    }

    /// Create an Ask signal (legacy compatibility).
    #[deprecated(note = "Use ExtractedSignal::new(\"ask\", value) for clarity")]
    pub fn ask(value: impl Into<String>) -> Self {
        Self::new("ask", value)
    }

    /// Create an Entity signal (legacy compatibility).
    #[deprecated(note = "Use ExtractedSignal::new(\"entity\", value) for clarity")]
    pub fn entity(value: impl Into<String>) -> Self {
        Self::new("entity", value)
    }

    /// Create an Entity signal with a specific subtype (legacy compatibility).
    #[deprecated(note = "Use ExtractedSignal::with_type_and_subtype(\"entity\", value, subtype)")]
    pub fn entity_with_type(value: impl Into<String>, entity_type: impl Into<String>) -> Self {
        Self::with_type_and_subtype("entity", value, entity_type)
    }

    // =========================================================================
    // Builder methods
    // =========================================================================

    /// Set the subtype.
    pub fn with_subtype(mut self, subtype: impl Into<String>) -> Self {
        self.subtype = Some(subtype.into());
        self
    }

    /// Set the source ID (for evidence grounding).
    ///
    /// Every signal should be traceable back to its source.
    pub fn with_source(mut self, source_id: Uuid) -> Self {
        self.source_id = Some(source_id);
        self
    }

    /// Set the confidence score.
    pub fn with_confidence(mut self, confidence: f32) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }

    /// Set the context snippet.
    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context_snippet = Some(context.into());
        self
    }

    /// Set the group ID.
    pub fn with_group(mut self, group_id: Uuid) -> Self {
        self.group_id = Some(group_id);
        self
    }

    /// Add a tag.
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Add multiple tags.
    pub fn with_tags(mut self, tags: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.tags.extend(tags.into_iter().map(Into::into));
        self
    }

    // =========================================================================
    // Query methods
    // =========================================================================

    /// Check if this signal has the given type.
    pub fn is_type(&self, signal_type: &str) -> bool {
        self.signal_type.eq_ignore_ascii_case(signal_type)
    }

    /// Check if this is a high-confidence signal.
    pub fn is_high_confidence(&self, threshold: f32) -> bool {
        self.confidence >= threshold
    }

    /// Check if this signal has a specific tag.
    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.iter().any(|t| t == tag)
    }
}

/// Collection of extracted signals with filtering capabilities.
///
/// Provides convenience methods for querying signals by type,
/// confidence, and tags. Works with any user-defined signal types.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StructuredSignals {
    /// All extracted signals with metadata
    #[serde(default)]
    pub signals: Vec<ExtractedSignal>,
}

impl StructuredSignals {
    /// Create an empty signal collection.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a signal.
    pub fn add(&mut self, signal: ExtractedSignal) {
        self.signals.push(signal);
    }

    /// Add a signal (builder pattern).
    pub fn with_signal(mut self, signal: ExtractedSignal) -> Self {
        self.signals.push(signal);
        self
    }

    /// Get all signals of a specific type (case-insensitive).
    pub fn by_type<'a>(
        &'a self,
        signal_type: &'a str,
    ) -> impl Iterator<Item = &'a ExtractedSignal> {
        self.signals
            .iter()
            .filter(move |s| s.signal_type.eq_ignore_ascii_case(signal_type))
    }

    /// Get all unique signal types in this collection.
    pub fn signal_types(&self) -> Vec<&str> {
        let mut types: Vec<&str> = self
            .signals
            .iter()
            .map(|s| s.signal_type.as_str())
            .collect();
        types.sort();
        types.dedup();
        types
    }

    // =========================================================================
    // Legacy convenience methods (backwards compatibility)
    // =========================================================================

    /// Get all CTAs (legacy compatibility).
    #[deprecated(note = "Use by_type(\"cta\") for clarity")]
    pub fn calls_to_action(&self) -> impl Iterator<Item = &ExtractedSignal> {
        self.by_type("cta")
    }

    /// Get all Offers (legacy compatibility).
    #[deprecated(note = "Use by_type(\"offer\") for clarity")]
    pub fn offers(&self) -> impl Iterator<Item = &ExtractedSignal> {
        self.by_type("offer")
    }

    /// Get all Asks (legacy compatibility).
    #[deprecated(note = "Use by_type(\"ask\") for clarity")]
    pub fn asks(&self) -> impl Iterator<Item = &ExtractedSignal> {
        self.by_type("ask")
    }

    /// Get all Entities (legacy compatibility).
    #[deprecated(note = "Use by_type(\"entity\") for clarity")]
    pub fn entities(&self) -> impl Iterator<Item = &ExtractedSignal> {
        self.by_type("entity")
    }

    // =========================================================================
    // Generic query methods
    // =========================================================================

    /// Get signals above a confidence threshold.
    pub fn high_confidence(&self, threshold: f32) -> impl Iterator<Item = &ExtractedSignal> {
        self.signals
            .iter()
            .filter(move |s| s.confidence >= threshold)
    }

    /// Get signals with a specific tag.
    pub fn with_tag<'a>(&'a self, tag: &'a str) -> impl Iterator<Item = &'a ExtractedSignal> {
        self.signals.iter().filter(move |s| s.has_tag(tag))
    }

    /// Get signals in a specific group.
    pub fn in_group(&self, group_id: Uuid) -> impl Iterator<Item = &ExtractedSignal> {
        self.signals
            .iter()
            .filter(move |s| s.group_id == Some(group_id))
    }

    /// Get signals from a specific source (evidence tracing).
    pub fn from_source(&self, source_id: Uuid) -> impl Iterator<Item = &ExtractedSignal> {
        self.signals
            .iter()
            .filter(move |s| s.source_id == Some(source_id))
    }

    /// Check if there are any signals.
    pub fn is_empty(&self) -> bool {
        self.signals.is_empty()
    }

    /// Get total count of all signals.
    pub fn count(&self) -> usize {
        self.signals.len()
    }

    /// Get count by signal type (case-insensitive).
    pub fn count_by_type(&self, signal_type: &str) -> usize {
        self.by_type(signal_type).count()
    }
}

/// Convert from legacy RecallSignals to StructuredSignals.
#[allow(deprecated)]
impl From<&super::summary::RecallSignals> for StructuredSignals {
    fn from(legacy: &super::summary::RecallSignals) -> Self {
        let mut signals = StructuredSignals::new();

        for cta in &legacy.calls_to_action {
            signals.add(ExtractedSignal::new("cta", cta));
        }

        for offer in &legacy.offers {
            signals.add(ExtractedSignal::new("offer", offer));
        }

        for ask in &legacy.asks {
            signals.add(ExtractedSignal::new("ask", ask));
        }

        for entity in &legacy.entities {
            signals.add(ExtractedSignal::new("entity", entity));
        }

        signals
    }
}

/// Convert from StructuredSignals to legacy RecallSignals.
impl From<&StructuredSignals> for super::summary::RecallSignals {
    fn from(structured: &StructuredSignals) -> Self {
        let mut legacy = super::summary::RecallSignals::new();

        for signal in &structured.signals {
            match signal.signal_type.to_lowercase().as_str() {
                "cta" => legacy.calls_to_action.push(signal.value.clone()),
                "offer" => legacy.offers.push(signal.value.clone()),
                "ask" => legacy.asks.push(signal.value.clone()),
                "entity" => legacy.entities.push(signal.value.clone()),
                // Unknown types get added to entities as a fallback
                _ => legacy.entities.push(signal.value.clone()),
            }
        }

        legacy
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generic_signal_creation() {
        // E-commerce domain
        let product = ExtractedSignal::new("product", "iPhone 15 Pro")
            .with_subtype("electronics")
            .with_confidence(0.95)
            .with_tag("featured");

        assert_eq!(product.signal_type, "product");
        assert_eq!(product.value, "iPhone 15 Pro");
        assert_eq!(product.subtype, Some("electronics".to_string()));
        assert!(product.is_type("product"));
        assert!(product.is_type("PRODUCT")); // case-insensitive
        assert!(product.has_tag("featured"));

        // Real estate domain
        let listing =
            ExtractedSignal::new("listing", "3BR apartment downtown").with_subtype("rental");

        assert_eq!(listing.signal_type, "listing");
        assert_eq!(listing.subtype, Some("rental".to_string()));

        // Job board domain
        let requirement = ExtractedSignal::new("requirement", "5+ years Python");

        assert!(requirement.is_type("requirement"));
    }

    #[test]
    #[allow(deprecated)]
    fn test_legacy_signal_creation() {
        let signal = ExtractedSignal::cta("Sign up now")
            .with_confidence(0.95)
            .with_context("Found in hero section")
            .with_tag("primary");

        assert_eq!(signal.signal_type, "cta");
        assert_eq!(signal.value, "Sign up now");
        assert_eq!(signal.confidence, 0.95);
        assert_eq!(
            signal.context_snippet,
            Some("Found in hero section".to_string())
        );
        assert!(signal.has_tag("primary"));
    }

    #[test]
    #[allow(deprecated)]
    fn test_entity_with_type() {
        let signal = ExtractedSignal::entity_with_type("john@example.com", "contact");

        assert_eq!(signal.signal_type, "entity");
        assert_eq!(signal.subtype, Some("contact".to_string()));
    }

    #[test]
    fn test_confidence_clamping() {
        let high = ExtractedSignal::new("test", "value").with_confidence(1.5);
        assert_eq!(high.confidence, 1.0);

        let low = ExtractedSignal::new("test", "value").with_confidence(-0.5);
        assert_eq!(low.confidence, 0.0);
    }

    #[test]
    fn test_structured_signals_filtering() {
        let signals = StructuredSignals::new()
            .with_signal(ExtractedSignal::new("product", "iPhone").with_confidence(0.9))
            .with_signal(ExtractedSignal::new("product", "MacBook").with_confidence(0.5))
            .with_signal(ExtractedSignal::new("price", "$999"))
            .with_signal(ExtractedSignal::new("review", "Great product!").with_tag("verified"));

        assert_eq!(signals.count(), 4);
        assert_eq!(signals.count_by_type("product"), 2);
        assert_eq!(signals.count_by_type("PRODUCT"), 2); // case-insensitive
                                                         // High confidence (>=0.8): "iPhone" (0.9), "$999" (1.0), "Great product!" (1.0)
        assert_eq!(signals.high_confidence(0.8).count(), 3);
        assert_eq!(signals.with_tag("verified").count(), 1);
    }

    #[test]
    fn test_signal_types_discovery() {
        let signals = StructuredSignals::new()
            .with_signal(ExtractedSignal::new("product", "A"))
            .with_signal(ExtractedSignal::new("price", "B"))
            .with_signal(ExtractedSignal::new("product", "C"))
            .with_signal(ExtractedSignal::new("review", "D"));

        let types = signals.signal_types();
        assert_eq!(types, vec!["price", "product", "review"]);
    }

    #[test]
    #[allow(deprecated)]
    fn test_legacy_conversion() {
        let legacy = super::super::summary::RecallSignals::new()
            .with_cta("Sign up")
            .with_offer("Free trial")
            .with_entity("Acme Corp");

        let structured = StructuredSignals::from(&legacy);

        assert_eq!(structured.calls_to_action().count(), 1);
        assert_eq!(structured.offers().count(), 1);
        assert_eq!(structured.entities().count(), 1);

        // Round-trip
        let back: super::super::summary::RecallSignals = (&structured).into();
        assert_eq!(back.calls_to_action.len(), 1);
    }

    #[test]
    fn test_unknown_types_to_legacy() {
        let signals = StructuredSignals::new()
            .with_signal(ExtractedSignal::new("product", "iPhone"))
            .with_signal(ExtractedSignal::new("cta", "Buy now"));

        let legacy: super::super::summary::RecallSignals = (&signals).into();

        // "product" is unknown, goes to entities as fallback
        assert_eq!(legacy.entities.len(), 1);
        assert_eq!(legacy.entities[0], "iPhone");

        // "cta" is recognized
        assert_eq!(legacy.calls_to_action.len(), 1);
        assert_eq!(legacy.calls_to_action[0], "Buy now");
    }

    #[test]
    fn test_evidence_grounding_source_id() {
        let source1 = Uuid::new_v4();
        let source2 = Uuid::new_v4();

        let signals = StructuredSignals::new()
            .with_signal(ExtractedSignal::new("product", "iPhone").with_source(source1))
            .with_signal(ExtractedSignal::new("price", "$999").with_source(source1))
            .with_signal(ExtractedSignal::new("review", "Great!").with_source(source2));

        // Filter by source
        assert_eq!(signals.from_source(source1).count(), 2);
        assert_eq!(signals.from_source(source2).count(), 1);

        // Verify source_id is set correctly
        let product = signals.by_type("product").next().unwrap();
        assert_eq!(product.source_id, Some(source1));
    }
}
