/// Pure utility functions for relevance checking
///
/// These functions contain NO side effects - they implement the business logic
/// for determining if a candidate is relevant for a need based on similarity scores.

/// Relevance thresholds for similarity scores (0.0 to 1.0)
pub const SIMILARITY_THRESHOLD_LOW: f64 = 0.4;
pub const SIMILARITY_THRESHOLD_HIGH: f64 = 0.8;
pub const SIMILARITY_THRESHOLD_MEDIUM: f64 = 0.6;

/// Result of relevance check
#[derive(Debug, Clone, PartialEq)]
pub struct RelevanceResult {
    pub is_relevant: bool,
    pub explanation: String,
    pub confidence: RelevanceConfidence,
}

/// Confidence level of the relevance decision
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RelevanceConfidence {
    /// Low similarity - rejected without AI call
    Low,
    /// Medium similarity - would need AI confirmation (not implemented yet)
    Medium,
    /// High similarity - accepted without AI call
    High,
}

/// Check relevance based on similarity score
///
/// Algorithm:
/// - similarity < 0.4 -> Not relevant (low confidence)
/// - similarity >= 0.8 -> Relevant (high confidence)
/// - 0.4 <= similarity < 0.8 -> Use fallback threshold of 0.6 (medium confidence)
///
/// In production, the medium range would use an AI call to make the final decision.
/// For now, we use a simple threshold.
///
/// This is a pure function with no side effects.
///
/// # Examples
/// ```
/// let result = check_relevance_by_similarity(0.85);
/// assert!(result.is_relevant);
/// assert_eq!(result.confidence, RelevanceConfidence::High);
///
/// let result = check_relevance_by_similarity(0.3);
/// assert!(!result.is_relevant);
/// assert_eq!(result.confidence, RelevanceConfidence::Low);
/// ```
pub fn check_relevance_by_similarity(similarity: f64) -> RelevanceResult {
    // Quick reject: very low similarity
    if similarity < SIMILARITY_THRESHOLD_LOW {
        return RelevanceResult {
            is_relevant: false,
            explanation: "Low similarity score".to_string(),
            confidence: RelevanceConfidence::Low,
        };
    }

    // Quick accept: very high similarity
    if similarity >= SIMILARITY_THRESHOLD_HIGH {
        return RelevanceResult {
            is_relevant: true,
            explanation: format!(
                "Strong match based on your interests and skills ({}% similar)",
                (similarity * 100.0) as i32
            ),
            confidence: RelevanceConfidence::High,
        };
    }

    // Medium similarity: use fallback threshold
    // TODO: In production, this would make an AI call for better accuracy
    let is_relevant = similarity >= SIMILARITY_THRESHOLD_MEDIUM;
    let explanation = if is_relevant {
        format!(
            "Your profile matches this opportunity ({}% similar)",
            (similarity * 100.0) as i32
        )
    } else {
        "Not a strong match".to_string()
    };

    RelevanceResult {
        is_relevant,
        explanation,
        confidence: RelevanceConfidence::Medium,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_very_low_similarity_rejected() {
        let result = check_relevance_by_similarity(0.3);
        assert!(!result.is_relevant);
        assert_eq!(result.confidence, RelevanceConfidence::Low);
        assert_eq!(result.explanation, "Low similarity score");
    }

    #[test]
    fn test_low_similarity_threshold_rejected() {
        let result = check_relevance_by_similarity(0.39);
        assert!(!result.is_relevant);
        assert_eq!(result.confidence, RelevanceConfidence::Low);
    }

    #[test]
    fn test_at_low_threshold_medium_confidence() {
        let result = check_relevance_by_similarity(0.4);
        assert!(!result.is_relevant); // Below medium threshold
        assert_eq!(result.confidence, RelevanceConfidence::Medium);
    }

    #[test]
    fn test_medium_similarity_below_threshold() {
        let result = check_relevance_by_similarity(0.55);
        assert!(!result.is_relevant); // Below 0.6 medium threshold
        assert_eq!(result.confidence, RelevanceConfidence::Medium);
    }

    #[test]
    fn test_medium_similarity_at_threshold() {
        let result = check_relevance_by_similarity(0.6);
        assert!(result.is_relevant);
        assert_eq!(result.confidence, RelevanceConfidence::Medium);
        assert!(result.explanation.contains("60% similar"));
    }

    #[test]
    fn test_medium_similarity_above_threshold() {
        let result = check_relevance_by_similarity(0.7);
        assert!(result.is_relevant);
        assert_eq!(result.confidence, RelevanceConfidence::Medium);
        assert!(result.explanation.contains("70% similar"));
    }

    #[test]
    fn test_high_similarity_threshold() {
        let result = check_relevance_by_similarity(0.8);
        assert!(result.is_relevant);
        assert_eq!(result.confidence, RelevanceConfidence::High);
        assert!(result.explanation.contains("80% similar"));
        assert!(result.explanation.contains("Strong match"));
    }

    #[test]
    fn test_very_high_similarity() {
        let result = check_relevance_by_similarity(0.95);
        assert!(result.is_relevant);
        assert_eq!(result.confidence, RelevanceConfidence::High);
        assert!(result.explanation.contains("95% similar"));
    }

    #[test]
    fn test_perfect_similarity() {
        let result = check_relevance_by_similarity(1.0);
        assert!(result.is_relevant);
        assert_eq!(result.confidence, RelevanceConfidence::High);
        assert!(result.explanation.contains("100% similar"));
    }

    #[test]
    fn test_zero_similarity() {
        let result = check_relevance_by_similarity(0.0);
        assert!(!result.is_relevant);
        assert_eq!(result.confidence, RelevanceConfidence::Low);
    }

    #[test]
    fn test_explanation_format_medium() {
        let result = check_relevance_by_similarity(0.65);
        assert!(result.explanation.contains("profile matches"));
        assert!(result.explanation.contains("65% similar"));
    }

    #[test]
    fn test_explanation_format_high() {
        let result = check_relevance_by_similarity(0.85);
        assert!(result.explanation.contains("Strong match"));
        assert!(result.explanation.contains("interests and skills"));
        assert!(result.explanation.contains("85% similar"));
    }

    #[test]
    fn test_boundary_values() {
        // Test exact boundary values
        assert!(!check_relevance_by_similarity(0.399).is_relevant);
        assert!(!check_relevance_by_similarity(0.4).is_relevant);
        assert!(!check_relevance_by_similarity(0.599).is_relevant);
        assert!(check_relevance_by_similarity(0.6).is_relevant);
        assert!(check_relevance_by_similarity(0.799).is_relevant);
        assert!(check_relevance_by_similarity(0.8).is_relevant);
    }
}
