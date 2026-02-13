//! The Index - main entry point for the extraction library.
//!
//! The Index is a flat index over all ingested pages. Sites are metadata
//! for filtering, not structural units.
//!
//! # Detective Engine Support
//!
//! The Index provides **mechanical** investigation primitives:
//! - `plan_investigation()` - Suggests steps to resolve gaps
//! - `execute_step()` - Executes a single investigation step
//!
//! **Policy** (token budgets, iteration limits, ghost gap prevention) belongs
//! in the caller's orchestrator. See `examples/detective_orchestrator.rs`.

use async_stream::stream;
use futures::{future::join_all, Stream};
use std::pin::Pin;
use tokio_util::sync::CancellationToken;
use tracing::{debug, info};
use uuid::Uuid;

use crate::error::{ExtractionError, Result};
use crate::traits::{
    ai::{ExtractionStrategy, AI},
    store::{cosine_similarity, KeywordSearch, PageStore},
};
use crate::types::{
    config::{ExtractionConfig, QueryFilter},
    extraction::Extraction,
    investigation::{
        GapType, InvestigationAction, InvestigationPlan, InvestigationStep, StepResult,
    },
    page::{CachedPage, PageRef},
    summary::Summary,
};

/// The main entry point - a flat index over all ingested pages.
///
/// # Example
///
/// ```rust,ignore
/// let index = Index::new(store, ai);
///
/// // Ingest sites
/// index.ingest("https://redcross.org").await?;
///
/// // Query across all sites
/// let results = index.extract("volunteer opportunities", None).await?;
///
/// // Query one site
/// let filter = QueryFilter::for_site("redcross.org");
/// let results = index.extract("volunteer opportunities", Some(filter)).await?;
/// ```
pub struct Index<S: PageStore + KeywordSearch, A: AI> {
    store: S,
    ai: A,
    config: ExtractionConfig,
}

impl<S: PageStore + KeywordSearch, A: AI> Index<S, A> {
    /// Create a new index.
    pub fn new(store: S, ai: A) -> Self {
        Self {
            store,
            ai,
            config: ExtractionConfig::default(),
        }
    }

    /// Create with custom configuration.
    pub fn with_config(store: S, ai: A, config: ExtractionConfig) -> Self {
        Self { store, ai, config }
    }

    /// Get a reference to the configuration.
    pub fn config(&self) -> &ExtractionConfig {
        &self.config
    }

    /// Get a mutable reference to the configuration.
    pub fn config_mut(&mut self) -> &mut ExtractionConfig {
        &mut self.config
    }

    // =========================================================================
    // Agent-Native Primitives
    // =========================================================================

    /// PRIMITIVE: Search the index.
    ///
    /// Returns page references sorted by relevance.
    pub async fn search(
        &self,
        query: &str,
        limit: usize,
        filter: Option<&QueryFilter>,
    ) -> Result<Vec<PageRef>> {
        let query_embedding = self.ai.embed(query).await?;
        self.store
            .search_similar(&query_embedding, limit, filter)
            .await
    }

    /// PRIMITIVE: Read specific pages.
    ///
    /// Returns full page content for the given URLs.
    pub async fn read(&self, urls: &[&str]) -> Result<Vec<CachedPage>> {
        self.store.get_pages(urls).await
    }

    /// PRIMITIVE: Extract from specific pages (skip recall).
    ///
    /// Useful for agents that want to control which pages to extract from.
    pub async fn extract_from(&self, query: &str, pages: &[CachedPage]) -> Result<Extraction> {
        let hints = if self.config.hints.is_empty() {
            None
        } else {
            Some(self.config.hints.as_slice())
        };

        self.ai.extract(query, pages, hints).await
    }

    /// PRIMITIVE: Search for a specific gap.
    ///
    /// Keyword-heavy search for filling in missing information.
    pub async fn search_for_gap(
        &self,
        gap_query: &str,
        limit: usize,
        filter: Option<&QueryFilter>,
    ) -> Result<Vec<PageRef>> {
        // For gap queries, weight keyword search higher
        let query_embedding = self.ai.embed(gap_query).await?;
        self.store
            .search_similar(&query_embedding, limit, filter)
            .await
    }

    // =========================================================================
    // High-Level API
    // =========================================================================

    /// HIGH-LEVEL: Full extraction pipeline.
    ///
    /// Automatically selects strategy based on query type.
    pub async fn extract(
        &self,
        query: &str,
        filter: Option<QueryFilter>,
    ) -> Result<Vec<Extraction>> {
        let strategy = self.classify_query(query).await;
        debug!(query = %query, strategy = ?strategy, "Extraction strategy");

        match strategy {
            ExtractionStrategy::Collection => self.extract_collection(query, filter.as_ref()).await,
            ExtractionStrategy::Singular => {
                let extraction = self.extract_singular(query, filter.as_ref()).await?;
                Ok(vec![extraction])
            }
            ExtractionStrategy::Narrative => {
                let extraction = self.extract_narrative(query, filter.as_ref()).await?;
                Ok(vec![extraction])
            }
        }
    }

    /// Extract with cancellation support.
    pub async fn extract_with_cancel(
        &self,
        query: &str,
        filter: Option<QueryFilter>,
        cancel: CancellationToken,
    ) -> Result<Vec<Extraction>> {
        tokio::select! {
            result = self.extract(query, filter) => result,
            _ = cancel.cancelled() => Err(ExtractionError::Cancelled),
        }
    }

    /// Return a stream of extractions (for Collection queries).
    ///
    /// Yields extractions as each bucket is processed.
    pub fn extract_stream(
        &self,
        query: &str,
        filter: Option<QueryFilter>,
    ) -> Pin<Box<dyn Stream<Item = Result<Extraction>> + Send + '_>> {
        let query = query.to_string();
        Box::pin(stream! {
            // Get partitions
            let partitions = match self.recall_and_partition(&query, filter.as_ref()).await {
                Ok(p) => p,
                Err(e) => {
                    yield Err(e);
                    return;
                }
            };

            // Process each partition
            for partition in partitions {
                let urls: Vec<&str> = partition.urls.iter().map(|s| s.as_str()).collect();
                let pages = match self.store.get_pages(&urls).await {
                    Ok(p) => p,
                    Err(e) => {
                        yield Err(e);
                        continue;
                    }
                };

                let hints = if self.config.hints.is_empty() {
                    None
                } else {
                    Some(self.config.hints.as_slice())
                };

                match self.ai.extract(&query, &pages, hints).await {
                    Ok(extraction) => yield Ok(extraction),
                    Err(e) => yield Err(e),
                }
            }
        })
    }

    // =========================================================================
    // Strategy-Specific Methods
    // =========================================================================

    /// Classify query intent using heuristics + LLM fallback.
    async fn classify_query(&self, query: &str) -> ExtractionStrategy {
        let query_lower = query.to_lowercase();

        // Heuristics first (cheap, fast)
        if query_lower.starts_with("find all")
            || query_lower.starts_with("list ")
            || query_lower.contains("list of")
            || query_lower.contains("opportunities")
            || query_lower.contains("services")
            || query_lower.contains("programs")
        {
            return ExtractionStrategy::Collection;
        }

        if query_lower.starts_with("what is the")
            || query_lower.starts_with("what's the")
            || query_lower.contains("phone")
            || query_lower.contains("email")
            || query_lower.contains("address")
            || query_lower.contains("contact")
        {
            return ExtractionStrategy::Singular;
        }

        if query_lower.starts_with("summarize")
            || query_lower.starts_with("describe")
            || query_lower.starts_with("what does")
            || query_lower.contains("overview")
            || query_lower.contains("about")
        {
            return ExtractionStrategy::Narrative;
        }

        // Fall back to LLM classification
        self.ai
            .classify_query(query)
            .await
            .unwrap_or(ExtractionStrategy::Collection)
    }

    /// Collection strategy: Recall → Partition → Extract each bucket (in parallel).
    async fn extract_collection(
        &self,
        query: &str,
        filter: Option<&QueryFilter>,
    ) -> Result<Vec<Extraction>> {
        let partitions = self.recall_and_partition(query, filter).await?;

        // Process all partitions in parallel
        let futures = partitions.into_iter().map(|partition| async move {
            let urls: Vec<&str> = partition.urls.iter().map(|s| s.as_str()).collect();
            let pages = self.store.get_pages(&urls).await?;

            let hints = if self.config.hints.is_empty() {
                None
            } else {
                Some(self.config.hints.as_slice())
            };

            self.ai.extract(query, &pages, hints).await
        });

        let results: Vec<Result<Extraction>> = join_all(futures).await;

        // Collect successes, log errors
        let mut extractions = Vec::with_capacity(results.len());
        for result in results {
            match result {
                Ok(extraction) => extractions.push(extraction),
                Err(e) => {
                    tracing::warn!(error = %e, "Partition extraction failed");
                }
            }
        }

        info!(
            partitions = extractions.len(),
            "Collection extraction complete"
        );
        Ok(extractions)
    }

    /// Singular strategy: Recall → Extract single answer.
    async fn extract_singular(
        &self,
        query: &str,
        filter: Option<&QueryFilter>,
    ) -> Result<Extraction> {
        let pages = self.recall_pages(query, 10, filter).await?;
        self.ai.extract_single(query, &pages).await
    }

    /// Narrative strategy: Recall → Summarize → Generate narrative.
    async fn extract_narrative(
        &self,
        query: &str,
        filter: Option<&QueryFilter>,
    ) -> Result<Extraction> {
        let pages = self.recall_pages(query, 20, filter).await?;
        self.ai.extract_narrative(query, &pages).await
    }

    // =========================================================================
    // Recall & Partition
    // =========================================================================

    /// Recall and partition in one step.
    async fn recall_and_partition(
        &self,
        query: &str,
        filter: Option<&QueryFilter>,
    ) -> Result<Vec<crate::traits::ai::Partition>> {
        let summaries = self.ranked_recall(query, filter).await?;

        if summaries.is_empty() {
            return Ok(vec![]);
        }

        self.ai.recall_and_partition(query, &summaries).await
    }

    /// Ranked recall for large sites.
    ///
    /// Uses embedding search to get top N summaries before LLM partitioning.
    async fn ranked_recall(
        &self,
        query: &str,
        filter: Option<&QueryFilter>,
    ) -> Result<Vec<Summary>> {
        let all_summaries = self.store.get_summaries(filter).await?;

        if all_summaries.len() <= self.config.max_summaries_for_partition {
            return Ok(all_summaries);
        }

        // Rank by embedding similarity
        let query_embedding = self.ai.embed(query).await?;

        let mut scored: Vec<_> = all_summaries
            .into_iter()
            .filter_map(|s| {
                let emb = s.embedding.as_ref()?;
                let score = cosine_similarity(&query_embedding, emb);
                Some((score, s))
            })
            .collect();

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        Ok(scored
            .into_iter()
            .take(self.config.max_summaries_for_partition)
            .map(|(_, s)| s)
            .collect())
    }

    /// Recall pages (for Singular/Narrative strategies).
    async fn recall_pages(
        &self,
        query: &str,
        limit: usize,
        filter: Option<&QueryFilter>,
    ) -> Result<Vec<CachedPage>> {
        let page_refs = self.search(query, limit, filter).await?;
        let urls: Vec<&str> = page_refs.iter().map(|p| p.url.as_str()).collect();
        self.store.get_pages(&urls).await
    }

    // =========================================================================
    // Detective Engine: Investigation Planning (Mechanism, Not Policy)
    // =========================================================================

    /// MECHANICAL: Analyze gaps and suggest investigation steps.
    ///
    /// The library provides **intelligence** to investigate; the caller owns
    /// the **will** to investigate (loop control, token budget, retry limits).
    ///
    /// # Returns
    /// An `InvestigationPlan` with suggested steps. The caller decides:
    /// - Which steps to execute
    /// - How many attempts per gap
    /// - When to give up
    ///
    /// # Example
    /// ```rust,ignore
    /// let extraction = index.extract("board members", None).await?;
    ///
    /// if extraction.has_gaps() {
    ///     let plan = index.plan_investigation(&extraction);
    ///     println!("Suggested {} investigation steps", plan.len());
    ///
    ///     for step in &plan.steps {
    ///         let pages = index.execute_step(step, None).await?;
    ///         if !pages.is_empty() {
    ///             let supplement = index.extract_from("board members", &pages).await?;
    ///             extraction.merge(supplement);
    ///         }
    ///     }
    /// }
    /// ```
    pub fn plan_investigation(&self, extraction: &Extraction) -> InvestigationPlan {
        let mut plan = InvestigationPlan::new();

        for gap in &extraction.gaps {
            let gap_id = Uuid::new_v4(); // Caller may override with tracked ID
            let gap_type = GapType::classify(&gap.query);

            let action = match gap_type {
                GapType::Entity => {
                    // Entity queries benefit from FTS-heavy search (emails, names, etc.)
                    InvestigationAction::HybridSearch {
                        query: gap.query.clone(),
                        semantic_weight: gap_type.recommended_semantic_weight(),
                        limit: 10,
                    }
                }
                GapType::Semantic => {
                    // Concept queries benefit from semantic-heavy search
                    InvestigationAction::HybridSearch {
                        query: gap.query.clone(),
                        semantic_weight: gap_type.recommended_semantic_weight(),
                        limit: 10,
                    }
                }
                GapType::Structural => {
                    // Structural gaps might need broader search
                    InvestigationAction::HybridSearch {
                        query: gap.query.clone(),
                        semantic_weight: gap_type.recommended_semantic_weight(),
                        limit: 15,
                    }
                }
            };

            let step = InvestigationStep::new(gap_id, &gap.field, &gap.query, action)
                .with_rationale(format!(
                    "{:?} gap, using {:.0}% semantic / {:.0}% keyword",
                    gap_type,
                    gap_type.recommended_semantic_weight() * 100.0,
                    (1.0 - gap_type.recommended_semantic_weight()) * 100.0
                ));

            plan.add_step(step);
        }

        plan
    }

    /// MECHANICAL: Execute a single investigation step.
    ///
    /// Returns pages found (may be empty). The caller decides what to do with them.
    ///
    /// # Arguments
    /// * `step` - The investigation step to execute
    /// * `filter` - Optional site filter (e.g., limit to same site as original extraction)
    ///
    /// # Returns
    /// `StepResult` containing pages found and execution metadata.
    pub async fn execute_step(
        &self,
        step: &InvestigationStep,
        filter: Option<&QueryFilter>,
    ) -> Result<StepResult> {
        let start = std::time::Instant::now();

        let page_refs = match &step.recommended_action {
            InvestigationAction::HybridSearch {
                query,
                semantic_weight,
                limit,
            } => {
                // Use hybrid search if we have the capability
                self.hybrid_search_for_gap(query, *limit, filter, *semantic_weight)
                    .await?
            }
            InvestigationAction::FetchUrl { url } => {
                // Direct fetch - check if page is in cache
                if let Some(page) = self.store.get_page(url).await? {
                    vec![PageRef {
                        url: page.url,
                        title: page.title,
                        site_url: page.site_url,
                        score: 1.0,
                    }]
                } else {
                    vec![]
                }
            }
            InvestigationAction::CrawlSite { query, .. } => {
                // For crawl actions, just do a search (actual crawling is caller's responsibility)
                self.search(query, 10, filter).await?
            }
            InvestigationAction::ExternalSearch { query, num_results } => {
                // External search not directly supported in Index - return empty
                // Caller should use InformedCrawler or TavilyCrawler for this
                tracing::debug!(
                    query = query,
                    num_results = num_results,
                    "External search requested but not executed by Index"
                );
                vec![]
            }
        };

        let duration = start.elapsed();
        let urls: Vec<String> = page_refs.iter().map(|p| p.url.clone()).collect();

        Ok(StepResult::success(step.clone(), urls).with_duration(duration.as_millis() as u64))
    }

    /// Hybrid search optimized for gap resolution.
    ///
    /// Combines semantic and keyword search with configurable weight.
    async fn hybrid_search_for_gap(
        &self,
        query: &str,
        limit: usize,
        filter: Option<&QueryFilter>,
        semantic_weight: f32,
    ) -> Result<Vec<PageRef>> {
        // Get semantic results
        let query_embedding = self.ai.embed(query).await?;
        let semantic_results = self
            .store
            .search_similar(&query_embedding, limit * 2, filter)
            .await?;

        // Get keyword results
        let keyword_results = self.store.keyword_search(query, limit * 2, filter).await?;

        // Combine with RRF-style fusion
        let k = 60.0; // Standard RRF constant
        let mut combined: std::collections::HashMap<String, (PageRef, f32)> =
            std::collections::HashMap::new();

        // Score semantic results
        for (rank, page_ref) in semantic_results.into_iter().enumerate() {
            let rrf_score = semantic_weight / (k + rank as f32 + 1.0);
            combined
                .entry(page_ref.url.clone())
                .and_modify(|(_, score)| *score += rrf_score)
                .or_insert((page_ref, rrf_score));
        }

        // Score keyword results
        let keyword_weight = 1.0 - semantic_weight;
        for (rank, page_ref) in keyword_results.into_iter().enumerate() {
            let rrf_score = keyword_weight / (k + rank as f32 + 1.0);
            combined
                .entry(page_ref.url.clone())
                .and_modify(|(_, score)| *score += rrf_score)
                .or_insert((page_ref, rrf_score));
        }

        // Sort by combined score
        let mut results: Vec<_> = combined
            .into_values()
            .map(|(mut page_ref, score)| {
                page_ref.score = score;
                page_ref
            })
            .collect();

        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        results.truncate(limit);
        Ok(results)
    }

    /// Get pages from step result URLs.
    ///
    /// Convenience method to fetch full page content after `execute_step()`.
    pub async fn pages_from_step_result(&self, result: &StepResult) -> Result<Vec<CachedPage>> {
        if result.pages_found.is_empty() {
            return Ok(vec![]);
        }

        let urls: Vec<&str> = result.pages_found.iter().map(|s| s.as_str()).collect();
        self.store.get_pages(&urls).await
    }

    /// Get a reference to the store.
    pub fn store(&self) -> &S {
        &self.store
    }

    /// Get a reference to the AI.
    pub fn ai(&self) -> &A {
        &self.ai
    }

    // =========================================================================
    // Ingestor-based Ingestion (new pattern)
    // =========================================================================

    /// Ingest pages from a URL using the provided ingestor.
    ///
    /// This is the main entry point for adding content to the index using
    /// the new pluggable Ingestor pattern.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use extraction::ingestors::{HttpIngestor, ValidatedIngestor, DiscoverConfig};
    ///
    /// let ingestor = ValidatedIngestor::new(HttpIngestor::new());
    /// let config = DiscoverConfig::new("https://example.com").with_limit(10);
    ///
    /// let result = index.ingest(&config, &ingestor).await?;
    /// println!("Ingested {} pages", result.pages_summarized);
    /// ```
    pub async fn ingest<I: crate::traits::ingestor::Ingestor>(
        &self,
        discover_config: &crate::traits::ingestor::DiscoverConfig,
        ingestor: &I,
    ) -> Result<crate::pipeline::ingest::IngestResult> {
        let config = crate::pipeline::ingest::IngestorConfig::default();
        crate::pipeline::ingest::ingest_with_ingestor(
            discover_config,
            &config,
            &self.store,
            &self.ai,
            ingestor,
        )
        .await
    }

    /// Ingest pages with custom configuration.
    pub async fn ingest_with_config<I: crate::traits::ingestor::Ingestor>(
        &self,
        discover_config: &crate::traits::ingestor::DiscoverConfig,
        ingest_config: &crate::pipeline::ingest::IngestorConfig,
        ingestor: &I,
    ) -> Result<crate::pipeline::ingest::IngestResult> {
        crate::pipeline::ingest::ingest_with_ingestor(
            discover_config,
            ingest_config,
            &self.store,
            &self.ai,
            ingestor,
        )
        .await
    }

    /// Fetch and ingest specific URLs (for gap-filling).
    ///
    /// Used by the Detective to follow GapQuery suggestions.
    pub async fn ingest_urls<I: crate::traits::ingestor::Ingestor>(
        &self,
        urls: &[String],
        ingestor: &I,
    ) -> Result<crate::pipeline::ingest::IngestResult> {
        let config = crate::pipeline::ingest::IngestorConfig::default();
        crate::pipeline::ingest::ingest_urls_with_ingestor(
            urls,
            &config,
            &self.store,
            &self.ai,
            ingestor,
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::extraction::GapQuery;

    #[test]
    fn test_query_classification_heuristics() {
        // These are just the heuristic patterns - actual classification
        // would require a mock AI
        let collection_queries = [
            "find all volunteer opportunities",
            "list of services",
            "programs available",
        ];
        let singular_queries = [
            "what is the phone number",
            "email address",
            "contact information",
        ];
        let narrative_queries = [
            "summarize this organization",
            "describe their mission",
            "what does this nonprofit do",
        ];

        for q in collection_queries {
            let lower = q.to_lowercase();
            let is_collection = lower.starts_with("find all")
                || lower.contains("list of")
                || lower.contains("opportunities")
                || lower.contains("services")
                || lower.contains("programs");
            assert!(is_collection, "Expected collection: {}", q);
        }

        for q in singular_queries {
            let lower = q.to_lowercase();
            let is_singular = lower.starts_with("what is the")
                || lower.contains("phone")
                || lower.contains("email")
                || lower.contains("contact");
            assert!(is_singular, "Expected singular: {}", q);
        }

        for q in narrative_queries {
            let lower = q.to_lowercase();
            let is_narrative = lower.starts_with("summarize")
                || lower.starts_with("describe")
                || lower.starts_with("what does");
            assert!(is_narrative, "Expected narrative: {}", q);
        }
    }

    #[test]
    fn test_plan_investigation_entity_gap() {
        use crate::stores::MemoryStore;
        use crate::testing::MockAI;

        let store = MemoryStore::new();
        let ai = MockAI::new();
        let index = Index::new(store, ai);

        let mut extraction = Extraction::new("Some content".to_string());
        extraction.gaps.push(GapQuery::new(
            "contact email",
            "the volunteer coordinator email address",
        ));

        let plan = index.plan_investigation(&extraction);

        assert_eq!(plan.len(), 1);
        let step = &plan.steps[0];
        assert_eq!(step.field, "contact email");

        // Entity gaps should get lower semantic weight (FTS-heavy)
        if let InvestigationAction::HybridSearch {
            semantic_weight, ..
        } = &step.recommended_action
        {
            assert!(
                *semantic_weight < 0.5,
                "Entity gap should use FTS-heavy search"
            );
        } else {
            panic!("Expected HybridSearch action");
        }
    }

    #[test]
    fn test_plan_investigation_semantic_gap() {
        use crate::stores::MemoryStore;
        use crate::testing::MockAI;

        let store = MemoryStore::new();
        let ai = MockAI::new();
        let index = Index::new(store, ai);

        let mut extraction = Extraction::new("Some content".to_string());
        extraction.gaps.push(GapQuery::new(
            "services offered",
            "what services and programs do they provide",
        ));

        let plan = index.plan_investigation(&extraction);

        assert_eq!(plan.len(), 1);
        let step = &plan.steps[0];

        // Semantic gaps should get higher semantic weight
        if let InvestigationAction::HybridSearch {
            semantic_weight, ..
        } = &step.recommended_action
        {
            assert!(
                *semantic_weight > 0.5,
                "Semantic gap should use semantic-heavy search"
            );
        } else {
            panic!("Expected HybridSearch action");
        }
    }

    #[test]
    fn test_plan_investigation_multiple_gaps() {
        use crate::stores::MemoryStore;
        use crate::testing::MockAI;

        let store = MemoryStore::new();
        let ai = MockAI::new();
        let index = Index::new(store, ai);

        let mut extraction = Extraction::new("Some content".to_string());
        extraction
            .gaps
            .push(GapQuery::new("email", "contact@example.com email address"));
        extraction
            .gaps
            .push(GapQuery::new("mission", "what is their mission statement"));
        extraction.gaps.push(GapQuery::new(
            "board",
            "the board of directors section is missing",
        ));

        let plan = index.plan_investigation(&extraction);

        assert_eq!(plan.len(), 3);

        // Verify different gap types get different weights
        let weights: Vec<f32> = plan
            .steps
            .iter()
            .filter_map(|s| {
                if let InvestigationAction::HybridSearch {
                    semantic_weight, ..
                } = &s.recommended_action
                {
                    Some(*semantic_weight)
                } else {
                    None
                }
            })
            .collect();

        // Should have at least 2 distinct weights
        let unique_weights: std::collections::HashSet<_> =
            weights.iter().map(|w| (*w * 10.0) as i32).collect();
        assert!(
            unique_weights.len() >= 2,
            "Different gap types should get different weights"
        );
    }
}
