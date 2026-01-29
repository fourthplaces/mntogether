use serde::{Deserialize, Serialize};

/// Configuration for detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectionConfig {
    pub kind: String,
    pub heuristics: Vec<Heuristic>,
    pub ai_prompt: Option<String>,
    pub confidence_threshold: f32,
    pub page_limit: Option<usize>,
}

impl DetectionConfig {
    pub fn new(kind: String) -> Self {
        Self {
            kind,
            heuristics: Vec::new(),
            ai_prompt: None,
            confidence_threshold: 0.5,
            page_limit: Some(100),
        }
    }

    pub fn with_heuristic(mut self, heuristic: Heuristic) -> Self {
        self.heuristics.push(heuristic);
        self
    }

    pub fn with_ai_prompt(mut self, prompt: String) -> Self {
        self.ai_prompt = Some(prompt);
        self
    }

    pub fn with_threshold(mut self, threshold: f32) -> Self {
        self.confidence_threshold = threshold;
        self
    }

    pub fn with_page_limit(mut self, limit: usize) -> Self {
        self.page_limit = Some(limit);
        self
    }
}

/// Heuristic detection methods
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Heuristic {
    Keywords { words: Vec<String> },
    UrlPattern { pattern: String },
    DomSelector { selectors: Vec<String> },
}

/// Rule for resolving relationships between extractions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipRule {
    pub from_kind: String,
    pub to_kind: String,
    pub relationship_type: String,
    pub same_page_required: bool,
    pub confidence_threshold: f32,
}

impl RelationshipRule {
    pub fn new(from_kind: String, to_kind: String, relationship_type: String) -> Self {
        Self {
            from_kind,
            to_kind,
            relationship_type,
            same_page_required: false,
            confidence_threshold: 0.5,
        }
    }

    pub fn same_page(mut self) -> Self {
        self.same_page_required = true;
        self
    }

    pub fn with_threshold(mut self, threshold: f32) -> Self {
        self.confidence_threshold = threshold;
        self
    }
}
