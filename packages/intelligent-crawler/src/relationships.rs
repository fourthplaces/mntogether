use crate::config::RelationshipRule;
use crate::types::*;
use anyhow::Result;
use chrono::Utc;
use std::collections::HashMap;

/// Trait for AI relationship resolvers
#[async_trait::async_trait]
pub trait AIRelationshipResolver: Send + Sync {
    async fn resolve(
        &self,
        from_data: &serde_json::Value,
        to_data: &serde_json::Value,
        relationship_type: &str,
    ) -> Result<(bool, f32, String)>; // (related, confidence, reasoning)
}

/// Resolve relationships between extractions based on rules
pub async fn resolve_relationships(
    extractions: &[Extraction],
    rules: &[RelationshipRule],
    ai_resolver: Option<&impl AIRelationshipResolver>,
) -> Result<Vec<Relationship>> {
    tracing::debug!(
        extraction_count = extractions.len(),
        rule_count = rules.len(),
        "Starting relationship resolution"
    );

    let mut relationships = Vec::new();

    // Group extractions by page for same_page_required rules
    let mut extractions_by_page: HashMap<PageSnapshotId, Vec<&Extraction>> = HashMap::new();
    for extraction in extractions {
        extractions_by_page
            .entry(extraction.page_snapshot_id)
            .or_default()
            .push(extraction);
    }

    // Get schema names (in real implementation, would query storage)
    // For now, use a simple heuristic based on data structure
    let get_kind = |extraction: &Extraction| -> String {
        // Extract kind from schema_id or data
        if let Some(obj) = extraction.data.as_object() {
            if let Some(kind) = obj.get("kind").and_then(|k| k.as_str()) {
                return kind.to_string();
            }
            if let Some(type_field) = obj.get("type").and_then(|t| t.as_str()) {
                return type_field.to_string();
            }
        }
        format!("extraction_{}", extraction.schema_id.0)
    };

    // Apply rules
    for rule in rules {
        tracing::debug!(
            from_kind = %rule.from_kind,
            to_kind = %rule.to_kind,
            relationship_type = %rule.relationship_type,
            "Applying relationship rule"
        );

        let candidates: Vec<(&Extraction, &Extraction)> = if rule.same_page_required {
            // Only consider extractions on the same page
            extractions_by_page
                .values()
                .flat_map(|page_extractions| {
                    page_extractions
                        .iter()
                        .flat_map(|from| {
                            page_extractions
                                .iter()
                                .filter(|to| from.id != to.id)
                                .map(move |to| (*from, *to))
                        })
                })
                .collect()
        } else {
            // Consider all extraction pairs
            extractions
                .iter()
                .flat_map(|from| {
                    extractions
                        .iter()
                        .filter(|to| from.id != to.id)
                        .map(move |to| (from, to))
                })
                .collect()
        };

        for (from, to) in candidates {
            let from_kind = get_kind(from);
            let to_kind = get_kind(to);

            // Check if kinds match rule
            if from_kind != rule.from_kind || to_kind != rule.to_kind {
                continue;
            }

            // Run heuristic checks
            let heuristic_result = check_heuristic_relationship(from, to, rule);

            // Run AI check if available and confidence is low
            let (related, confidence, origin) = if let Some(resolver) = ai_resolver {
                if let Some((h_conf, h_origin)) = heuristic_result {
                    if h_conf >= rule.confidence_threshold {
                        (true, h_conf, h_origin)
                    } else {
                        // Try AI to boost confidence
                        match resolver
                            .resolve(&from.data, &to.data, &rule.relationship_type)
                            .await
                        {
                            Ok((true, ai_conf, reasoning)) => {
                                let overall_conf = h_conf.max(ai_conf);
                                (
                                    overall_conf >= rule.confidence_threshold,
                                    overall_conf,
                                    RelationshipOrigin::AI {
                                        model: "unknown".to_string(),
                                        reasoning,
                                    },
                                )
                            }
                            _ => (false, h_conf, h_origin),
                        }
                    }
                } else {
                    // No heuristic match, try AI only
                    match resolver
                        .resolve(&from.data, &to.data, &rule.relationship_type)
                        .await
                    {
                        Ok((true, conf, reasoning)) if conf >= rule.confidence_threshold => {
                            (
                                true,
                                conf,
                                RelationshipOrigin::AI {
                                    model: "unknown".to_string(),
                                    reasoning,
                                },
                            )
                        }
                        _ => continue,
                    }
                }
            } else if let Some((conf, origin)) = heuristic_result {
                if conf >= rule.confidence_threshold {
                    (true, conf, origin)
                } else {
                    continue;
                }
            } else {
                continue;
            };

            if related {
                let relationship = Relationship {
                    id: RelationshipId::new(),
                    from_extraction_id: from.id,
                    to_extraction_id: to.id,
                    kind: rule.relationship_type.clone(),
                    confidence: ConfidenceScores::heuristic(confidence),
                    origin,
                    metadata: HashMap::new(),
                    created_at: Utc::now(),
                };

                tracing::debug!(
                    relationship_id = %relationship.id.0,
                    from_id = %from.id.0,
                    to_id = %to.id.0,
                    confidence = confidence,
                    "Relationship created"
                );

                relationships.push(relationship);
            }
        }
    }

    tracing::info!(
        relationships_found = relationships.len(),
        "Relationship resolution completed"
    );

    Ok(relationships)
}

/// Check for heuristic-based relationships
fn check_heuristic_relationship(
    from: &Extraction,
    to: &Extraction,
    rule: &RelationshipRule,
) -> Option<(f32, RelationshipOrigin)> {
    // Same page heuristic
    if rule.same_page_required && from.page_snapshot_id == to.page_snapshot_id {
        return Some((
            0.7,
            RelationshipOrigin::Heuristic {
                rules: vec!["same_page".to_string()],
            },
        ));
    }

    // Check for explicit references in data
    let from_obj = from.data.as_object()?;
    let to_obj = to.data.as_object()?;

    // Check if 'from' references 'to' by ID or name
    if let Some(from_refs) = from_obj.get("references").and_then(|r| r.as_array()) {
        if let Some(to_id) = to_obj.get("id").and_then(|i| i.as_str()) {
            for reference in from_refs {
                if let Some(ref_str) = reference.as_str() {
                    if ref_str == to_id {
                        return Some((
                            1.0,
                            RelationshipOrigin::Explicit {
                                source: "data.references".to_string(),
                            },
                        ));
                    }
                }
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_same_page_relationship() {
        let snapshot_id = PageSnapshotId::new();
        let schema_id = SchemaId::new();

        let extraction1 = Extraction {
            id: ExtractionId::new(),
            fingerprint: ContentHash::from_content("test1"),
            page_snapshot_id: snapshot_id,
            schema_id,
            schema_version: 1,
            data: serde_json::json!({
                "type": "organization",
                "name": "Test Org"
            }),
            confidence: ConfidenceScores::heuristic(0.9),
            origin: ExtractionOrigin::Heuristic {
                rules: vec!["test".to_string()],
            },
            field_provenance: vec![],
            extracted_at: Utc::now(),
        };

        let extraction2 = Extraction {
            id: ExtractionId::new(),
            fingerprint: ContentHash::from_content("test2"),
            page_snapshot_id: snapshot_id,
            schema_id,
            schema_version: 1,
            data: serde_json::json!({
                "type": "volunteer_opportunity",
                "title": "Volunteer"
            }),
            confidence: ConfidenceScores::heuristic(0.9),
            origin: ExtractionOrigin::Heuristic {
                rules: vec!["test".to_string()],
            },
            field_provenance: vec![],
            extracted_at: Utc::now(),
        };

        let rule = RelationshipRule::new(
            "organization".to_string(),
            "volunteer_opportunity".to_string(),
            "offers".to_string(),
        )
        .same_page()
        .with_threshold(0.5);

        let relationships = resolve_relationships(
            &[extraction1, extraction2],
            &[rule],
            None::<&MockAIResolver>,
        )
        .await
        .unwrap();

        assert_eq!(relationships.len(), 1);
        assert_eq!(relationships[0].kind, "offers");
    }

    struct MockAIResolver;

    #[async_trait::async_trait]
    impl AIRelationshipResolver for MockAIResolver {
        async fn resolve(
            &self,
            _from_data: &serde_json::Value,
            _to_data: &serde_json::Value,
            _relationship_type: &str,
        ) -> Result<(bool, f32, String)> {
            Ok((true, 0.9, "Test reasoning".to_string()))
        }
    }
}
