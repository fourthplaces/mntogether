use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

/// Autonomous agent for discovery, scraping, and extraction
///
/// An agent handles the complete pipeline:
/// 1. Search for domains via Tavily (using query_template)
/// 2. Auto-scrape discovered domains via Firecrawl
/// 3. Extract listings via AI (using extraction_instructions)
/// 4. Auto-approve domains when listings are found
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Agent {
    pub id: Uuid,
    pub name: String,
    pub query_template: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub search_frequency_hours: i32,
    pub last_searched_at: Option<DateTime<Utc>>,
    pub location_context: String,
    pub service_area_tags: Vec<String>,
    pub search_depth: String,
    pub max_results: i32,
    pub days_range: i32,
    pub min_relevance_score: f64,

    // Extraction fields
    pub extraction_instructions: Option<String>,
    pub system_prompt: Option<String>,

    // Automation flags
    pub auto_approve_domains: bool,
    pub auto_scrape: bool,
    pub auto_create_listings: bool,

    // Statistics
    pub total_searches_run: i32,
    pub total_domains_discovered: i32,
    pub total_domains_approved: i32,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: Option<Uuid>,
}

impl Agent {
    /// Find all agents that are due for searching
    pub async fn find_due_for_searching(pool: &PgPool) -> Result<Vec<Self>> {
        let agents = sqlx::query_as::<_, Agent>(
            r#"
            SELECT * FROM agents
            WHERE enabled = true
              AND (last_searched_at IS NULL
                   OR last_searched_at < NOW() - (search_frequency_hours || ' hours')::INTERVAL)
            ORDER BY last_searched_at NULLS FIRST
            "#
        )
        .fetch_all(pool)
        .await?;

        Ok(agents)
    }

    /// Find all enabled agents
    pub async fn find_all_enabled(pool: &PgPool) -> Result<Vec<Self>> {
        let agents = sqlx::query_as::<_, Agent>(
            r#"
            SELECT * FROM agents
            WHERE enabled = true
            ORDER BY name
            "#
        )
        .fetch_all(pool)
        .await?;

        Ok(agents)
    }

    /// Find an agent by ID
    pub async fn find_by_id(id: Uuid, pool: &PgPool) -> Result<Self> {
        let agent = sqlx::query_as::<_, Agent>(
            r#"
            SELECT * FROM agents WHERE id = $1
            "#
        )
        .bind(id)
        .fetch_one(pool)
        .await?;

        Ok(agent)
    }

    /// Create a new agent
    pub async fn create(
        name: String,
        query_template: String,
        description: Option<String>,
        extraction_instructions: Option<String>,
        system_prompt: Option<String>,
        location_context: String,
        pool: &PgPool,
    ) -> Result<Self> {
        let agent = sqlx::query_as::<_, Agent>(
            r#"
            INSERT INTO agents (
                name,
                query_template,
                description,
                extraction_instructions,
                system_prompt,
                location_context,
                enabled,
                auto_approve_domains,
                auto_scrape,
                auto_create_listings
            )
            VALUES ($1, $2, $3, $4, $5, $6, true, true, true, true)
            RETURNING *
            "#
        )
        .bind(name)
        .bind(query_template)
        .bind(description)
        .bind(extraction_instructions)
        .bind(system_prompt)
        .bind(location_context)
        .fetch_one(pool)
        .await?;

        Ok(agent)
    }

    /// Update agent extraction instructions
    pub async fn update_extraction_instructions(
        id: Uuid,
        extraction_instructions: String,
        system_prompt: Option<String>,
        pool: &PgPool,
    ) -> Result<Self> {
        let agent = sqlx::query_as::<_, Agent>(
            r#"
            UPDATE agents
            SET extraction_instructions = $2,
                system_prompt = $3,
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#
        )
        .bind(id)
        .bind(extraction_instructions)
        .bind(system_prompt)
        .fetch_one(pool)
        .await?;

        Ok(agent)
    }

    /// Update search statistics after a search run
    pub async fn update_stats(
        id: Uuid,
        results_found: usize,
        domains_created: usize,
        pool: &PgPool,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE agents
            SET last_searched_at = NOW(),
                total_searches_run = total_searches_run + 1,
                total_domains_discovered = total_domains_discovered + $2,
                updated_at = NOW()
            WHERE id = $1
            "#
        )
        .bind(id)
        .bind(domains_created as i32)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Increment approved domains count
    pub async fn increment_approved_count(id: Uuid, pool: &PgPool) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE agents
            SET total_domains_approved = total_domains_approved + 1,
                updated_at = NOW()
            WHERE id = $1
            "#
        )
        .bind(id)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Get relevance score as f64 for filtering
    pub fn min_relevance_score_f64(&self) -> f64 {
        self.min_relevance_score
    }

    /// Get extraction instructions (with fallback to default)
    pub fn get_extraction_instructions(&self) -> String {
        self.extraction_instructions.clone().unwrap_or_else(|| {
            "Extract community resources, services, and volunteer opportunities. \
             Include eligibility requirements, contact information, and how to access the service."
                .to_string()
        })
    }

    /// Get system prompt (with fallback to default)
    pub fn get_system_prompt(&self) -> String {
        self.system_prompt.clone().unwrap_or_else(|| {
            "You are an expert at identifying community resources and services. \
             Extract detailed information about programs, eligibility, contact details, \
             and how people can access help."
                .to_string()
        })
    }
}
