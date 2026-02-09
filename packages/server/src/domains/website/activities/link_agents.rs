//! Link a website to matching agents using LLM reasoning.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::info;
use uuid::Uuid;

use crate::common::WebsiteId;
use crate::domains::agents::models::{
    Agent, AgentCuratorConfig, AgentSearchQuery, AgentWebsite,
};
use crate::domains::website::models::{Website, WebsiteAssessment};
use crate::kernel::llm_request::LlmRequestExt;
use crate::kernel::ServerDeps;

#[derive(Debug, Clone)]
pub struct LinkedAgentResult {
    pub agent_id: Uuid,
    pub display_name: String,
    pub reason: String,
}

#[derive(Debug, Clone)]
pub struct LinkAgentsResult {
    pub linked: Vec<LinkedAgentResult>,
}

#[derive(Debug, Serialize, Deserialize)]
struct AgentMatch {
    agent_id: String,
    reason: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct AgentMatchResponse {
    matches: Vec<AgentMatch>,
}

/// Use LLM to match a website to the best-fit curator agents and link them.
pub async fn link_website_to_agents(
    website_id: WebsiteId,
    deps: &ServerDeps,
) -> Result<LinkAgentsResult> {
    let pool = &deps.db_pool;
    let website = Website::find_by_id(website_id, pool).await?;
    let assessment = WebsiteAssessment::find_latest_by_website_id(website_id.into_uuid(), pool).await?;

    // Load all active curator agents
    let curators = Agent::find_active_curators(pool).await?;
    if curators.is_empty() {
        info!("No active curator agents found");
        return Ok(LinkAgentsResult { linked: vec![] });
    }

    // Find which agents are already linked
    let existing_links = AgentWebsite::find_by_website(website_id.into_uuid(), pool).await?;
    let already_linked: std::collections::HashSet<Uuid> =
        existing_links.iter().map(|aw| aw.agent_id).collect();

    // Filter to unlinked agents only
    let unlinked_curators: Vec<&Agent> = curators
        .iter()
        .filter(|a| !already_linked.contains(&a.id))
        .collect();

    if unlinked_curators.is_empty() {
        info!(website_id = %website_id, "All curators already linked");
        return Ok(LinkAgentsResult { linked: vec![] });
    }

    // Build agent descriptions for the LLM prompt
    let mut agent_descriptions = Vec::new();
    for agent in &unlinked_curators {
        let config = AgentCuratorConfig::find_by_agent(agent.id, pool).await.ok();
        let queries = AgentSearchQuery::find_by_agent(agent.id, pool).await.unwrap_or_default();

        let purpose = config
            .as_ref()
            .map(|c| c.purpose.as_str())
            .unwrap_or("(no purpose set)");

        let query_list: Vec<&str> = queries.iter().map(|q| q.query_text.as_str()).collect();

        agent_descriptions.push(format!(
            "- Agent ID: {}\n  Name: {}\n  Purpose: {}\n  Search queries: [{}]",
            agent.id,
            agent.display_name,
            purpose,
            query_list.join(", ")
        ));
    }

    let assessment_summary = assessment
        .map(|a| format!("Assessment:\n{}", a.assessment_markdown))
        .unwrap_or_else(|| "No assessment available.".to_string());

    let user_prompt = format!(
        "Website domain: {}\n{}\n\nAvailable agents:\n{}",
        website.domain,
        assessment_summary,
        agent_descriptions.join("\n\n")
    );

    info!(website_id = %website_id, agent_count = unlinked_curators.len(), "Asking LLM to match agents");

    let llm_response: AgentMatchResponse = deps
        .ai
        .request()
        .system(
            r#"You match websites to curator agents for MN Together, a community resource platform in Minnesota.

Given a website (domain + optional assessment) and a list of available curator agents (with their purpose and search queries), determine which agents should be linked to this website.

An agent should be linked if:
- The website's content aligns with the agent's purpose
- The website is the kind of site the agent's search queries would find
- The website serves the same audience the agent targets

Only match agents that are clearly relevant. It's OK to return zero matches.

Respond with ONLY valid JSON, no markdown fences:
{"matches": [{"agent_id": "uuid-here", "reason": "Brief explanation why this agent matches"}]}"#,
        )
        .user(&user_prompt)
        .output::<AgentMatchResponse>()
        .await?;

    // Link matched agents
    let mut linked = Vec::new();
    for m in &llm_response.matches {
        let Ok(agent_id) = Uuid::parse_str(&m.agent_id) else {
            continue;
        };

        // Verify agent exists in our unlinked list
        let Some(agent) = unlinked_curators.iter().find(|a| a.id == agent_id) else {
            continue;
        };

        AgentWebsite::link(agent_id, website_id.into_uuid(), pool).await?;
        info!(website_id = %website_id, agent_id = %agent_id, agent_name = %agent.display_name, "Linked agent to website");

        linked.push(LinkedAgentResult {
            agent_id,
            display_name: agent.display_name.clone(),
            reason: m.reason.clone(),
        });
    }

    info!(
        website_id = %website_id,
        linked_count = linked.len(),
        "Finished linking agents to website"
    );

    Ok(LinkAgentsResult { linked })
}
