//! Extraction module - parse AI responses and build extractions.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::types::extraction::{
    Conflict, ConflictingClaim, Extraction, GapQuery, GroundingGrade, Source, SourceRole,
};

/// Internal extraction response from AI (before transformation).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIExtractionResponse {
    /// Extracted content as markdown
    pub content: String,

    /// Claims with evidence
    #[serde(default)]
    pub claims: Vec<AIClaim>,

    /// Sources used
    #[serde(default)]
    pub sources: Vec<AISource>,

    /// Gaps identified
    #[serde(default)]
    pub gaps: Vec<AIGap>,

    /// Conflicts detected
    #[serde(default)]
    pub conflicts: Vec<AIConflict>,
}

/// A claim from the AI response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIClaim {
    pub statement: String,
    #[serde(default)]
    pub evidence: Vec<AIEvidence>,
    pub grounding: String, // "DIRECT", "INFERRED", "ASSUMED"
}

/// Evidence for a claim.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIEvidence {
    pub quote: String,
    pub source_url: String,
}

/// A source from the AI response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AISource {
    pub url: String,
    #[serde(default)]
    pub role: Option<String>, // "PRIMARY", "SUPPORTING", "CORROBORATING"
    #[serde(default)]
    pub title: Option<String>,
}

/// A gap from the AI response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIGap {
    pub field: String,
    pub query: String,
}

/// A conflict from the AI response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIConflict {
    pub topic: String,
    #[serde(default)]
    pub claims: Vec<AIConflictClaim>,
}

/// A conflicting claim.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIConflictClaim {
    pub statement: String,
    pub source_url: String,
}

/// Configuration for extraction transformation.
#[derive(Debug, Clone)]
pub struct ExtractionTransformConfig {
    /// Discard "ASSUMED" claims (potential hallucinations)
    pub strict_mode: bool,

    /// Minimum sources for "Verified" grounding
    pub verified_threshold: usize,
}

impl Default for ExtractionTransformConfig {
    fn default() -> Self {
        Self {
            strict_mode: true,
            verified_threshold: 2,
        }
    }
}

/// Transform AI response into a clean Extraction.
pub fn transform_extraction(
    response: AIExtractionResponse,
    config: &ExtractionTransformConfig,
) -> Extraction {
    // Filter claims if strict mode
    let filtered_claims: Vec<_> = if config.strict_mode {
        response
            .claims
            .into_iter()
            .filter(|c| c.grounding.to_uppercase() != "ASSUMED")
            .collect()
    } else {
        response.claims
    };

    // Check for inferred claims
    let has_inference = filtered_claims
        .iter()
        .any(|c| c.grounding.to_uppercase() == "INFERRED");

    // Transform sources
    let sources: Vec<Source> = response
        .sources
        .into_iter()
        .map(|s| {
            let role = match s.role.as_deref() {
                Some("PRIMARY") => SourceRole::Primary,
                Some("CORROBORATING") => SourceRole::Corroborating,
                _ => SourceRole::Supporting,
            };

            Source {
                url: s.url,
                title: s.title,
                fetched_at: chrono::Utc::now(),
                role,
                metadata: HashMap::new(),
            }
        })
        .collect();

    // Transform gaps
    let gaps: Vec<GapQuery> = response
        .gaps
        .into_iter()
        .map(|g| GapQuery::new(g.field, g.query))
        .collect();

    // Transform conflicts
    let conflicts: Vec<Conflict> = response
        .conflicts
        .into_iter()
        .map(|c| Conflict {
            topic: c.topic,
            claims: c
                .claims
                .into_iter()
                .map(|claim| ConflictingClaim {
                    statement: claim.statement,
                    source_url: claim.source_url,
                })
                .collect(),
        })
        .collect();

    // Calculate grounding
    let grounding = calculate_grounding(&sources, &conflicts, has_inference, config);

    Extraction {
        content: response.content,
        sources,
        gaps,
        grounding,
        conflicts,
    }
}

/// Calculate grounding grade.
fn calculate_grounding(
    sources: &[Source],
    conflicts: &[Conflict],
    has_inference: bool,
    config: &ExtractionTransformConfig,
) -> GroundingGrade {
    if !conflicts.is_empty() {
        return GroundingGrade::Conflicted;
    }
    if has_inference {
        return GroundingGrade::Inferred;
    }
    if sources.len() >= config.verified_threshold {
        return GroundingGrade::Verified;
    }
    GroundingGrade::SingleSource
}

/// Parse AI JSON response into structured extraction response.
pub fn parse_extraction_response(json: &str) -> Result<AIExtractionResponse, serde_json::Error> {
    serde_json::from_str(json)
}

/// Parse a single-answer response (for Singular strategy).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AISingleResponse {
    pub content: String,
    pub found: bool,
    #[serde(default)]
    pub source: Option<AISingleSource>,
    #[serde(default)]
    pub conflicts: Vec<AIConflict>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AISingleSource {
    pub url: String,
    pub quote: String,
}

/// Transform single-answer response to Extraction.
pub fn transform_single_response(
    response: AISingleResponse,
    _config: &ExtractionTransformConfig,
) -> Extraction {
    let sources = if let Some(src) = response.source {
        vec![Source {
            url: src.url,
            title: None,
            fetched_at: chrono::Utc::now(),
            role: SourceRole::Primary,
            metadata: HashMap::new(),
        }]
    } else {
        vec![]
    };

    let conflicts: Vec<Conflict> = response
        .conflicts
        .into_iter()
        .map(|c| Conflict {
            topic: c.topic,
            claims: c
                .claims
                .into_iter()
                .map(|claim| ConflictingClaim {
                    statement: claim.statement,
                    source_url: claim.source_url,
                })
                .collect(),
        })
        .collect();

    let grounding = if !response.found {
        GroundingGrade::Inferred // Not found = uncertain
    } else if !conflicts.is_empty() {
        GroundingGrade::Conflicted
    } else if sources.is_empty() {
        GroundingGrade::Inferred
    } else {
        GroundingGrade::SingleSource
    };

    let gaps = if !response.found {
        vec![GapQuery::new("answer", &response.content)]
    } else {
        vec![]
    };

    Extraction {
        content: response.content,
        sources,
        gaps,
        grounding,
        conflicts,
    }
}

/// Parse a narrative response (for Narrative strategy).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AINarrativeResponse {
    pub content: String,
    #[serde(default)]
    pub sources: Vec<AINarrativeSource>,
    #[serde(default)]
    pub key_points: Vec<String>,
    #[serde(default)]
    pub conflicts: Vec<AIConflict>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AINarrativeSource {
    pub number: i32,
    pub url: String,
    #[serde(default)]
    pub title: Option<String>,
}

/// Transform narrative response to Extraction.
pub fn transform_narrative_response(
    response: AINarrativeResponse,
    config: &ExtractionTransformConfig,
) -> Extraction {
    let sources: Vec<Source> = response
        .sources
        .into_iter()
        .enumerate()
        .map(|(i, s)| Source {
            url: s.url,
            title: s.title,
            fetched_at: chrono::Utc::now(),
            role: if i == 0 {
                SourceRole::Primary
            } else {
                SourceRole::Supporting
            },
            metadata: HashMap::new(),
        })
        .collect();

    let conflicts: Vec<Conflict> = response
        .conflicts
        .into_iter()
        .map(|c| Conflict {
            topic: c.topic,
            claims: c
                .claims
                .into_iter()
                .map(|claim| ConflictingClaim {
                    statement: claim.statement,
                    source_url: claim.source_url,
                })
                .collect(),
        })
        .collect();

    let grounding = calculate_grounding(&sources, &conflicts, false, config);

    Extraction {
        content: response.content,
        sources,
        gaps: vec![],
        grounding,
        conflicts,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transform_extraction() {
        let response = AIExtractionResponse {
            content: "Extracted content".to_string(),
            claims: vec![
                AIClaim {
                    statement: "Claim 1".to_string(),
                    evidence: vec![AIEvidence {
                        quote: "Quote".to_string(),
                        source_url: "https://example.com".to_string(),
                    }],
                    grounding: "DIRECT".to_string(),
                },
                AIClaim {
                    statement: "Assumed claim".to_string(),
                    evidence: vec![],
                    grounding: "ASSUMED".to_string(),
                },
            ],
            sources: vec![
                AISource {
                    url: "https://example.com".to_string(),
                    role: Some("PRIMARY".to_string()),
                    title: Some("Example".to_string()),
                },
                AISource {
                    url: "https://other.com".to_string(),
                    role: None,
                    title: None,
                },
            ],
            gaps: vec![AIGap {
                field: "email".to_string(),
                query: "contact email".to_string(),
            }],
            conflicts: vec![],
        };

        let config = ExtractionTransformConfig::default();
        let extraction = transform_extraction(response, &config);

        assert_eq!(extraction.content, "Extracted content");
        assert_eq!(extraction.sources.len(), 2);
        assert_eq!(extraction.gaps.len(), 1);
        assert_eq!(extraction.grounding, GroundingGrade::Verified);
    }

    #[test]
    fn test_strict_mode_filters_assumed() {
        let response = AIExtractionResponse {
            content: "Content".to_string(),
            claims: vec![
                AIClaim {
                    statement: "Direct".to_string(),
                    evidence: vec![],
                    grounding: "DIRECT".to_string(),
                },
                AIClaim {
                    statement: "Assumed".to_string(),
                    evidence: vec![],
                    grounding: "ASSUMED".to_string(),
                },
            ],
            sources: vec![],
            gaps: vec![],
            conflicts: vec![],
        };

        let config = ExtractionTransformConfig {
            strict_mode: true,
            ..Default::default()
        };
        let extraction = transform_extraction(response, &config);

        // Assumed claim filtered, no inference flag
        assert_eq!(extraction.grounding, GroundingGrade::SingleSource);
    }

    #[test]
    fn test_conflicts_set_grounding() {
        let response = AIExtractionResponse {
            content: "Content".to_string(),
            claims: vec![],
            sources: vec![
                AISource {
                    url: "url1".to_string(),
                    role: None,
                    title: None,
                },
                AISource {
                    url: "url2".to_string(),
                    role: None,
                    title: None,
                },
            ],
            gaps: vec![],
            conflicts: vec![AIConflict {
                topic: "Schedule".to_string(),
                claims: vec![
                    AIConflictClaim {
                        statement: "Monday".to_string(),
                        source_url: "url1".to_string(),
                    },
                    AIConflictClaim {
                        statement: "Tuesday".to_string(),
                        source_url: "url2".to_string(),
                    },
                ],
            }],
        };

        let config = ExtractionTransformConfig::default();
        let extraction = transform_extraction(response, &config);

        assert_eq!(extraction.grounding, GroundingGrade::Conflicted);
        assert_eq!(extraction.conflicts.len(), 1);
    }

    #[test]
    fn test_parse_extraction_response() {
        let json = r#"{
            "content": "Test content",
            "claims": [],
            "sources": [{"url": "https://example.com"}],
            "gaps": [],
            "conflicts": []
        }"#;

        let response = parse_extraction_response(json).unwrap();
        assert_eq!(response.content, "Test content");
        assert_eq!(response.sources.len(), 1);
    }

    #[test]
    fn test_transform_single_response_found() {
        let response = AISingleResponse {
            content: "555-1234".to_string(),
            found: true,
            source: Some(AISingleSource {
                url: "https://example.com/contact".to_string(),
                quote: "Call us at 555-1234".to_string(),
            }),
            conflicts: vec![],
        };

        let config = ExtractionTransformConfig::default();
        let extraction = transform_single_response(response, &config);

        assert_eq!(extraction.content, "555-1234");
        assert_eq!(extraction.grounding, GroundingGrade::SingleSource);
        assert!(extraction.gaps.is_empty());
    }

    #[test]
    fn test_transform_single_response_not_found() {
        let response = AISingleResponse {
            content: "Not found".to_string(),
            found: false,
            source: None,
            conflicts: vec![],
        };

        let config = ExtractionTransformConfig::default();
        let extraction = transform_single_response(response, &config);

        assert_eq!(extraction.grounding, GroundingGrade::Inferred);
        assert_eq!(extraction.gaps.len(), 1);
    }
}
