//! LLM-based PII detection
//!
//! Uses the OpenAIClient for context-aware PII detection that can
//! catch unstructured PII like names and addresses that regex misses.

use anyhow::Result;
use ai_client::OpenRouter;
use crate::kernel::FRONTIER_MODEL;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::detector::{PiiFindings, PiiType};

/// PII entity detected by LLM with context
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PiiEntity {
    pub entity_type: String, // "person_name", "street_address", "organization_name", etc.
    pub value: String,
    pub confidence: f64, // 0.0 to 1.0 — f64 for JsonSchema compatibility
    pub context: Option<String>,
}

/// Wrapper for OpenAI structured output (must be top-level object)
#[derive(Debug, Clone, Deserialize, JsonSchema)]
struct PiiDetectionResponse {
    pub entities: Vec<PiiEntity>,
}

/// System prompt for PII detection
const PII_DETECTION_PROMPT: &str = r#"You are a PII (Personally Identifiable Information) detection system.
Analyze the provided text and identify any PII that could identify a specific individual.

Detect:
- Person names (full names, first + last)
- Street addresses (with house numbers)
- Medical information (diagnoses, medications, conditions)
- Financial information (account numbers, banking details)
- Government IDs (driver's license numbers, passport numbers)
- Personal characteristics that could identify someone

DO NOT flag:
- Organization names alone (unless part of personal context)
- Generic locations (city, state, country)
- Generic email domains or phone area codes
- Job titles or roles without names

For each detected PII entity, provide:
- entity_type: "person_name", "street_address", "medical_info", "financial_info", "government_id", "email", "phone", "ssn", "credit_card", "ip_address"
- value: the exact PII text found
- confidence: 0.0 to 1.0
- context: brief description of where/how it appears (or null)

If no PII is detected, return an empty entities array."#;

/// Detect PII using an AI model for context-aware analysis.
///
/// This detects unstructured PII like names, addresses, and medical info
/// that regex patterns cannot reliably catch.
pub async fn detect_pii_with_ai(text: &str, ai: &OpenRouter) -> Result<Vec<PiiEntity>> {
    if text.trim().is_empty() {
        return Ok(Vec::new());
    }

    let user_prompt = format!("Analyze this text for PII:\n\n{}", text);

    let response: PiiDetectionResponse = ai
        .extract(FRONTIER_MODEL, PII_DETECTION_PROMPT, &user_prompt)
        .await
        .map_err(|e| anyhow::anyhow!("PII detection failed: {}", e))?;

    Ok(response.entities)
}

/// Legacy function that takes an API key directly.
///
/// Creates an OpenRouter client internally. Prefer `detect_pii_with_ai` for
/// better testability and consistency with the rest of the codebase.
pub async fn detect_pii_with_gpt(text: &str, openrouter_api_key: &str) -> Result<Vec<PiiEntity>> {
    let ai = OpenRouter::new(openrouter_api_key.to_string(), FRONTIER_MODEL);
    detect_pii_with_ai(text, &ai).await
}

/// Convert LLM-detected entities to PiiFindings format
/// This allows combining regex and LLM detections
pub fn entities_to_findings(text: &str, entities: &[PiiEntity]) -> PiiFindings {
    let mut findings = PiiFindings::new();

    for entity in entities {
        // Find the position of this entity in the text
        if let Some(start) = text.find(&entity.value) {
            let end = start + entity.value.len();

            // Map entity type to PiiType (best effort)
            // Note: LLM detects types we don't have enums for (names, addresses)
            // For now, we'll skip mapping these and handle them separately
            // in the redaction logic

            // Only add if it's a type we can map
            match entity.entity_type.as_str() {
                "email" | "email_address" => {
                    findings.add(PiiType::Email, entity.value.clone(), start, end);
                }
                "phone" | "phone_number" => {
                    findings.add(PiiType::Phone, entity.value.clone(), start, end);
                }
                "ssn" | "social_security_number" => {
                    findings.add(PiiType::Ssn, entity.value.clone(), start, end);
                }
                "credit_card" | "credit_card_number" => {
                    findings.add(PiiType::CreditCard, entity.value.clone(), start, end);
                }
                "ip_address" => {
                    findings.add(PiiType::IpAddress, entity.value.clone(), start, end);
                }
                _ => {
                    // Names, addresses, etc. - we'd need to extend PiiType enum
                    // For MVP, log these but don't add to findings
                    tracing::debug!(
                        entity_type = %entity.entity_type,
                        value = %entity.value,
                        "Detected unstructured PII (not yet supported in enum)"
                    );
                }
            }
        }
    }

    findings
}

/// Hybrid detection: combines regex and LLM
pub async fn detect_pii_hybrid(text: &str, api_key: &str) -> Result<PiiFindings> {
    use super::detector::detect_structured_pii;

    // Start with regex detection (fast, reliable for structured data)
    let mut findings = detect_structured_pii(text);

    // Add LLM detection for unstructured PII
    let entities = detect_pii_with_gpt(text, api_key).await?;
    let llm_findings = entities_to_findings(text, &entities);

    // Merge findings (deduplicating overlaps)
    for new_match in llm_findings.matches {
        // Check if this overlaps with existing matches
        let overlaps = findings.matches.iter().any(|existing| {
            // Check for overlap
            (new_match.start >= existing.start && new_match.start < existing.end)
                || (new_match.end > existing.start && new_match.end <= existing.end)
                || (new_match.start <= existing.start && new_match.end >= existing.end)
        });

        if !overlaps {
            findings.matches.push(new_match);
        }
    }

    Ok(findings)
}

/// Hybrid detection with an AI client instance (preferred for testing)
pub async fn detect_pii_hybrid_with_ai(text: &str, ai: &OpenRouter) -> Result<PiiFindings> {
    use super::detector::detect_structured_pii;

    // Start with regex detection (fast, reliable for structured data)
    let mut findings = detect_structured_pii(text);

    // Add LLM detection for unstructured PII
    let entities = detect_pii_with_ai(text, ai).await?;
    let llm_findings = entities_to_findings(text, &entities);

    // Merge findings (deduplicating overlaps)
    for new_match in llm_findings.matches {
        let overlaps = findings.matches.iter().any(|existing| {
            (new_match.start >= existing.start && new_match.start < existing.end)
                || (new_match.end > existing.start && new_match.end <= existing.end)
                || (new_match.start <= existing.start && new_match.end >= existing.end)
        });

        if !overlaps {
            findings.matches.push(new_match);
        }
    }

    Ok(findings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entities_to_findings() {
        let text = "Contact John Smith at john@example.com";
        let entities = vec![
            PiiEntity {
                entity_type: "email".to_string(),
                value: "john@example.com".to_string(),
                confidence: 0.99,
                context: None,
            },
            PiiEntity {
                entity_type: "person_name".to_string(),
                value: "John Smith".to_string(),
                confidence: 0.95,
                context: Some("contact person".to_string()),
            },
        ];

        let findings = entities_to_findings(text, &entities);

        // Should detect the email (which we can map)
        let emails = findings.by_type(&PiiType::Email);
        assert_eq!(emails.len(), 1);
        assert_eq!(emails[0].value, "john@example.com");

        // Note: "person_name" is not in PiiType enum yet, so it won't be added
    }

    #[test]
    fn test_empty_text() {
        let entities = vec![];
        let findings = entities_to_findings("", &entities);
        assert!(findings.is_empty());
    }

    // Integration test - requires API key
    #[tokio::test]
    #[ignore] // Only run with: cargo test -- --ignored
    async fn test_gpt_detection() {
        let api_key = std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY not set");

        let text = "My name is Jane Doe and I live at 123 Main Street, Springfield. \
                    Call me at (555) 123-4567 or email jane@example.com";

        let entities = detect_pii_with_gpt(text, &api_key).await.unwrap();

        // Should detect multiple types of PII
        assert!(!entities.is_empty());

        // Check we detected at least some PII
        let has_name = entities.iter().any(|e| e.entity_type.contains("name"));
        let has_address = entities.iter().any(|e| e.entity_type.contains("address"));

        assert!(has_name || has_address, "Should detect names or addresses");
    }
}
