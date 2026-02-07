//! Register member workflow
//!
//! Durable workflow that orchestrates member registration:
//! 1. Register member in DB (with geocoding)
//! 2. Generate embedding (non-fatal if fails)

use restate_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domains::member::activities;
use crate::impl_restate_serde;
use crate::kernel::ServerDeps;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterMemberRequest {
    pub expo_push_token: String,
    pub searchable_text: String,
    pub city: String,
    pub state: String,
}

impl_restate_serde!(RegisterMemberRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterMemberResult {
    pub member_id: Uuid,
    pub embedding_generated: bool,
}

impl_restate_serde!(RegisterMemberResult);

#[restate_sdk::workflow]
pub trait RegisterMemberWorkflow {
    async fn run(request: RegisterMemberRequest) -> Result<RegisterMemberResult, HandlerError>;
}

pub struct RegisterMemberWorkflowImpl {
    deps: std::sync::Arc<ServerDeps>,
}

impl RegisterMemberWorkflowImpl {
    pub fn with_deps(deps: std::sync::Arc<ServerDeps>) -> Self {
        Self { deps }
    }
}

impl RegisterMemberWorkflow for RegisterMemberWorkflowImpl {
    async fn run(
        &self,
        ctx: WorkflowContext<'_>,
        request: RegisterMemberRequest,
    ) -> Result<RegisterMemberResult, HandlerError> {
        tracing::info!(
            expo_push_token = %request.expo_push_token,
            city = %request.city,
            state = %request.state,
            "Starting register member workflow"
        );

        // Single durable block: register + generate embedding
        let result = ctx
            .run(|| async {
                // Step 1: Register member in DB
                let member_id = activities::register_member(
                    request.expo_push_token.clone(),
                    request.searchable_text.clone(),
                    request.city.clone(),
                    request.state.clone(),
                    &self.deps,
                )
                .await?;

                // Step 2: Generate embedding (non-fatal)
                let embedding_generated = match activities::generate_embedding(
                    member_id,
                    self.deps.embedding_service.as_ref(),
                    &self.deps.db_pool,
                )
                .await
                {
                    Ok(result) => {
                        tracing::info!(
                            member_id = %result.member_id,
                            dimensions = result.dimensions,
                            "Embedding generated for member"
                        );
                        true
                    }
                    Err(e) => {
                        tracing::warn!(
                            member_id = %member_id,
                            error = %e,
                            "Failed to generate embedding (non-fatal)"
                        );
                        false
                    }
                };

                Ok(RegisterMemberResult {
                    member_id,
                    embedding_generated,
                })
            })
            .await?;

        Ok(result)
    }
}
