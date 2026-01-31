use anyhow::Result;
use async_trait::async_trait;
use seesaw_core::{Effect, EffectContext};
use url::Url;
use uuid::Uuid;

use super::deps::ServerDeps;
use crate::common::JobId;
use crate::domains::listings::commands::ListingCommand;
use crate::domains::listings::events::ListingEvent;
use crate::domains::scraping::models::{Agent, Website};

pub struct SearchEffect;

#[async_trait]
impl Effect<ListingCommand, ServerDeps> for SearchEffect {
    type Event = ListingEvent;

    async fn execute(
        &self,
        cmd: ListingCommand,
        ctx: EffectContext<ServerDeps>,
    ) -> Result<ListingEvent> {
        match cmd {
            ListingCommand::ExecuteSearch { agent_id, job_id } => {
                handle_execute_search(agent_id, job_id, &ctx).await
            }
            _ => anyhow::bail!("SearchEffect: Unexpected command"),
        }
    }
}

async fn handle_execute_search(
    agent_id: Uuid,
    job_id: JobId,
    ctx: &EffectContext<ServerDeps>,
) -> Result<ListingEvent> {
    // 1. Get agent from database
    let agent = match Agent::find_by_id(agent_id, &ctx.deps().db_pool).await {
        Ok(agent) => agent,
        Err(e) => {
            tracing::error!(agent_id = %agent_id, error = %e, "Failed to find agent");
            return Ok(ListingEvent::AgentSearchFailed {
                agent_id,
                job_id,
                reason: format!("Failed to find agent: {}", e),
            });
        }
    };

    tracing::info!(
        agent_name = %agent.name,
        query_template = %agent.query_template,
        auto_scrape = agent.auto_scrape,
        auto_approve = agent.auto_approve_domains,
        "Executing Tavily search for agent"
    );

    // 2. Build query with location
    let query = agent
        .query_template
        .replace("{location}", &agent.location_context);

    // 3. Execute Tavily search
    let results = match ctx
        .deps()
        .search_service
        .search(
            &query,
            Some(agent.max_results as usize),
            Some(&agent.search_depth),
            Some(agent.days_range),
        )
        .await
    {
        Ok(results) => results,
        Err(e) => {
            tracing::error!(
                agent_id = %agent_id,
                agent_name = %agent.name,
                query = %query,
                error = %e,
                "Tavily search failed"
            );
            return Ok(ListingEvent::AgentSearchFailed {
                agent_id,
                job_id,
                reason: format!("Tavily search failed: {}", e),
            });
        }
    };

    tracing::info!(
        results_count = results.len(),
        agent_name = %agent.name,
        "Tavily search completed"
    );

    // 4. Filter by relevance score
    let min_score = agent.min_relevance_score_f64();
    let filtered: Vec<_> = results
        .into_iter()
        .filter(|r| r.score >= min_score)
        .collect();

    tracing::info!(
        filtered_count = filtered.len(),
        min_score = min_score,
        "Filtered search results by relevance score"
    );

    // 5. Create domains (skip duplicates) and auto-scrape if enabled
    let mut domains_created = 0;

    for result in &filtered {
        // Extract domain from URL
        let domain_url = match extract_domain_from_url(&result.url) {
            Ok(url) => url,
            Err(e) => {
                tracing::warn!(
                    url = %result.url,
                    error = %e,
                    "Failed to extract domain from URL, skipping"
                );
                continue;
            }
        };

        // Skip if domain exists
        if Website::find_by_url(&domain_url, &ctx.deps().db_pool)
            .await?
            .is_some()
        {
            tracing::debug!(
                domain_url = %domain_url,
                "Domain already exists, skipping"
            );
            continue;
        }

        // Create pending website
        let website = match Website::create(
            domain_url.clone(),
            None, // No member submitted this
            "system".to_string(),
            Some(format!("Discovered via Tavily agent: {}", agent.name)),
            3, // Default max_crawl_depth
            &ctx.deps().db_pool,
        )
        .await
        {
            Ok(website) => website,
            Err(e) => {
                tracing::error!(
                    domain_url = %domain_url,
                    error = %e,
                    "Failed to create domain, skipping"
                );
                continue;
            }
        };

        // Link to agent and store metadata
        let metadata = serde_json::json!({
            "title": result.title,
            "published_date": result.published_date,
            "search_query": query,
        });

        if let Err(e) = sqlx::query(
            r#"
            UPDATE websites
            SET agent_id = $1,
                tavily_relevance_score = $2,
                tavily_search_metadata = $3
            WHERE id = $4
            "#,
        )
        .bind(agent_id)
        .bind(result.score)
        .bind(metadata)
        .bind(website.id)
        .execute(&ctx.deps().db_pool)
        .await
        {
            tracing::error!(
                website_id = %website.id,
                error = %e,
                "Failed to link website to agent, skipping"
            );
            continue;
        }

        tracing::info!(
            website_id = %website.id,
            domain_url = %domain_url,
            relevance_score = result.score,
            agent_name = %agent.name,
            "Created website from agent search"
        );

        domains_created += 1;

        // Note: Auto-scraping handled by scheduled scraper (runs hourly)
        // Websites discovered by agents are prioritized based on:
        // 1. last_scraped_at IS NULL (never scraped)
        // 2. Agent auto_scrape setting
        // Once listings are extracted and website is auto-approved, it becomes active
    }

    // 6. Update statistics
    if let Err(e) = Agent::update_stats(
        agent_id,
        filtered.len(),
        domains_created,
        &ctx.deps().db_pool,
    )
    .await
    {
        tracing::warn!(
            agent_id = %agent_id,
            error = %e,
            "Failed to update agent stats (non-critical)"
        );
        // Don't fail the entire operation for stats update failure
    }

    tracing::info!(
        agent_name = %agent.name,
        results_found = filtered.len(),
        domains_created = domains_created,
        auto_scraped = agent.auto_scrape && domains_created > 0,
        "Agent search execution completed"
    );

    Ok(ListingEvent::AgentSearchCompleted {
        agent_id,
        job_id,
        results_found: filtered.len(),
        domains_created,
    })
}

/// Extract domain from full URL (e.g., "https://example.com/path" -> "example.com")
fn extract_domain_from_url(url: &str) -> Result<String> {
    let parsed = Url::parse(url).map_err(|e| anyhow::anyhow!("Invalid URL: {}", e))?;

    let host = parsed
        .host_str()
        .ok_or_else(|| anyhow::anyhow!("URL has no host"))?
        .to_string();

    Ok(host)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_domain_from_url() {
        assert_eq!(
            extract_domain_from_url("https://example.com/path/to/page").unwrap(),
            "example.com"
        );
        assert_eq!(
            extract_domain_from_url("http://subdomain.example.org:8080/page?query=1").unwrap(),
            "subdomain.example.org"
        );
        assert!(extract_domain_from_url("not-a-url").is_err());
    }
}
