//! Detective Orchestrator - Reference Implementation
//!
//! This example demonstrates how to build a recursive "Detective Engine" that
//! auto-resolves gaps in extractions. It's a **reference implementation** showing
//! how to combine the library's mechanical primitives with your own policy decisions.
//!
//! # Mechanism vs Policy
//!
//! The extraction library provides **mechanism** (how to investigate):
//! - `plan_investigation()` - Suggests steps to resolve gaps
//! - `execute_step()` - Executes a single investigation step
//! - `Extraction::merge()` - Combines new information
//!
//! This orchestrator adds **policy** (when and how much to investigate):
//! - Token budget limits
//! - Maximum iteration count
//! - Ghost gap prevention (don't retry failed gaps forever)
//! - Convergence detection
//!
//! # Usage
//!
//! Copy this file and customize for your use case:
//! - Adjust `max_iterations` and `token_budget` for your cost tolerance
//! - Add custom gap filtering (e.g., skip certain gap types)
//! - Add logging/observability integration
//! - Integrate with your rate limiting infrastructure
//!
//! ```bash
//! cargo run --example detective_orchestrator --features postgres
//! ```

use std::collections::HashMap;

use extraction::{
    Extraction, Index,
    PageStore, AI,
};
use extraction::traits::store::KeywordSearch;

/// Configuration for the Detective orchestrator.
#[derive(Debug, Clone)]
pub struct DetectiveConfig {
    /// Maximum number of investigation iterations.
    /// Each iteration processes all pending gaps once.
    pub max_iterations: usize,

    /// Maximum tokens to spend on investigation.
    /// Set to 0 for unlimited (not recommended in production).
    pub token_budget: usize,

    /// Maximum attempts per gap before giving up.
    /// Prevents "ghost gaps" that can never be resolved.
    pub max_gap_attempts: usize,

    /// Minimum pages required to consider a step successful.
    pub min_pages_for_success: usize,
}

impl Default for DetectiveConfig {
    fn default() -> Self {
        Self {
            max_iterations: 3,
            token_budget: 10_000,
            max_gap_attempts: 3,
            min_pages_for_success: 1,
        }
    }
}

/// Tracks the state of an ongoing investigation.
#[derive(Debug)]
pub struct InvestigationState {
    /// Total tokens used so far
    pub tokens_used: usize,

    /// Completed iteration count
    pub iterations: usize,

    /// Gap attempts: field -> attempt count
    pub gap_attempts: HashMap<String, usize>,

    /// Gaps that have been abandoned (exceeded max_gap_attempts)
    pub abandoned_gaps: Vec<String>,

    /// Whether the investigation has converged (no new information)
    pub converged: bool,
}

impl InvestigationState {
    pub fn new() -> Self {
        Self {
            tokens_used: 0,
            iterations: 0,
            gap_attempts: HashMap::new(),
            abandoned_gaps: Vec::new(),
            converged: false,
        }
    }

    /// Check if we should continue investigating.
    pub fn should_continue(&self, config: &DetectiveConfig) -> bool {
        !self.converged
            && self.iterations < config.max_iterations
            && (config.token_budget == 0 || self.tokens_used < config.token_budget)
    }

    /// Record an attempt for a gap.
    /// Returns true if we should skip this gap (exceeded attempts).
    pub fn record_attempt(&mut self, field: &str, config: &DetectiveConfig) -> bool {
        let attempts = self.gap_attempts.entry(field.to_string()).or_insert(0);
        *attempts += 1;

        if *attempts > config.max_gap_attempts {
            if !self.abandoned_gaps.contains(&field.to_string()) {
                self.abandoned_gaps.push(field.to_string());
            }
            true // Skip this gap
        } else {
            false
        }
    }
}

/// The main Detective orchestrator.
///
/// This is where **policy** meets **mechanism**.
pub struct DetectiveOrchestrator<'a, S: PageStore + KeywordSearch, A: AI> {
    index: &'a Index<S, A>,
    config: DetectiveConfig,
}

impl<'a, S: PageStore + KeywordSearch, A: AI> DetectiveOrchestrator<'a, S, A> {
    /// Create a new orchestrator with default config.
    pub fn new(index: &'a Index<S, A>) -> Self {
        Self {
            index,
            config: DetectiveConfig::default(),
        }
    }

    /// Create with custom configuration.
    pub fn with_config(index: &'a Index<S, A>, config: DetectiveConfig) -> Self {
        Self { index, config }
    }

    /// Run the full Detective extraction with automatic gap resolution.
    ///
    /// # Arguments
    /// * `query` - The extraction query
    /// * `filter` - Optional site filter
    ///
    /// # Returns
    /// The final extraction with as many gaps resolved as possible within budget.
    pub async fn extract(
        &self,
        query: &str,
        filter: Option<extraction::QueryFilter>,
    ) -> extraction::error::Result<(Extraction, InvestigationState)> {
        // Initial extraction
        let extractions = self.index.extract(query, filter.clone()).await?;
        let mut extraction = Extraction::combine(extractions);

        let mut state = InvestigationState::new();

        // Investigation loop (POLICY: max_iterations controls this)
        while extraction.has_gaps() && state.should_continue(&self.config) {
            state.iterations += 1;
            let initial_gap_count = extraction.gaps.len();

            // Get investigation plan (MECHANISM)
            let plan = self.index.plan_investigation(&extraction);

            if plan.is_empty() {
                state.converged = true;
                break;
            }

            // Execute each step (with POLICY filters)
            let mut any_progress = false;
            for step in &plan.steps {
                // POLICY: Ghost gap prevention
                if state.record_attempt(&step.field, &self.config) {
                    tracing::debug!(
                        field = %step.field,
                        "Skipping gap - exceeded max attempts"
                    );
                    continue;
                }

                // POLICY: Token budget check
                if self.config.token_budget > 0 && state.tokens_used >= self.config.token_budget {
                    tracing::debug!(
                        tokens_used = state.tokens_used,
                        budget = self.config.token_budget,
                        "Stopping - token budget exhausted"
                    );
                    break;
                }

                // MECHANISM: Execute the step
                let result = self
                    .index
                    .execute_step(step, filter.as_ref())
                    .await?;

                // POLICY: Check if step was successful enough
                if result.pages_found.len() >= self.config.min_pages_for_success {
                    // Fetch full pages and extract
                    let pages = self.index.pages_from_step_result(&result).await?;

                    if !pages.is_empty() {
                        let supplement = self.index.extract_from(query, &pages).await?;

                        // Track tokens (estimate based on content size)
                        let estimated_tokens = pages.iter().map(|p| p.content.len() / 4).sum::<usize>();
                        state.tokens_used += estimated_tokens;

                        // MECHANISM: Merge the new information
                        let old_gaps = extraction.gaps.len();
                        extraction.merge(supplement);
                        let new_gaps = extraction.gaps.len();

                        if new_gaps < old_gaps {
                            any_progress = true;
                            tracing::debug!(
                                field = %step.field,
                                resolved = old_gaps - new_gaps,
                                "Gap partially resolved"
                            );
                        }
                    }
                }
            }

            // POLICY: Convergence detection
            if !any_progress && extraction.gaps.len() >= initial_gap_count {
                tracing::debug!("No progress made in iteration, marking as converged");
                state.converged = true;
            }
        }

        Ok((extraction, state))
    }
}

/// Example usage showing the Detective orchestrator in action.
#[cfg(feature = "postgres")]
async fn example_usage() -> extraction::error::Result<()> {
    use extraction::{PostgresStore, ExtractionConfig};

    // Setup (your AI implementation)
    let store = PostgresStore::new("postgres://localhost/extraction").await?;
    // let ai = YourAIImplementation::new();
    // let index = Index::new(store, ai);

    // Configure the Detective
    let config = DetectiveConfig {
        max_iterations: 5,      // More iterations for complex queries
        token_budget: 50_000,   // Higher budget for important extractions
        max_gap_attempts: 3,    // Standard ghost gap prevention
        min_pages_for_success: 1,
    };

    // let detective = DetectiveOrchestrator::with_config(&index, config);

    // Run extraction with automatic gap resolution
    // let (extraction, state) = detective.extract(
    //     "Find all board members with their contact information",
    //     None,
    // ).await?;

    // println!("Final grounding: {:?}", extraction.grounding);
    // println!("Remaining gaps: {}", extraction.gaps.len());
    // println!("Tokens used: {}", state.tokens_used);
    // println!("Iterations: {}", state.iterations);
    // println!("Abandoned gaps: {:?}", state.abandoned_gaps);

    Ok(())
}

fn main() {
    println!("Detective Orchestrator - Reference Implementation");
    println!();
    println!("This is a reference implementation showing how to build a recursive");
    println!("Detective Engine on top of the extraction library's primitives.");
    println!();
    println!("Key concepts:");
    println!("  - MECHANISM (library): plan_investigation(), execute_step(), merge()");
    println!("  - POLICY (this code): token_budget, max_iterations, ghost_gap_prevention");
    println!();
    println!("Copy and customize this code for your use case!");
    println!();
    println!("Example:");
    println!("  let detective = DetectiveOrchestrator::with_config(&index, config);");
    println!("  let (extraction, state) = detective.extract(query, None).await?;");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = DetectiveConfig::default();
        assert_eq!(config.max_iterations, 3);
        assert_eq!(config.token_budget, 10_000);
        assert_eq!(config.max_gap_attempts, 3);
    }

    #[test]
    fn test_investigation_state_should_continue() {
        let config = DetectiveConfig {
            max_iterations: 3,
            token_budget: 1000,
            ..Default::default()
        };

        let mut state = InvestigationState::new();

        // Should continue initially
        assert!(state.should_continue(&config));

        // Should stop after max iterations
        state.iterations = 3;
        assert!(!state.should_continue(&config));
        state.iterations = 0;

        // Should stop after budget exhausted
        state.tokens_used = 1000;
        assert!(!state.should_continue(&config));
        state.tokens_used = 0;

        // Should stop when converged
        state.converged = true;
        assert!(!state.should_continue(&config));
    }

    #[test]
    fn test_ghost_gap_prevention() {
        let config = DetectiveConfig {
            max_gap_attempts: 2,
            ..Default::default()
        };

        let mut state = InvestigationState::new();

        // First two attempts should proceed
        assert!(!state.record_attempt("email", &config));
        assert!(!state.record_attempt("email", &config));

        // Third attempt should be skipped
        assert!(state.record_attempt("email", &config));

        // Gap should be marked as abandoned
        assert!(state.abandoned_gaps.contains(&"email".to_string()));

        // Different gap should still work
        assert!(!state.record_attempt("phone", &config));
    }
}
