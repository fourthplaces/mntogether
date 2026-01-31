use crate::domains::scraping::models::Agent as AgentModel;
use crate::server::graphql::context::GraphQLContext;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentData {
    pub id: String,
    pub name: String,
    pub query_template: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub search_frequency_hours: i32,
    pub last_searched_at: Option<DateTime<Utc>>,
    pub location_context: String,
    pub search_depth: String,
    pub max_results: i32,
    pub days_range: i32,
    pub min_relevance_score: f64,
    pub extraction_instructions: Option<String>,
    pub system_prompt: Option<String>,
    pub auto_approve_websites: bool,
    pub auto_scrape: bool,
    pub auto_create_listings: bool,
    pub total_searches_run: i32,
    pub total_websites_discovered: i32,
    pub total_websites_approved: i32,
    pub created_at: DateTime<Utc>,
}

#[juniper::graphql_object(Context = GraphQLContext)]
impl AgentData {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn query_template(&self) -> &str {
        &self.query_template
    }

    fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    fn enabled(&self) -> bool {
        self.enabled
    }

    fn search_frequency_hours(&self) -> i32 {
        self.search_frequency_hours
    }

    fn last_searched_at(&self) -> Option<DateTime<Utc>> {
        self.last_searched_at
    }

    fn location_context(&self) -> &str {
        &self.location_context
    }

    fn search_depth(&self) -> &str {
        &self.search_depth
    }

    fn max_results(&self) -> i32 {
        self.max_results
    }

    fn days_range(&self) -> i32 {
        self.days_range
    }

    fn min_relevance_score(&self) -> f64 {
        self.min_relevance_score
    }

    fn extraction_instructions(&self) -> Option<&str> {
        self.extraction_instructions.as_deref()
    }

    fn system_prompt(&self) -> Option<&str> {
        self.system_prompt.as_deref()
    }

    fn auto_approve_websites(&self) -> bool {
        self.auto_approve_websites
    }

    fn auto_scrape(&self) -> bool {
        self.auto_scrape
    }

    fn auto_create_listings(&self) -> bool {
        self.auto_create_listings
    }

    fn total_searches_run(&self) -> i32 {
        self.total_searches_run
    }

    fn total_websites_discovered(&self) -> i32 {
        self.total_websites_discovered
    }

    fn total_websites_approved(&self) -> i32 {
        self.total_websites_approved
    }

    fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}

impl From<AgentModel> for AgentData {
    fn from(agent: AgentModel) -> Self {
        Self {
            id: agent.id.to_string(),
            name: agent.name,
            query_template: agent.query_template,
            description: agent.description,
            enabled: agent.enabled,
            search_frequency_hours: agent.search_frequency_hours,
            last_searched_at: agent.last_searched_at,
            location_context: agent.location_context,
            search_depth: agent.search_depth,
            max_results: agent.max_results,
            days_range: agent.days_range,
            min_relevance_score: agent.min_relevance_score,
            extraction_instructions: agent.extraction_instructions,
            system_prompt: agent.system_prompt,
            auto_approve_websites: agent.auto_approve_websites,
            auto_scrape: agent.auto_scrape,
            auto_create_listings: agent.auto_create_listings,
            total_searches_run: agent.total_searches_run,
            total_websites_discovered: agent.total_websites_discovered,
            total_websites_approved: agent.total_websites_approved,
            created_at: agent.created_at,
        }
    }
}
