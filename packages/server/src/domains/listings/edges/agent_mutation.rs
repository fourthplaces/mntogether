// Agent management mutations

use crate::domains::scraping::effects::agent_config_generator::{
    generate_agent_config, AgentConfigRequest, AgentConfigSuggestion,
};
use crate::server::graphql::context::GraphQLContext;
use juniper::{FieldError, FieldResult, GraphQLObject};

#[derive(GraphQLObject, Clone)]
pub struct GenerateAgentConfigResult {
    pub name: String,
    pub query_template: String,
    pub extraction_instructions: String,
    pub system_prompt: String,
}

impl From<AgentConfigSuggestion> for GenerateAgentConfigResult {
    fn from(suggestion: AgentConfigSuggestion) -> Self {
        Self {
            name: suggestion.name,
            query_template: suggestion.query_template,
            extraction_instructions: suggestion.extraction_instructions,
            system_prompt: suggestion.system_prompt,
        }
    }
}

/// Generate agent configuration from natural language description
/// Uses AI to convert user intent into technical configuration
pub async fn generate_agent_config_from_description(
    ctx: &GraphQLContext,
    description: String,
    location_context: String,
) -> FieldResult<GenerateAgentConfigResult> {
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

    let request = AgentConfigRequest {
        description,
        location_context,
    };

    let suggestion = generate_agent_config(&*ctx.openai_client, request)
        .await
        .map_err(|e| {
            FieldError::new(
                format!("Failed to generate agent config: {}", e),
                juniper::Value::null(),
            )
        })?;

    Ok(suggestion.into())
}
