//! Grounding calculation and verification.
//!
//! Calculates GroundingGrade based on source analysis and conflict detection.

use crate::types::extraction::{Conflict, GroundingGrade, Source};

/// Internal claim representation for grounding analysis.
#[derive(Debug, Clone)]
pub struct Claim {
    /// The statement being made
    pub statement: String,

    /// Evidence supporting the claim
    pub evidence: Vec<Evidence>,

    /// Grounding type
    pub grounding: ClaimGrounding,
}

/// Evidence supporting a claim.
#[derive(Debug, Clone)]
pub struct Evidence {
    /// Exact quote from source
    pub quote: String,

    /// URL of the source
    pub source_url: String,
}

/// How well-grounded is a claim?
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClaimGrounding {
    /// Exact quote supports the claim
    Direct,

    /// Reasonable inference from source
    Inferred,

    /// No direct evidence (often hallucination)
    Assumed,
}

/// Configuration for grounding calculation.
#[derive(Debug, Clone)]
pub struct GroundingConfig {
    /// Discard claims with "Assumed" grounding
    pub strict_mode: bool,

    /// Minimum number of sources for "Verified" grade
    pub verified_threshold: usize,
}

impl Default for GroundingConfig {
    fn default() -> Self {
        Self {
            strict_mode: true,
            verified_threshold: 2,
        }
    }
}

/// Calculate grounding grade from claims and sources.
pub fn calculate_grounding(
    sources: &[Source],
    conflicts: &[Conflict],
    claims: &[Claim],
    config: &GroundingConfig,
) -> GroundingGrade {
    // Conflicts take precedence
    if !conflicts.is_empty() {
        return GroundingGrade::Conflicted;
    }

    // Check for "Assumed" claims (potential hallucinations)
    let has_assumed = claims
        .iter()
        .any(|c| c.grounding == ClaimGrounding::Assumed);

    if has_assumed && !config.strict_mode {
        // In non-strict mode, assumed claims result in Inferred grade
        return GroundingGrade::Inferred;
    }

    // Check for inferred claims
    let has_inferred = claims
        .iter()
        .any(|c| c.grounding == ClaimGrounding::Inferred);

    if has_inferred {
        return GroundingGrade::Inferred;
    }

    // Check source count for verification
    if sources.len() >= config.verified_threshold {
        return GroundingGrade::Verified;
    }

    GroundingGrade::SingleSource
}

/// Filter claims based on grounding config.
pub fn filter_claims(claims: Vec<Claim>, config: &GroundingConfig) -> Vec<Claim> {
    if config.strict_mode {
        claims
            .into_iter()
            .filter(|c| c.grounding != ClaimGrounding::Assumed)
            .collect()
    } else {
        claims
    }
}

/// Detect conflicts by grouping claims by topic.
pub fn detect_conflicts(claims: &[Claim]) -> Vec<Conflict> {
    use std::collections::HashMap;

    // Group claims by normalized topic (simplified approach)
    let mut by_topic: HashMap<String, Vec<&Claim>> = HashMap::new();

    for claim in claims {
        // Use first few words as topic key (simplified)
        let topic_key = claim
            .statement
            .split_whitespace()
            .take(3)
            .collect::<Vec<_>>()
            .join(" ")
            .to_lowercase();

        by_topic.entry(topic_key).or_default().push(claim);
    }

    // Find topics with conflicting claims
    let mut conflicts = Vec::new();

    for (topic, topic_claims) in by_topic {
        if topic_claims.len() < 2 {
            continue;
        }

        // Check if claims from different sources have different content
        let sources_with_content: Vec<_> = topic_claims
            .iter()
            .filter_map(|c| {
                c.evidence.first().map(|e| (&c.statement, &e.source_url))
            })
            .collect();

        // Group by source
        let unique_statements: std::collections::HashSet<_> =
            sources_with_content.iter().map(|(s, _)| *s).collect();

        if unique_statements.len() > 1 {
            // We have conflicting statements
            let conflict = Conflict {
                topic: topic.clone(),
                claims: sources_with_content
                    .iter()
                    .map(|(statement, url)| crate::types::extraction::ConflictingClaim {
                        statement: (*statement).clone(),
                        source_url: (*url).clone(),
                    })
                    .collect(),
            };
            conflicts.push(conflict);
        }
    }

    conflicts
}

/// Aggregate evidence from multiple claims.
pub fn aggregate_sources(claims: &[Claim]) -> Vec<Source> {
    use chrono::Utc;
    use std::collections::HashMap;

    let mut source_counts: HashMap<String, usize> = HashMap::new();

    for claim in claims {
        for evidence in &claim.evidence {
            *source_counts.entry(evidence.source_url.clone()).or_insert(0) += 1;
        }
    }

    // Sort by count (most evidence = Primary)
    let mut sources: Vec<_> = source_counts.into_iter().collect();
    sources.sort_by(|a, b| b.1.cmp(&a.1));

    sources
        .into_iter()
        .enumerate()
        .map(|(i, (url, _))| Source {
            url,
            title: None,
            fetched_at: Utc::now(),
            role: if i == 0 {
                crate::types::extraction::SourceRole::Primary
            } else if i == 1 {
                crate::types::extraction::SourceRole::Corroborating
            } else {
                crate::types::extraction::SourceRole::Supporting
            },
            metadata: HashMap::new(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn mock_source(url: &str) -> Source {
        Source {
            url: url.to_string(),
            title: None,
            fetched_at: Utc::now(),
            role: crate::types::extraction::SourceRole::Primary,
            metadata: std::collections::HashMap::new(),
        }
    }

    fn mock_claim(statement: &str, grounding: ClaimGrounding, source_url: &str) -> Claim {
        Claim {
            statement: statement.to_string(),
            evidence: vec![Evidence {
                quote: statement.to_string(),
                source_url: source_url.to_string(),
            }],
            grounding,
        }
    }

    #[test]
    fn test_grounding_verified() {
        let sources = vec![mock_source("url1"), mock_source("url2")];
        let claims = vec![mock_claim("Test", ClaimGrounding::Direct, "url1")];
        let config = GroundingConfig::default();

        let grade = calculate_grounding(&sources, &[], &claims, &config);
        assert_eq!(grade, GroundingGrade::Verified);
    }

    #[test]
    fn test_grounding_single_source() {
        let sources = vec![mock_source("url1")];
        let claims = vec![mock_claim("Test", ClaimGrounding::Direct, "url1")];
        let config = GroundingConfig::default();

        let grade = calculate_grounding(&sources, &[], &claims, &config);
        assert_eq!(grade, GroundingGrade::SingleSource);
    }

    #[test]
    fn test_grounding_conflicted() {
        let sources = vec![mock_source("url1"), mock_source("url2")];
        let conflicts = vec![Conflict {
            topic: "Schedule".to_string(),
            claims: vec![],
        }];
        let claims = vec![];
        let config = GroundingConfig::default();

        let grade = calculate_grounding(&sources, &conflicts, &claims, &config);
        assert_eq!(grade, GroundingGrade::Conflicted);
    }

    #[test]
    fn test_grounding_inferred() {
        let sources = vec![mock_source("url1"), mock_source("url2")];
        let claims = vec![mock_claim("Test", ClaimGrounding::Inferred, "url1")];
        let config = GroundingConfig::default();

        let grade = calculate_grounding(&sources, &[], &claims, &config);
        assert_eq!(grade, GroundingGrade::Inferred);
    }

    #[test]
    fn test_filter_claims_strict_mode() {
        let claims = vec![
            mock_claim("Direct", ClaimGrounding::Direct, "url1"),
            mock_claim("Assumed", ClaimGrounding::Assumed, "url2"),
            mock_claim("Inferred", ClaimGrounding::Inferred, "url3"),
        ];

        let config = GroundingConfig {
            strict_mode: true,
            ..Default::default()
        };

        let filtered = filter_claims(claims, &config);
        assert_eq!(filtered.len(), 2);
        assert!(filtered.iter().all(|c| c.grounding != ClaimGrounding::Assumed));
    }

    #[test]
    fn test_aggregate_sources() {
        let claims = vec![
            mock_claim("A", ClaimGrounding::Direct, "url1"),
            mock_claim("B", ClaimGrounding::Direct, "url1"),
            mock_claim("C", ClaimGrounding::Direct, "url2"),
        ];

        let sources = aggregate_sources(&claims);

        assert_eq!(sources.len(), 2);
        // url1 has more evidence, should be Primary
        assert_eq!(sources[0].url, "url1");
        assert_eq!(
            sources[0].role,
            crate::types::extraction::SourceRole::Primary
        );
    }
}
