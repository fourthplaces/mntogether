//! Recall module for finding relevant pages.
//!
//! Implements hybrid recall combining semantic and keyword search.

use crate::traits::store::cosine_similarity;
use crate::types::summary::Summary;

/// Configuration for recall operations.
#[derive(Debug, Clone)]
pub struct RecallConfig {
    /// Maximum results to return
    pub limit: usize,

    /// Enable hybrid recall (semantic + keyword)
    pub hybrid: bool,

    /// Weight for semantic search (0.0 to 1.0)
    pub semantic_weight: f32,

    /// Boost weight for specific terms (proper nouns, IDs)
    pub specific_term_boost: f32,
}

impl Default for RecallConfig {
    fn default() -> Self {
        Self {
            limit: 50,
            hybrid: true,
            semantic_weight: 0.6,
            specific_term_boost: 1.5,
        }
    }
}

/// Check if a query contains specific terms that benefit from keyword search.
///
/// Specific terms include:
/// - Proper nouns (capitalized words)
/// - Numbers and IDs
/// - Quoted phrases
/// - Technical terms
pub fn has_specific_terms(query: &str) -> bool {
    let words: Vec<&str> = query.split_whitespace().collect();

    // Check for quoted phrases
    if query.contains('"') {
        return true;
    }

    // Check for numbers
    if words.iter().any(|w| w.chars().any(|c| c.is_numeric())) {
        return true;
    }

    // Check for proper nouns (capitalized words not at start)
    let has_proper_nouns = words
        .iter()
        .skip(1) // Skip first word
        .any(|w| {
            w.chars()
                .next()
                .map(|c| c.is_uppercase())
                .unwrap_or(false)
        });

    if has_proper_nouns {
        return true;
    }

    // Check for technical patterns (kebab-case, snake_case, camelCase)
    if words.iter().any(|w| w.contains('-') || w.contains('_')) {
        return true;
    }

    false
}

/// Calculate dynamic weights based on query characteristics.
pub fn calculate_weights(query: &str, config: &RecallConfig) -> (f32, f32) {
    if !config.hybrid {
        return (1.0, 0.0); // Semantic only
    }

    let base_semantic = config.semantic_weight;
    let base_keyword = 1.0 - config.semantic_weight;

    if has_specific_terms(query) {
        // Boost keyword weight for specific terms
        let boosted_keyword = (base_keyword * config.specific_term_boost).min(0.8);
        let adjusted_semantic = 1.0 - boosted_keyword;
        (adjusted_semantic, boosted_keyword)
    } else {
        (base_semantic, base_keyword)
    }
}

/// Rank summaries by embedding similarity to query.
pub fn rank_by_embedding<'a>(
    query_embedding: &[f32],
    summaries: &'a [Summary],
    limit: usize,
) -> Vec<(f32, &'a Summary)> {
    let mut scored: Vec<_> = summaries
        .iter()
        .filter_map(|s| {
            s.embedding.as_ref().map(|emb| {
                let score = cosine_similarity(query_embedding, emb);
                (score, s)
            })
        })
        .collect();

    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(limit);
    scored
}

/// Simple keyword matching for summaries.
///
/// Returns a score based on term frequency.
pub fn keyword_match(query: &str, text: &str) -> f32 {
    let query_lower = query.to_lowercase();
    let query_terms: Vec<&str> = query_lower
        .split_whitespace()
        .filter(|w| w.len() > 2) // Skip short words
        .collect();

    if query_terms.is_empty() {
        return 0.0;
    }

    let text_lower = text.to_lowercase();
    let matches = query_terms
        .iter()
        .filter(|term| text_lower.contains(*term))
        .count();

    matches as f32 / query_terms.len() as f32
}

/// Rank summaries by keyword matching.
pub fn rank_by_keyword<'a>(query: &str, summaries: &'a [Summary], limit: usize) -> Vec<(f32, &'a Summary)> {
    let mut scored: Vec<_> = summaries
        .iter()
        .map(|s| {
            // Match against summary text and signals
            let text_score = keyword_match(query, &s.text);
            let signal_text = s.embedding_text();
            let signal_score = keyword_match(query, &signal_text);

            // Combine scores
            let score = (text_score + signal_score) / 2.0;
            (score, s)
        })
        .filter(|(score, _)| *score > 0.0)
        .collect();

    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(limit);
    scored
}

/// Combine semantic and keyword results using Reciprocal Rank Fusion.
pub fn hybrid_rank<'a>(
    semantic_results: &[(f32, &'a Summary)],
    keyword_results: &[(f32, &'a Summary)],
    semantic_weight: f32,
    keyword_weight: f32,
    limit: usize,
) -> Vec<&'a Summary> {
    use std::collections::HashMap;

    const K: f32 = 60.0;
    let mut scores: HashMap<&str, (f32, &Summary)> = HashMap::new();

    // Score from semantic results
    for (rank, (_, summary)) in semantic_results.iter().enumerate() {
        let rrf_score = semantic_weight / (K + rank as f32 + 1.0);
        scores
            .entry(&summary.url)
            .and_modify(|(s, _)| *s += rrf_score)
            .or_insert((rrf_score, *summary));
    }

    // Score from keyword results
    for (rank, (_, summary)) in keyword_results.iter().enumerate() {
        let rrf_score = keyword_weight / (K + rank as f32 + 1.0);
        scores
            .entry(&summary.url)
            .and_modify(|(s, _)| *s += rrf_score)
            .or_insert((rrf_score, *summary));
    }

    // Sort and return
    let mut combined: Vec<_> = scores.into_values().collect();
    combined.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    combined
        .into_iter()
        .take(limit)
        .map(|(_, s)| s)
        .collect()
}

/// Perform hybrid recall on summaries.
pub fn hybrid_recall<'a>(
    query: &str,
    query_embedding: &[f32],
    summaries: &'a [Summary],
    config: &RecallConfig,
) -> Vec<&'a Summary> {
    let (semantic_weight, keyword_weight) = calculate_weights(query, config);

    let semantic_results = rank_by_embedding(query_embedding, summaries, config.limit * 2);
    let keyword_results = rank_by_keyword(query, summaries, config.limit * 2);

    hybrid_rank(
        &semantic_results,
        &keyword_results,
        semantic_weight,
        keyword_weight,
        config.limit,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::summary::RecallSignals;

    fn mock_summary(url: &str, text: &str, embedding: Option<Vec<f32>>) -> Summary {
        Summary {
            url: url.to_string(),
            site_url: "https://example.com".to_string(),
            text: text.to_string(),
            signals: RecallSignals::default(),
            language: Some("en".to_string()),
            created_at: chrono::Utc::now(),
            prompt_hash: "hash".to_string(),
            content_hash: "hash".to_string(),
            embedding,
        }
    }

    #[test]
    fn test_has_specific_terms() {
        assert!(has_specific_terms("Contact John Smith"));
        assert!(has_specific_terms("Event on 2024-01-15"));
        assert!(has_specific_terms("user-profile-page"));
        assert!(has_specific_terms("\"exact phrase\""));
        assert!(!has_specific_terms("find volunteer opportunities"));
    }

    #[test]
    fn test_calculate_weights() {
        let config = RecallConfig::default();

        // Normal query - default weights
        let (sem, kw) = calculate_weights("volunteer opportunities", &config);
        assert!((sem - 0.6).abs() < 0.01);
        assert!((kw - 0.4).abs() < 0.01);

        // Specific terms - boosted keyword weight
        let (_sem, kw) = calculate_weights("Contact John Smith", &config);
        assert!(kw > 0.4); // Keyword weight should be boosted
    }

    #[test]
    fn test_keyword_match() {
        let score = keyword_match("volunteer opportunities", "We offer volunteer opportunities for everyone");
        assert!(score > 0.5);

        let score = keyword_match("volunteer opportunities", "Donate today");
        assert!(score < 0.5);
    }

    #[test]
    fn test_rank_by_embedding() {
        let summaries = vec![
            mock_summary("url1", "Text 1", Some(vec![1.0, 0.0, 0.0])),
            mock_summary("url2", "Text 2", Some(vec![0.0, 1.0, 0.0])),
            mock_summary("url3", "Text 3", Some(vec![0.9, 0.1, 0.0])),
        ];

        let query_embedding = vec![1.0, 0.0, 0.0];
        let ranked = rank_by_embedding(&query_embedding, &summaries, 10);

        assert_eq!(ranked.len(), 3);
        assert_eq!(ranked[0].1.url, "url1"); // Most similar
    }

    #[test]
    fn test_hybrid_rank() {
        let s1 = mock_summary("url1", "Text", None);
        let s2 = mock_summary("url2", "Text", None);
        let s3 = mock_summary("url3", "Text", None);

        let semantic = vec![(0.9, &s1), (0.8, &s2)];
        let keyword = vec![(0.9, &s2), (0.8, &s3)];

        let combined = hybrid_rank(&semantic, &keyword, 0.5, 0.5, 10);

        // s2 should be first (appears in both)
        assert_eq!(combined[0].url, "url2");
        assert_eq!(combined.len(), 3);
    }
}
