use crate::types::*;
use anyhow::{Context, Result};
use chrono::Utc;

/// Trait for AI extraction clients
#[async_trait::async_trait]
pub trait AIExtractor: Send + Sync {
    async fn extract(
        &self,
        content: &str,
        schema: &serde_json::Value,
    ) -> Result<(serde_json::Value, f32)>; // (extracted_data, confidence)
}

/// Extract structured data from a page using a schema
pub async fn extract_structured_data(
    detection: &Detection,
    snapshot: &PageSnapshot,
    schema: &Schema,
    ai_extractor: &impl AIExtractor,
) -> Result<Extraction> {
    tracing::debug!(
        detection_id = %detection.id.0,
        snapshot_id = %snapshot.id.0,
        schema_id = %schema.id.0,
        "Starting extraction"
    );

    // Use markdown if available, otherwise HTML
    let content = snapshot.markdown.as_ref().unwrap_or(&snapshot.html);

    // Extract using AI
    let (data, confidence) = ai_extractor
        .extract(content, &schema.json_schema)
        .await
        .context("Failed to extract data with AI")?;

    tracing::debug!(
        detection_id = %detection.id.0,
        confidence = confidence,
        "AI extraction completed"
    );

    // Calculate fingerprint for deduplication
    let fingerprint = Extraction::calculate_fingerprint(&data);

    // Create extraction with provenance
    let extraction = Extraction {
        id: ExtractionId::new(),
        fingerprint,
        page_snapshot_id: snapshot.id,
        schema_id: schema.id,
        schema_version: schema.version,
        data,
        confidence: ConfidenceScores::ai(confidence),
        origin: ExtractionOrigin::AI {
            model: "unknown".to_string(),
            prompt: format!("Extract according to schema: {}", schema.name),
        },
        field_provenance: generate_basic_provenance(&schema.json_schema),
        extracted_at: Utc::now(),
    };

    tracing::info!(
        extraction_id = %extraction.id.0,
        fingerprint = %extraction.fingerprint.to_hex(),
        confidence = extraction.confidence.overall,
        "Extraction created"
    );

    Ok(extraction)
}

/// Generate basic provenance information from schema
fn generate_basic_provenance(schema: &serde_json::Value) -> Vec<FieldProvenance> {
    let mut provenance = Vec::new();

    if let Some(obj) = schema.as_object() {
        if let Some(properties) = obj.get("properties").and_then(|p| p.as_object()) {
            for (field_name, _field_schema) in properties {
                provenance.push(FieldProvenance {
                    field_path: field_name.clone(),
                    source_location: "document".to_string(),
                    extraction_method: "ai".to_string(),
                });
            }
        }
    }

    provenance
}

/// Extract data from multiple detections in batch
pub async fn extract_batch(
    detections: Vec<(Detection, PageSnapshot)>,
    schema: &Schema,
    ai_extractor: &impl AIExtractor,
) -> Result<Vec<Extraction>> {
    let mut extractions = Vec::new();

    for (detection, snapshot) in detections {
        match extract_structured_data(&detection, &snapshot, schema, ai_extractor).await {
            Ok(extraction) => extractions.push(extraction),
            Err(e) => {
                tracing::warn!(
                    detection_id = %detection.id.0,
                    error = %e,
                    "Failed to extract from detection"
                );
            }
        }
    }

    Ok(extractions)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockAIExtractor {
        result: serde_json::Value,
    }

    #[async_trait::async_trait]
    impl AIExtractor for MockAIExtractor {
        async fn extract(
            &self,
            _content: &str,
            _schema: &serde_json::Value,
        ) -> Result<(serde_json::Value, f32)> {
            Ok((self.result.clone(), 0.95))
        }
    }

    #[tokio::test]
    async fn test_extract_structured_data() {
        let snapshot = PageSnapshot::new(
            "https://example.com".to_string(),
            "<html><body>Volunteer opportunity</body></html>".to_string(),
            Some("Volunteer opportunity".to_string()),
            "test".to_string(),
        );

        let detection = Detection {
            id: DetectionId::new(),
            page_snapshot_id: snapshot.id,
            kind: "volunteer_opportunity".to_string(),
            confidence: ConfidenceScores::heuristic(0.8),
            origin: DetectionOrigin::Heuristic {
                rules: vec!["test".to_string()],
            },
            evidence: vec![],
            detected_at: Utc::now(),
        };

        let schema = Schema {
            id: SchemaId::new(),
            name: "volunteer_opportunity".to_string(),
            version: 1,
            json_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "title": { "type": "string" },
                    "description": { "type": "string" }
                }
            }),
            created_at: Utc::now(),
        };

        let extractor = MockAIExtractor {
            result: serde_json::json!({
                "title": "Volunteer at Food Bank",
                "description": "Help distribute food to families"
            }),
        };

        let extraction = extract_structured_data(&detection, &snapshot, &schema, &extractor)
            .await
            .unwrap();

        assert_eq!(extraction.schema_id, schema.id);
        assert_eq!(extraction.confidence.overall, 0.95);
        assert!(extraction.data.is_object());
    }

    #[tokio::test]
    async fn test_fingerprint_deduplication() {
        let data1 = serde_json::json!({
            "title": "Test",
            "description": "Description"
        });

        let data2 = serde_json::json!({
            "title": "  test  ", // Different whitespace and case
            "description": "  description  "
        });

        let fp1 = Extraction::calculate_fingerprint(&data1);
        let fp2 = Extraction::calculate_fingerprint(&data2);

        // Should match due to normalization
        assert_eq!(fp1, fp2);
    }
}
