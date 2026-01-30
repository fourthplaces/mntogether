// Agent queries and mutations

use crate::common::JobId;
use crate::domains::listings::events::ListingEvent;
use crate::server::graphql::context::GraphQLContext;
use juniper::{FieldError, FieldResult, GraphQLObject};
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
