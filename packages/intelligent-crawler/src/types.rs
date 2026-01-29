use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use uuid::Uuid;

/// Unique identifier for a page snapshot
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PageSnapshotId(pub Uuid);

impl PageSnapshotId {
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }
}

impl Default for PageSnapshotId {
    fn default() -> Self {
        Self::new()
    }
}

/// Unique identifier for a detection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DetectionId(pub Uuid);

impl DetectionId {
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }
}

impl Default for DetectionId {
    fn default() -> Self {
        Self::new()
    }
}

/// Unique identifier for an extraction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ExtractionId(pub Uuid);

impl ExtractionId {
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }
}

impl Default for ExtractionId {
    fn default() -> Self {
        Self::new()
    }
}

/// Unique identifier for a relationship
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RelationshipId(pub Uuid);

impl RelationshipId {
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }
}

impl Default for RelationshipId {
    fn default() -> Self {
        Self::new()
    }
}

/// Unique identifier for a schema
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SchemaId(pub Uuid);

impl SchemaId {
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }
}

impl Default for SchemaId {
    fn default() -> Self {
        Self::new()
    }
}

/// Content hash for deduplication
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ContentHash(pub Vec<u8>);

impl ContentHash {
    pub fn from_content(content: &str) -> Self {
        let normalized = normalize_content(content);
        let mut hasher = Sha256::new();
        hasher.update(normalized.as_bytes());
        Self(hasher.finalize().to_vec())
    }

    pub fn to_hex(&self) -> String {
        hex::encode(&self.0)
    }
}

/// Normalize content for consistent hashing
fn normalize_content(content: &str) -> String {
    content
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Confidence scores for various detection/extraction methods
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidenceScores {
    pub overall: f32,
    pub heuristic: Option<f32>,
    pub ai: Option<f32>,
}

impl ConfidenceScores {
    pub fn deterministic() -> Self {
        Self {
            overall: 1.0,
            heuristic: None,
            ai: None,
        }
    }

    pub fn heuristic(score: f32) -> Self {
        Self {
            overall: score,
            heuristic: Some(score),
            ai: None,
        }
    }

    pub fn ai(score: f32) -> Self {
        Self {
            overall: score,
            heuristic: None,
            ai: Some(score),
        }
    }

    pub fn hybrid(heuristic_score: f32, ai_score: f32) -> Self {
        // Take the maximum of the two scores as overall
        let overall = heuristic_score.max(ai_score);
        Self {
            overall,
            heuristic: Some(heuristic_score),
            ai: Some(ai_score),
        }
    }
}

/// Origin of a detection
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum DetectionOrigin {
    Deterministic,
    Heuristic { rules: Vec<String> },
    AI { model: String, prompt: String },
    Hybrid { heuristic: Box<DetectionOrigin>, ai: Box<DetectionOrigin> },
}

/// Evidence supporting a detection
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Evidence {
    KeywordMatch { keywords: Vec<String>, locations: Vec<String> },
    UrlPattern { pattern: String, matched: bool },
    DomSelector { selectors: Vec<String>, found_count: usize },
    AIReasoning { explanation: String },
}

/// A detection that a page contains relevant information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Detection {
    pub id: DetectionId,
    pub page_snapshot_id: PageSnapshotId,
    pub kind: String,
    pub confidence: ConfidenceScores,
    pub origin: DetectionOrigin,
    pub evidence: Vec<Evidence>,
    pub detected_at: DateTime<Utc>,
}

/// Origin of an extraction
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ExtractionOrigin {
    Deterministic { method: String },
    Heuristic { rules: Vec<String> },
    AI { model: String, prompt: String },
    Hybrid { heuristic: Box<ExtractionOrigin>, ai: Box<ExtractionOrigin> },
}

/// Provenance information for an extracted field
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldProvenance {
    pub field_path: String,
    pub source_location: String, // CSS selector, XPath, or text location
    pub extraction_method: String,
}

/// An extraction of structured data from a page
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Extraction {
    pub id: ExtractionId,
    pub fingerprint: ContentHash,
    pub page_snapshot_id: PageSnapshotId,
    pub schema_id: SchemaId,
    pub schema_version: u32,
    pub data: serde_json::Value,
    pub confidence: ConfidenceScores,
    pub origin: ExtractionOrigin,
    pub field_provenance: Vec<FieldProvenance>,
    pub extracted_at: DateTime<Utc>,
}

impl Extraction {
    /// Calculate fingerprint from normalized data
    pub fn calculate_fingerprint(data: &serde_json::Value) -> ContentHash {
        let normalized = normalize_json(data);
        let json_str = serde_json::to_string(&normalized).unwrap_or_default();
        ContentHash::from_content(&json_str)
    }
}

/// Normalize JSON for consistent fingerprinting
fn normalize_json(value: &serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::String(s) => {
            serde_json::Value::String(s.trim().to_lowercase())
        }
        serde_json::Value::Object(map) => {
            let mut normalized = serde_json::Map::new();
            for (k, v) in map {
                normalized.insert(k.clone(), normalize_json(v));
            }
            serde_json::Value::Object(normalized)
        }
        serde_json::Value::Array(arr) => {
            serde_json::Value::Array(arr.iter().map(normalize_json).collect())
        }
        other => other.clone(),
    }
}

/// Origin of a relationship
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum RelationshipOrigin {
    Explicit { source: String },
    Heuristic { rules: Vec<String> },
    AI { model: String, reasoning: String },
}

/// A relationship between two extractions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    pub id: RelationshipId,
    pub from_extraction_id: ExtractionId,
    pub to_extraction_id: ExtractionId,
    pub kind: String,
    pub confidence: ConfidenceScores,
    pub origin: RelationshipOrigin,
    pub metadata: HashMap<String, serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

/// A schema definition for structured extraction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schema {
    pub id: SchemaId,
    pub name: String,
    pub version: u32,
    pub json_schema: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

/// An immutable snapshot of a crawled page
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageSnapshot {
    pub id: PageSnapshotId,
    pub url: String,
    pub content_hash: ContentHash,
    pub html: String,
    pub markdown: Option<String>,
    pub fetched_via: String, // "firecrawl", "manual", etc.
    pub metadata: HashMap<String, serde_json::Value>,
    pub crawled_at: DateTime<Utc>,
}

impl PageSnapshot {
    pub fn new(
        url: String,
        html: String,
        markdown: Option<String>,
        fetched_via: String,
    ) -> Self {
        let content_hash = ContentHash::from_content(&html);
        Self {
            id: PageSnapshotId::new(),
            url,
            content_hash,
            html,
            markdown,
            fetched_via,
            metadata: HashMap::new(),
            crawled_at: Utc::now(),
        }
    }
}
