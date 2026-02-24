//! Strategy orchestrator for query classification.
//!
//! Classifies queries into Collection/Singular/Narrative strategies
//! using heuristics first, with LLM fallback.

use crate::traits::ai::ExtractionStrategy;

/// Keywords that suggest a Collection query.
const COLLECTION_KEYWORDS: &[&str] = &[
    "find all",
    "list of",
    "list all",
    "show all",
    "all the",
    "every",
    "opportunities",
    "services",
    "programs",
    "events",
    "jobs",
    "positions",
    "listings",
    "items",
    "products",
    "articles",
    "posts",
];

/// Keywords that suggest a Singular query.
const SINGULAR_KEYWORDS: &[&str] = &[
    "what is the",
    "what's the",
    "where is the",
    "where's the",
    "when is the",
    "when's the",
    "who is the",
    "who's the",
    "phone",
    "email",
    "address",
    "contact",
    "location",
    "hours",
    "price",
    "cost",
    "date",
    "time",
    "deadline",
];

/// Keywords that suggest a Narrative query.
const NARRATIVE_KEYWORDS: &[&str] = &[
    "summarize",
    "describe",
    "explain",
    "tell me about",
    "what does",
    "who does",
    "how does",
    "overview",
    "about",
    "mission",
    "history",
    "background",
];

/// Classify a query using heuristics.
///
/// Returns `Some(strategy)` if heuristics are confident,
/// `None` if LLM classification is needed.
pub fn classify_by_heuristics(query: &str) -> Option<ExtractionStrategy> {
    let query_lower = query.to_lowercase();

    // Check Collection keywords
    let collection_score = COLLECTION_KEYWORDS
        .iter()
        .filter(|k| query_lower.contains(*k))
        .count();

    // Check Singular keywords
    let singular_score = SINGULAR_KEYWORDS
        .iter()
        .filter(|k| query_lower.contains(*k))
        .count();

    // Check Narrative keywords
    let narrative_score = NARRATIVE_KEYWORDS
        .iter()
        .filter(|k| query_lower.contains(*k))
        .count();

    // If one category is clearly dominant, return it
    let max_score = collection_score.max(singular_score).max(narrative_score);

    if max_score == 0 {
        return None; // No keywords matched, need LLM
    }

    // Need at least 2x the score of others to be confident
    if collection_score >= 1
        && collection_score > singular_score
        && collection_score > narrative_score
    {
        return Some(ExtractionStrategy::Collection);
    }

    if singular_score >= 1 && singular_score > collection_score && singular_score > narrative_score
    {
        return Some(ExtractionStrategy::Singular);
    }

    if narrative_score >= 1
        && narrative_score > collection_score
        && narrative_score > singular_score
    {
        return Some(ExtractionStrategy::Narrative);
    }

    // Ambiguous - need LLM
    None
}

/// Analyze query structure for additional classification hints.
pub struct QueryAnalysis {
    /// Whether the query is a question
    pub is_question: bool,

    /// Whether the query uses plural nouns
    pub uses_plural: bool,

    /// Whether the query mentions specific fields
    pub mentions_specific_field: bool,

    /// Word count
    pub word_count: usize,
}

impl QueryAnalysis {
    /// Analyze a query.
    pub fn analyze(query: &str) -> Self {
        let query_lower = query.to_lowercase();
        let words: Vec<&str> = query.split_whitespace().collect();

        Self {
            is_question: query.ends_with('?')
                || query_lower.starts_with("what")
                || query_lower.starts_with("where")
                || query_lower.starts_with("when")
                || query_lower.starts_with("who")
                || query_lower.starts_with("how"),
            uses_plural: words.iter().any(|w| {
                w.ends_with("ies") || (w.ends_with('s') && !w.ends_with("ss") && w.len() > 3)
            }),
            mentions_specific_field: SINGULAR_KEYWORDS.iter().any(|k| query_lower.contains(k)),
            word_count: words.len(),
        }
    }

    /// Get suggested strategy based on analysis.
    pub fn suggested_strategy(&self) -> Option<ExtractionStrategy> {
        // Short queries mentioning specific fields -> Singular
        if self.mentions_specific_field && self.word_count <= 5 {
            return Some(ExtractionStrategy::Singular);
        }

        // Questions with plural nouns -> Collection
        if self.is_question && self.uses_plural {
            return Some(ExtractionStrategy::Collection);
        }

        // Questions about "what does X do" -> Narrative
        if self.is_question && !self.uses_plural && !self.mentions_specific_field {
            return Some(ExtractionStrategy::Narrative);
        }

        None
    }
}

/// Combined classification using heuristics and analysis.
pub fn classify_query(query: &str) -> Option<ExtractionStrategy> {
    // Try keyword heuristics first
    if let Some(strategy) = classify_by_heuristics(query) {
        return Some(strategy);
    }

    // Try structural analysis
    let analysis = QueryAnalysis::analyze(query);
    analysis.suggested_strategy()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collection_queries() {
        let queries = [
            "find all volunteer opportunities",
            "list of services",
            "show all events",
            "what programs are available",
        ];

        for query in queries {
            let result = classify_by_heuristics(query);
            assert_eq!(
                result,
                Some(ExtractionStrategy::Collection),
                "Expected Collection for: {}",
                query
            );
        }
    }

    #[test]
    fn test_singular_queries() {
        let queries = [
            "what is the phone number",
            "where is the address",
            "email contact",
            "what are the hours",
        ];

        for query in queries {
            let result = classify_by_heuristics(query);
            assert_eq!(
                result,
                Some(ExtractionStrategy::Singular),
                "Expected Singular for: {}",
                query
            );
        }
    }

    #[test]
    fn test_narrative_queries() {
        let queries = [
            "summarize this organization",
            "describe their mission",
            "tell me about this nonprofit",
            "what does this company do",
        ];

        for query in queries {
            let result = classify_by_heuristics(query);
            assert_eq!(
                result,
                Some(ExtractionStrategy::Narrative),
                "Expected Narrative for: {}",
                query
            );
        }
    }

    #[test]
    fn test_ambiguous_queries() {
        let queries = ["stuff", "things here", "xyz"];

        for query in queries {
            let result = classify_by_heuristics(query);
            assert_eq!(result, None, "Expected None (ambiguous) for: {}", query);
        }
    }

    #[test]
    fn test_query_analysis() {
        let analysis = QueryAnalysis::analyze("what volunteer opportunities are available?");
        assert!(analysis.is_question);
        assert!(analysis.uses_plural);
        assert!(!analysis.mentions_specific_field);
    }

    #[test]
    fn test_combined_classification() {
        // Should use heuristics
        assert_eq!(
            classify_query("list of services"),
            Some(ExtractionStrategy::Collection)
        );

        // Should use analysis when heuristics fail
        assert_eq!(
            classify_query("what are the jobs?"),
            Some(ExtractionStrategy::Collection) // "jobs" is a collection keyword
        );
    }
}
