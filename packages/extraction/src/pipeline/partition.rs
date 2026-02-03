//! Partition module - group pages into buckets for extraction.

use serde::{Deserialize, Serialize};

use crate::traits::ai::Partition;
use crate::types::summary::Summary;

/// AI response for partitioning.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIPartitionResponse {
    #[serde(default)]
    pub partitions: Vec<AIPartition>,
}

/// A partition from the AI response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIPartition {
    pub title: String,
    #[serde(default)]
    pub urls: Vec<String>,
    #[serde(default)]
    pub rationale: Option<String>,
}

/// Parse partition response from AI.
pub fn parse_partition_response(json: &str) -> Result<Vec<Partition>, serde_json::Error> {
    // Try parsing as array first (common format)
    if let Ok(partitions) = serde_json::from_str::<Vec<AIPartition>>(json) {
        return Ok(partitions
            .into_iter()
            .map(|p| {
                Partition::new(p.title)
                    .with_urls(p.urls)
                    .with_rationale(p.rationale.unwrap_or_default())
            })
            .collect());
    }

    // Try parsing as object with partitions field
    let response: AIPartitionResponse = serde_json::from_str(json)?;
    Ok(response
        .partitions
        .into_iter()
        .map(|p| {
            Partition::new(p.title)
                .with_urls(p.urls)
                .with_rationale(p.rationale.unwrap_or_default())
        })
        .collect())
}

/// Validate partitions against available summaries.
pub fn validate_partitions(partitions: &[Partition], summaries: &[Summary]) -> Vec<Partition> {
    let valid_urls: std::collections::HashSet<_> = summaries.iter().map(|s| &s.url).collect();

    partitions
        .iter()
        .map(|p| {
            let valid_partition_urls: Vec<_> = p
                .urls
                .iter()
                .filter(|url| valid_urls.contains(url))
                .cloned()
                .collect();

            Partition {
                title: p.title.clone(),
                urls: valid_partition_urls,
                rationale: p.rationale.clone(),
            }
        })
        .filter(|p| !p.urls.is_empty())
        .collect()
}

/// Merge small partitions that might be duplicates.
pub fn merge_similar_partitions(partitions: Vec<Partition>, threshold: f32) -> Vec<Partition> {
    if partitions.len() <= 1 {
        return partitions;
    }

    let mut result: Vec<Partition> = Vec::new();

    for partition in partitions {
        let mut merged = false;

        for existing in &mut result {
            // Check URL overlap
            let overlap = partition
                .urls
                .iter()
                .filter(|url| existing.urls.contains(url))
                .count();

            let overlap_ratio = if partition.urls.is_empty() {
                0.0
            } else {
                overlap as f32 / partition.urls.len() as f32
            };

            if overlap_ratio >= threshold {
                // Merge into existing
                for url in &partition.urls {
                    if !existing.urls.contains(url) {
                        existing.urls.push(url.clone());
                    }
                }
                merged = true;
                break;
            }
        }

        if !merged {
            result.push(partition);
        }
    }

    result
}

/// Create a default partition when AI returns nothing useful.
pub fn default_partition(summaries: &[Summary]) -> Vec<Partition> {
    if summaries.is_empty() {
        return vec![];
    }

    // Group by similar content (simplified - just one partition)
    vec![Partition::new("All Results").with_urls(summaries.iter().map(|s| s.url.clone()))]
}

/// Split a large partition into smaller chunks.
pub fn split_large_partition(partition: Partition, max_urls: usize) -> Vec<Partition> {
    if partition.urls.len() <= max_urls {
        return vec![partition];
    }

    partition
        .urls
        .chunks(max_urls)
        .enumerate()
        .map(|(i, urls)| {
            Partition::new(format!("{} (Part {})", partition.title, i + 1))
                .with_urls(urls.iter().cloned())
                .with_rationale(&partition.rationale)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::summary::RecallSignals;

    fn mock_summary(url: &str) -> Summary {
        Summary {
            url: url.to_string(),
            site_url: "https://example.com".to_string(),
            text: "Summary".to_string(),
            signals: RecallSignals::default(),
            language: None,
            created_at: chrono::Utc::now(),
            prompt_hash: "hash".to_string(),
            content_hash: "hash".to_string(),
            embedding: None,
        }
    }

    #[test]
    fn test_parse_partition_response_array() {
        let json = r#"[
            {"title": "Item 1", "urls": ["url1", "url2"], "rationale": "Because"},
            {"title": "Item 2", "urls": ["url3"]}
        ]"#;

        let partitions = parse_partition_response(json).unwrap();
        assert_eq!(partitions.len(), 2);
        assert_eq!(partitions[0].title, "Item 1");
        assert_eq!(partitions[0].urls.len(), 2);
    }

    #[test]
    fn test_parse_partition_response_object() {
        let json = r#"{
            "partitions": [
                {"title": "Item", "urls": ["url1"]}
            ]
        }"#;

        let partitions = parse_partition_response(json).unwrap();
        assert_eq!(partitions.len(), 1);
    }

    #[test]
    fn test_validate_partitions() {
        let summaries = vec![mock_summary("url1"), mock_summary("url2")];

        let partitions = vec![
            Partition::new("Valid").with_urls(["url1", "url2"]),
            Partition::new("Partial").with_urls(["url2", "url3"]), // url3 invalid
            Partition::new("Invalid").with_urls(["url4"]),         // all invalid
        ];

        let validated = validate_partitions(&partitions, &summaries);

        assert_eq!(validated.len(), 2);
        assert_eq!(validated[0].urls.len(), 2);
        assert_eq!(validated[1].urls.len(), 1); // Only url2
    }

    #[test]
    fn test_merge_similar_partitions() {
        let partitions = vec![
            Partition::new("A").with_urls(["url1", "url2"]),
            Partition::new("B").with_urls(["url1", "url2", "url3"]), // 66% overlap
            Partition::new("C").with_urls(["url4", "url5"]),         // No overlap
        ];

        let merged = merge_similar_partitions(partitions, 0.5);

        assert_eq!(merged.len(), 2);
        // A and B should be merged
        assert!(merged[0].urls.contains(&"url3".to_string()));
    }

    #[test]
    fn test_split_large_partition() {
        let partition = Partition::new("Large").with_urls(["u1", "u2", "u3", "u4", "u5"]);

        let split = split_large_partition(partition, 2);

        assert_eq!(split.len(), 3);
        assert_eq!(split[0].urls.len(), 2);
        assert_eq!(split[1].urls.len(), 2);
        assert_eq!(split[2].urls.len(), 1);
    }

    #[test]
    fn test_default_partition() {
        let summaries = vec![mock_summary("url1"), mock_summary("url2")];

        let partitions = default_partition(&summaries);

        assert_eq!(partitions.len(), 1);
        assert_eq!(partitions[0].urls.len(), 2);
    }
}
