use crate::config::{DetectionConfig, Heuristic};
use crate::types::*;
use anyhow::Result;
use chrono::Utc;

/// Trait for AI detection clients
#[async_trait::async_trait]
pub trait AIDetector: Send + Sync {
    async fn detect(
        &self,
        content: &str,
        prompt: &str,
    ) -> Result<(bool, f32, String)>; // (detected, confidence, reasoning)
}

/// Detect information in a page using heuristics and/or AI
pub async fn detect_information(
    snapshot: &PageSnapshot,
    config: &DetectionConfig,
    ai_detector: Option<&impl AIDetector>,
) -> Result<Option<Detection>> {
    tracing::debug!(
        snapshot_id = %snapshot.id.0,
        kind = %config.kind,
        "Starting detection"
    );

    let mut evidence = Vec::new();
    let mut heuristic_confidence: Option<f32> = None;
    let mut heuristic_origin: Option<DetectionOrigin> = None;

    // Run heuristic detection if configured
    if !config.heuristics.is_empty() {
        let (detected, conf, evid, origin) =
            run_heuristics(&snapshot, &config.heuristics)?;

        if detected {
            heuristic_confidence = Some(conf);
            heuristic_origin = Some(origin);
            evidence.extend(evid);
            tracing::debug!(
                snapshot_id = %snapshot.id.0,
                confidence = conf,
                "Heuristic detection positive"
            );
        } else {
            tracing::debug!(
                snapshot_id = %snapshot.id.0,
                "Heuristic detection negative"
            );
        }
    }

    // Run AI detection if configured
    let mut ai_confidence: Option<f32> = None;
    let mut ai_origin: Option<DetectionOrigin> = None;

    if let (Some(prompt), Some(detector)) = (&config.ai_prompt, ai_detector) {
        // Use markdown if available, otherwise HTML
        let content = snapshot.markdown.as_ref().unwrap_or(&snapshot.html);

        let (detected, conf, reasoning) = detector
            .detect(content, prompt)
            .await?;

        if detected {
            ai_confidence = Some(conf);
            ai_origin = Some(DetectionOrigin::AI {
                model: "unknown".to_string(),
                prompt: prompt.clone(),
            });
            evidence.push(Evidence::AIReasoning {
                explanation: reasoning,
            });
            tracing::debug!(
                snapshot_id = %snapshot.id.0,
                confidence = conf,
                "AI detection positive"
            );
        } else {
            tracing::debug!(
                snapshot_id = %snapshot.id.0,
                "AI detection negative"
            );
        }
    }

    // Combine results
    let (confidence, origin) = match (heuristic_confidence, ai_confidence) {
        (Some(h), Some(a)) => {
            let scores = ConfidenceScores::hybrid(h, a);
            let origin = DetectionOrigin::Hybrid {
                heuristic: Box::new(heuristic_origin.unwrap()),
                ai: Box::new(ai_origin.unwrap()),
            };
            (scores, origin)
        }
        (Some(h), None) => (
            ConfidenceScores::heuristic(h),
            heuristic_origin.unwrap(),
        ),
        (None, Some(a)) => (
            ConfidenceScores::ai(a),
            ai_origin.unwrap(),
        ),
        (None, None) => {
            tracing::debug!(
                snapshot_id = %snapshot.id.0,
                "No detection methods ran"
            );
            return Ok(None);
        }
    };

    // Check against threshold
    if confidence.overall < config.confidence_threshold {
        tracing::debug!(
            snapshot_id = %snapshot.id.0,
            confidence = confidence.overall,
            threshold = config.confidence_threshold,
            "Detection below threshold"
        );
        return Ok(None);
    }

    let detection = Detection {
        id: DetectionId::new(),
        page_snapshot_id: snapshot.id,
        kind: config.kind.clone(),
        confidence,
        origin,
        evidence,
        detected_at: Utc::now(),
    };

    tracing::info!(
        snapshot_id = %snapshot.id.0,
        detection_id = %detection.id.0,
        confidence = detection.confidence.overall,
        "Detection created"
    );

    Ok(Some(detection))
}

/// Run heuristic detection
fn run_heuristics(
    snapshot: &PageSnapshot,
    heuristics: &[Heuristic],
) -> Result<(bool, f32, Vec<Evidence>, DetectionOrigin)> {
    let mut detected = false;
    let mut evidence = Vec::new();
    let mut rule_names = Vec::new();
    let mut total_score = 0.0;
    let mut rule_count = 0;

    let content = snapshot.markdown.as_ref().unwrap_or(&snapshot.html);

    for heuristic in heuristics {
        match heuristic {
            Heuristic::Keywords { words } => {
                let content_lower = content.to_lowercase();
                let mut matched_keywords = Vec::new();
                let mut locations = Vec::new();

                for word in words {
                    if content_lower.contains(&word.to_lowercase()) {
                        matched_keywords.push(word.clone());
                        locations.push(format!("keyword: {}", word));
                    }
                }

                if !matched_keywords.is_empty() {
                    detected = true;
                    evidence.push(Evidence::KeywordMatch {
                        keywords: matched_keywords.clone(),
                        locations: locations.clone(),
                    });
                    rule_names.push(format!("keywords:{}", matched_keywords.join(",")));
                    total_score += 0.7; // Keyword matches have medium confidence
                    rule_count += 1;
                }
            }
            Heuristic::UrlPattern { pattern } => {
                if snapshot.url.contains(pattern) {
                    detected = true;
                    evidence.push(Evidence::UrlPattern {
                        pattern: pattern.clone(),
                        matched: true,
                    });
                    rule_names.push(format!("url_pattern:{}", pattern));
                    total_score += 0.9; // URL patterns have high confidence
                    rule_count += 1;
                }
            }
            Heuristic::DomSelector { selectors } => {
                // Simple heuristic: check if selector strings appear in HTML
                let mut found_count = 0;
                for selector in selectors {
                    if snapshot.html.contains(selector) {
                        found_count += 1;
                    }
                }

                if found_count > 0 {
                    detected = true;
                    evidence.push(Evidence::DomSelector {
                        selectors: selectors.clone(),
                        found_count,
                    });
                    rule_names.push(format!("dom_selector:{}found", found_count));
                    total_score += 0.6; // DOM selector has lower confidence without proper parsing
                    rule_count += 1;
                }
            }
        }
    }

    let confidence = if rule_count > 0 {
        (total_score / rule_count as f32).min(1.0)
    } else {
        0.0
    };

    let origin = DetectionOrigin::Heuristic {
        rules: rule_names,
    };

    Ok((detected, confidence, evidence, origin))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_keyword_detection() {
        let snapshot = PageSnapshot::new(
            "https://example.com".to_string(),
            "<html><body>We need volunteers for our community project</body></html>".to_string(),
            Some("We need volunteers for our community project".to_string()),
            "test".to_string(),
        );

        let config = DetectionConfig::new("volunteer_opportunity".to_string())
            .with_heuristic(Heuristic::Keywords {
                words: vec!["volunteer".to_string(), "volunteers".to_string()],
            })
            .with_threshold(0.5);

        let detection = detect_information(&snapshot, &config, None::<&MockAIDetector>)
            .await
            .unwrap();

        assert!(detection.is_some());
        let detection = detection.unwrap();
        assert_eq!(detection.kind, "volunteer_opportunity");
        assert!(detection.confidence.overall >= 0.5);
    }

    #[tokio::test]
    async fn test_url_pattern_detection() {
        let snapshot = PageSnapshot::new(
            "https://example.com/volunteer-opportunities".to_string(),
            "<html><body>Test</body></html>".to_string(),
            None,
            "test".to_string(),
        );

        let config = DetectionConfig::new("volunteer_opportunity".to_string())
            .with_heuristic(Heuristic::UrlPattern {
                pattern: "volunteer".to_string(),
            })
            .with_threshold(0.5);

        let detection = detect_information(&snapshot, &config, None::<&MockAIDetector>)
            .await
            .unwrap();

        assert!(detection.is_some());
        let detection = detection.unwrap();
        assert!(detection.confidence.overall >= 0.8);
    }

    struct MockAIDetector;

    #[async_trait::async_trait]
    impl AIDetector for MockAIDetector {
        async fn detect(
            &self,
            _content: &str,
            _prompt: &str,
        ) -> Result<(bool, f32, String)> {
            Ok((true, 0.9, "Test reasoning".to_string()))
        }
    }
}
