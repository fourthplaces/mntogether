//! Search cascade handler - conducts Tavily research searches
//!
//! This handler is called by the effect when WebsiteResearchCreated is emitted.
//! Cascade flow: WebsiteResearchCreated → handle_conduct_searches → ResearchSearchesCompleted

use crate::common::{AppState, JobId, MemberId, WebsiteId};
use crate::domains::domain_approval::events::DomainApprovalEvent;
use crate::domains::website::models::{TavilySearchQuery, TavilySearchResult, WebsiteResearch};
use crate::kernel::ServerDeps;
use anyhow::{Context, Result};
use seesaw_core::EffectContext;
use tracing::info;
use uuid::Uuid;

// ============================================================================
// Handler Functions (Business Logic) - emit events directly
// ============================================================================

/// Handle the ConductResearchSearchesRequested event.
pub async fn handle_conduct_searches(
    research_id: Uuid,
    website_id: WebsiteId,
    job_id: JobId,
    requested_by: MemberId,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<()> {
    info!(
        research_id = %research_id,
        website_id = %website_id,
        job_id = %job_id,
        "Conducting research searches"
    );

    // Step 1: Load research to get website URL
    let research =
        WebsiteResearch::find_latest_by_website_id(website_id.into(), &ctx.deps().db_pool)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Research not found: {}", research_id))?;

    // Step 2: Extract domain name from URL
    let domain_name = extract_domain_name(&research.homepage_url);

    info!(
        research_id = %research_id,
        domain_name = %domain_name,
        "Extracted domain name"
    );

    // Step 3: Define research queries
    let queries = vec![
        format!("{} organization background mission", domain_name),
        format!("{} reviews complaints problems", domain_name),
        format!("{} founded history about", domain_name),
    ];

    let mut total_results = 0;

    // Step 4: Execute each search and store results
    for query_text in &queries {
        info!(query = %query_text, "Executing Tavily search");

        // Execute search
        let results = ctx
            .deps()
            .web_searcher
            .search_with_limit(query_text, 5)
            .await
            .context(format!("Failed to execute search: {}", query_text))?;

        info!(
            query = %query_text,
            result_count = results.len(),
            "Tavily search completed"
        );

        // Store query record
        let query_record = TavilySearchQuery::create(
            research.id,
            query_text.clone(),
            Some("basic".to_string()),
            Some(5),
            None,
            &ctx.deps().db_pool,
        )
        .await
        .context("Failed to store query record")?;

        // Store results
        if !results.is_empty() {
            let result_tuples: Vec<_> = results
                .into_iter()
                .map(|r| {
                    (
                        r.title.unwrap_or_default(),
                        r.url.to_string(),
                        r.snippet.unwrap_or_default(),
                        r.score.unwrap_or(0.0) as f64,
                        None::<String>, // published_date not available in extraction SearchResult
                    )
                })
                .collect();

            total_results += result_tuples.len();

            TavilySearchResult::create_batch(query_record.id, result_tuples, &ctx.deps().db_pool)
                .await
                .context("Failed to store search results")?;

            info!(
                query_id = %query_record.id,
                result_count = total_results,
                "Search results stored"
            );
        }
    }

    // Step 5: Mark research as complete
    research
        .mark_tavily_complete(&ctx.deps().db_pool)
        .await
        .context("Failed to mark research complete")?;

    info!(
        research_id = %research_id,
        total_queries = queries.len(),
        total_results = total_results,
        "All research searches completed"
    );

    // Step 6: Emit completion event
    ctx.emit(DomainApprovalEvent::ResearchSearchesCompleted {
        research_id,
        website_id,
        job_id,
        total_queries: queries.len(),
        total_results,
        requested_by,
    });
    Ok(())
}

// ============================================================================
// Helper Functions
// ============================================================================

fn extract_domain_name(url: &str) -> String {
    url.trim_start_matches("http://")
        .trim_start_matches("https://")
        .split('/')
        .next()
        .unwrap_or(url)
        .to_string()
}
