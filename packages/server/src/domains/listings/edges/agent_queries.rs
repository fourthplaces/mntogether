// Agent queries and mutations

use crate::common::JobId;
use crate::domains::listings::data::agent::AgentData;
use crate::domains::listings::events::ListingEvent;
use crate::domains::scraping::models::Agent;
use crate::server::graphql::context::GraphQLContext;
use juniper::{FieldError, FieldResult, GraphQLInputObject, GraphQLObject};
use uuid::Uuid;

#[derive(GraphQLObject, Clone)]
pub struct TriggerSearchResult {
    pub job_id: String,
    pub status: String,
    pub message: String,
}

/// Trigger an agent search manually (admin only)
/// Dispatches AgentSearchRequested event for immediate execution
pub async fn trigger_agent_search(
    ctx: &GraphQLContext,
    agent_id: String,
) -> FieldResult<TriggerSearchResult> {
    // Verify admin access
    let user = ctx
        .auth_user
        .as_ref()
        .ok_or_else(|| FieldError::new("Authentication required", juniper::Value::null()))?;

    if !user.is_admin {
        return Err(FieldError::new(
            "Admin authorization required",
            juniper::Value::null(),
        ));
    }

    // Parse agent ID
    let agent_uuid = Uuid::parse_str(&agent_id).map_err(|_| {
        FieldError::new("Invalid agent ID format", juniper::Value::null())
    })?;

    // Verify agent exists
    use crate::domains::scraping::models::Agent;
    Agent::find_by_id(agent_uuid, &ctx.db_pool)
        .await
        .map_err(|_| FieldError::new("Agent not found", juniper::Value::null()))?;

    // Generate job ID
    let job_id = JobId::new();

    tracing::info!(
        agent_id = %agent_uuid,
        job_id = %job_id,
        requested_by = %user.member_id,
        "Admin triggering agent search manually"
    );

    // Emit AgentSearchRequested event (fire-and-forget, non-blocking)
    ctx.bus.emit(ListingEvent::AgentSearchRequested {
        agent_id: agent_uuid,
        job_id,
    });

    Ok(TriggerSearchResult {
        job_id: job_id.to_string(),
        status: "queued".to_string(),
        message: format!(
            "Agent search queued successfully. The agent will search for domains via Tavily."
        ),
    })
}

/// Get all agents (admin only)
pub async fn get_all_agents(ctx: &GraphQLContext) -> FieldResult<Vec<AgentData>> {
    // Verify admin access
    let user = ctx
        .auth_user
        .as_ref()
        .ok_or_else(|| FieldError::new("Authentication required", juniper::Value::null()))?;

    if !user.is_admin {
        return Err(FieldError::new(
            "Admin authorization required",
            juniper::Value::null(),
        ));
    }

    // Fetch all agents
    let agents = Agent::find_all(&ctx.db_pool)
        .await
        .map_err(|e| FieldError::new(format!("Failed to fetch agents: {}", e), juniper::Value::null()))?;

    // Convert to GraphQL data type
    Ok(agents.into_iter().map(AgentData::from).collect())
}

#[derive(GraphQLInputObject, Clone)]
pub struct UpdateAgentInput {
    pub name: String,
    pub query_template: String,
    pub description: Option<String>,
    pub extraction_instructions: Option<String>,
    pub system_prompt: Option<String>,
    pub location_context: String,
    pub enabled: bool,
}

/// Update an agent (admin only)
pub async fn update_agent(
    ctx: &GraphQLContext,
    agent_id: String,
    input: UpdateAgentInput,
) -> FieldResult<AgentData> {
    // Verify admin access
    let user = ctx
        .auth_user
        .as_ref()
        .ok_or_else(|| FieldError::new("Authentication required", juniper::Value::null()))?;

    if !user.is_admin {
        return Err(FieldError::new(
            "Admin authorization required",
            juniper::Value::null(),
        ));
    }

    // Parse agent ID
    let agent_uuid = Uuid::parse_str(&agent_id).map_err(|_| {
        FieldError::new("Invalid agent ID format", juniper::Value::null())
    })?;

    tracing::info!(
        agent_id = %agent_uuid,
        updated_by = %user.member_id,
        "Admin updating agent"
    );

    // Update the agent
    let updated_agent = Agent::update(
        agent_uuid,
        input.name,
        input.query_template,
        input.description,
        input.extraction_instructions,
        input.system_prompt,
        input.location_context,
        input.enabled,
        &ctx.db_pool,
    )
    .await
    .map_err(|e| FieldError::new(format!("Failed to update agent: {}", e), juniper::Value::null()))?;

    Ok(AgentData::from(updated_agent))
}
