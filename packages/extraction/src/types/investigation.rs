//! Investigation planning types for the Detective Engine.
//!
//! These types represent the **mechanical** aspect of investigation:
//! - What steps could be taken to resolve gaps
//! - What actions the library can perform
//!
//! **Policy** decisions (token budgets, iteration limits, retry logic)
//! belong in the caller's orchestrator, not here.
//!
//! # Design Principle: Mechanism vs Policy
//!
//! | Mechanism (Library) | Policy (Caller) |
//! |---------------------|-----------------|
//! | plan_investigation() | max_iterations |
//! | execute_step() | token_budget |
//! | InvestigationPlan | ghost_gap_prevention |
//! | InvestigationStep | when to give up |

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::extraction::GapQuery;

/// A suggested investigation step (pure data, no behavior).
///
/// The library suggests steps; the caller decides whether to execute them.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvestigationStep {
    /// ID of the gap being addressed
    pub gap_id: Uuid,

    /// Human-readable field name (e.g., "contact email")
    pub field: String,

    /// Original gap query for reference
    pub original_query: String,

    /// Recommended action to resolve this gap
    pub recommended_action: InvestigationAction,

    /// Why this action was recommended
    pub rationale: Option<String>,
}

impl InvestigationStep {
    /// Create a new investigation step.
    pub fn new(
        gap_id: Uuid,
        field: impl Into<String>,
        query: impl Into<String>,
        action: InvestigationAction,
    ) -> Self {
        Self {
            gap_id,
            field: field.into(),
            original_query: query.into(),
            recommended_action: action,
            rationale: None,
        }
    }

    /// Add a rationale for this step.
    pub fn with_rationale(mut self, rationale: impl Into<String>) -> Self {
        self.rationale = Some(rationale.into());
        self
    }

    /// Create a step from a GapQuery.
    pub fn from_gap(gap_id: Uuid, gap: &GapQuery, action: InvestigationAction) -> Self {
        Self::new(gap_id, &gap.field, &gap.query, action)
    }
}

/// Mechanical actions the library can perform.
///
/// These are the primitives available for gap resolution.
/// The library executes them; the caller decides when and how often.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum InvestigationAction {
    /// Hybrid search combining semantic and keyword search.
    ///
    /// `semantic_weight` controls the balance:
    /// - 0.0 = pure keyword (good for entities like emails, names)
    /// - 1.0 = pure semantic (good for concepts)
    /// - 0.6 = default balance
    HybridSearch {
        /// The search query
        query: String,
        /// Weight for semantic vs keyword (0.0-1.0)
        semantic_weight: f32,
        /// Maximum results to return
        limit: usize,
    },

    /// Fetch a specific URL directly.
    ///
    /// Use when the gap query mentions a specific page.
    FetchUrl {
        /// URL to fetch
        url: String,
    },

    /// Crawl a site looking for specific content.
    ///
    /// Use for deep gaps that require exploring beyond indexed pages.
    CrawlSite {
        /// Site URL to crawl
        site_url: String,
        /// Query to guide crawling
        query: String,
        /// Maximum pages to crawl
        max_pages: usize,
    },

    /// Search external sources (e.g., Tavily).
    ///
    /// Use when indexed content is exhausted.
    ExternalSearch {
        /// Search query
        query: String,
        /// Number of results
        num_results: usize,
    },
}

impl InvestigationAction {
    /// Create a hybrid search action with default parameters.
    pub fn hybrid_search(query: impl Into<String>) -> Self {
        Self::HybridSearch {
            query: query.into(),
            semantic_weight: 0.6,
            limit: 10,
        }
    }

    /// Create a hybrid search optimized for entity queries (FTS-heavy).
    ///
    /// Use for: emails, phone numbers, names, dates
    pub fn entity_search(query: impl Into<String>) -> Self {
        Self::HybridSearch {
            query: query.into(),
            semantic_weight: 0.3, // FTS-heavy for specific terms
            limit: 10,
        }
    }

    /// Create a hybrid search optimized for semantic queries.
    ///
    /// Use for: concepts, descriptions, abstract queries
    pub fn semantic_search(query: impl Into<String>) -> Self {
        Self::HybridSearch {
            query: query.into(),
            semantic_weight: 0.8, // Semantic-heavy for concepts
            limit: 10,
        }
    }

    /// Create a URL fetch action.
    pub fn fetch_url(url: impl Into<String>) -> Self {
        Self::FetchUrl { url: url.into() }
    }

    /// Create a site crawl action.
    pub fn crawl_site(site_url: impl Into<String>, query: impl Into<String>) -> Self {
        Self::CrawlSite {
            site_url: site_url.into(),
            query: query.into(),
            max_pages: 10,
        }
    }

    /// Create an external search action.
    pub fn external_search(query: impl Into<String>) -> Self {
        Self::ExternalSearch {
            query: query.into(),
            num_results: 5,
        }
    }

    /// Get the action type as a string (for logging).
    pub fn action_type(&self) -> &'static str {
        match self {
            InvestigationAction::HybridSearch { .. } => "hybrid_search",
            InvestigationAction::FetchUrl { .. } => "fetch_url",
            InvestigationAction::CrawlSite { .. } => "crawl_site",
            InvestigationAction::ExternalSearch { .. } => "external_search",
        }
    }
}

/// A plan containing suggested investigation steps.
///
/// The library generates plans; the caller executes them (or not).
/// This separation allows the caller to:
/// - Filter steps based on policy (skip expensive actions)
/// - Prioritize steps (entity gaps before semantic gaps)
/// - Track attempts per gap (ghost gap prevention)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InvestigationPlan {
    /// Suggested steps to resolve gaps
    pub steps: Vec<InvestigationStep>,
}

impl InvestigationPlan {
    /// Create an empty plan.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a step to the plan.
    pub fn add_step(&mut self, step: InvestigationStep) {
        self.steps.push(step);
    }

    /// Add a step (builder pattern).
    pub fn with_step(mut self, step: InvestigationStep) -> Self {
        self.steps.push(step);
        self
    }

    /// Check if the plan is empty.
    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    /// Get the number of steps.
    pub fn len(&self) -> usize {
        self.steps.len()
    }

    /// Iterate over steps.
    pub fn iter(&self) -> impl Iterator<Item = &InvestigationStep> {
        self.steps.iter()
    }

    /// Get steps for a specific gap.
    pub fn steps_for_gap(&self, gap_id: Uuid) -> impl Iterator<Item = &InvestigationStep> {
        self.steps.iter().filter(move |s| s.gap_id == gap_id)
    }

    /// Get steps by action type.
    pub fn steps_by_action<'a>(&'a self, action_type: &'a str) -> impl Iterator<Item = &'a InvestigationStep> {
        self.steps
            .iter()
            .filter(move |s| s.recommended_action.action_type() == action_type)
    }
}

/// Result of executing an investigation step.
///
/// Pure observation of what happened - no policy implications.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResult {
    /// The step that was executed
    pub step: InvestigationStep,

    /// URLs of pages found (empty if none)
    pub pages_found: Vec<String>,

    /// Whether the step found potentially useful content
    pub found_content: bool,

    /// Tokens used (if applicable)
    pub tokens_used: Option<usize>,

    /// Execution duration in milliseconds
    pub duration_ms: Option<u64>,

    /// Error message if the step failed
    pub error: Option<String>,
}

impl StepResult {
    /// Create a successful result with pages.
    pub fn success(step: InvestigationStep, pages: Vec<String>) -> Self {
        let found_content = !pages.is_empty();
        Self {
            step,
            pages_found: pages,
            found_content,
            tokens_used: None,
            duration_ms: None,
            error: None,
        }
    }

    /// Create a failed result.
    pub fn failure(step: InvestigationStep, error: impl Into<String>) -> Self {
        Self {
            step,
            pages_found: Vec::new(),
            found_content: false,
            tokens_used: None,
            duration_ms: None,
            error: Some(error.into()),
        }
    }

    /// Set tokens used.
    pub fn with_tokens(mut self, tokens: usize) -> Self {
        self.tokens_used = Some(tokens);
        self
    }

    /// Set duration.
    pub fn with_duration(mut self, duration_ms: u64) -> Self {
        self.duration_ms = Some(duration_ms);
        self
    }

    /// Check if the step was successful.
    pub fn is_success(&self) -> bool {
        self.error.is_none()
    }

    /// Check if the step found content.
    pub fn has_content(&self) -> bool {
        self.found_content && !self.pages_found.is_empty()
    }
}

/// Classification of gap type for search tuning.
///
/// Different gap types benefit from different search strategies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GapType {
    /// Entity gap: specific named things (emails, names, dates).
    /// Best resolved with FTS-heavy search.
    Entity,

    /// Semantic gap: concepts, descriptions, relationships.
    /// Best resolved with semantic-heavy search.
    Semantic,

    /// Structural gap: missing sections, incomplete data.
    /// May require crawling new pages.
    Structural,
}

impl GapType {
    /// Get the recommended semantic weight for this gap type.
    pub fn recommended_semantic_weight(&self) -> f32 {
        match self {
            GapType::Entity => 0.3,     // FTS-heavy
            GapType::Semantic => 0.7,   // Semantic-heavy
            GapType::Structural => 0.5, // Balanced
        }
    }

    /// Classify a gap query heuristically.
    pub fn classify(query: &str) -> Self {
        let lower = query.to_lowercase();

        // Entity patterns
        if lower.contains("email")
            || lower.contains("phone")
            || lower.contains("address")
            || lower.contains("name of")
            || lower.contains("contact")
            || lower.contains('@')
            || lower.chars().any(|c| c.is_numeric())
        {
            return GapType::Entity;
        }

        // Structural patterns
        if lower.contains("section")
            || lower.contains("page")
            || lower.contains("missing")
            || lower.contains("incomplete")
        {
            return GapType::Structural;
        }

        // Default to semantic
        GapType::Semantic
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_investigation_step_creation() {
        let gap_id = Uuid::new_v4();
        let step = InvestigationStep::new(
            gap_id,
            "contact email",
            "the volunteer coordinator email",
            InvestigationAction::entity_search("volunteer coordinator email"),
        )
        .with_rationale("Entity query, using FTS-heavy search");

        assert_eq!(step.gap_id, gap_id);
        assert_eq!(step.field, "contact email");
        assert!(step.rationale.is_some());
    }

    #[test]
    fn test_action_types() {
        assert_eq!(
            InvestigationAction::hybrid_search("test").action_type(),
            "hybrid_search"
        );
        assert_eq!(
            InvestigationAction::fetch_url("http://example.com").action_type(),
            "fetch_url"
        );
    }

    #[test]
    fn test_gap_type_classification() {
        assert_eq!(
            GapType::classify("the contact email for volunteers"),
            GapType::Entity
        );
        assert_eq!(
            GapType::classify("what services do they offer"),
            GapType::Semantic
        );
        assert_eq!(
            GapType::classify("phone number: 555-1234"),
            GapType::Entity
        );
    }

    #[test]
    fn test_plan_filtering() {
        let gap1 = Uuid::new_v4();
        let gap2 = Uuid::new_v4();

        let plan = InvestigationPlan::new()
            .with_step(InvestigationStep::new(
                gap1,
                "email",
                "email query",
                InvestigationAction::entity_search("email"),
            ))
            .with_step(InvestigationStep::new(
                gap2,
                "services",
                "services query",
                InvestigationAction::semantic_search("services"),
            ))
            .with_step(InvestigationStep::new(
                gap1,
                "email",
                "email query 2",
                InvestigationAction::fetch_url("http://contact.example.com"),
            ));

        assert_eq!(plan.len(), 3);
        assert_eq!(plan.steps_for_gap(gap1).count(), 2);
        assert_eq!(plan.steps_by_action("hybrid_search").count(), 2);
        assert_eq!(plan.steps_by_action("fetch_url").count(), 1);
    }

    #[test]
    fn test_step_result() {
        let step = InvestigationStep::new(
            Uuid::new_v4(),
            "test",
            "test query",
            InvestigationAction::hybrid_search("test"),
        );

        let success = StepResult::success(step.clone(), vec!["http://a.com".to_string()])
            .with_duration(50)
            .with_tokens(100);

        assert!(success.is_success());
        assert!(success.has_content());
        assert_eq!(success.duration_ms, Some(50));

        let failure = StepResult::failure(step, "Connection timeout");
        assert!(!failure.is_success());
        assert!(!failure.has_content());
    }
}
