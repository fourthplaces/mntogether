use std::sync::Arc;

use seesaw_core::Engine;
use seesaw_postgres::PostgresStore;
use sqlx::PgPool;
use twilio::TwilioService;

use crate::common::AppState;
use crate::domains::auth::JwtService;
use crate::kernel::{OpenAIClient, ServerDeps};
use crate::server::graphql::loaders::DataLoaders;
use crate::server::middleware::AuthUser;
use crate::WorkflowClient;

/// The Seesaw QueueEngine type (legacy - being removed incrementally)
pub type AppQueueEngine = Engine<AppState, ServerDeps, PostgresStore>;

/// GraphQL request context
///
/// Contains shared resources available to all resolvers.
///
/// MIGRATION IN PROGRESS:
/// - queue_engine: For unmigrated domains (TODO: remove after all migrated)
/// - workflow_client: For migrated domains (Restate workflows)
#[derive(Clone)]
pub struct GraphQLContext {
    pub db_pool: PgPool,
    pub queue_engine: Arc<AppQueueEngine>, // TODO: Remove after all domains migrated
    pub workflow_client: Arc<WorkflowClient>,
    pub server_deps: Arc<ServerDeps>,
    pub auth_user: Option<AuthUser>,
    pub twilio: Arc<TwilioService>,
    pub jwt_service: Arc<JwtService>,
    pub openai_client: Arc<OpenAIClient>,
    pub loaders: Arc<DataLoaders>,
}

impl juniper::Context for GraphQLContext {}

impl GraphQLContext {
    pub fn new(
        db_pool: PgPool,
        queue_engine: Arc<AppQueueEngine>,
        server_deps: Arc<ServerDeps>,
        auth_user: Option<AuthUser>,
        twilio: Arc<TwilioService>,
        jwt_service: Arc<JwtService>,
        openai_client: Arc<OpenAIClient>,
        loaders: Arc<DataLoaders>,
        workflow_client: Arc<WorkflowClient>,
    ) -> Self {
        Self {
            db_pool,
            queue_engine,
            workflow_client,
            server_deps,
            auth_user,
            twilio,
            jwt_service,
            openai_client,
            loaders,
        }
    }

    /// Check if the current user is authenticated
    pub fn is_authenticated(&self) -> bool {
        self.auth_user.is_some()
    }

    /// Check if the current user is an admin
    pub fn is_admin(&self) -> bool {
        self.auth_user
            .as_ref()
            .map(|user| user.is_admin)
            .unwrap_or(false)
    }

    /// Require admin access, return error if not authorized
    pub fn require_admin(&self) -> Result<(), juniper::FieldError> {
        if !self.is_admin() {
            return Err(juniper::FieldError::new(
                "Unauthorized: Admin access required",
                juniper::Value::null(),
            ));
        }
        Ok(())
    }

    /// Get the current user ID, return error if not authenticated
    pub fn require_auth(&self) -> Result<&str, juniper::FieldError> {
        self.auth_user
            .as_ref()
            .map(|user| user.user_id.as_str())
            .ok_or_else(|| {
                juniper::FieldError::new(
                    "Unauthenticated: Valid JWT required",
                    juniper::Value::null(),
                )
            })
    }

    /// Create AppState with visitor info from the current request.
    pub fn app_state(&self) -> AppState {
        match &self.auth_user {
            Some(user) => match uuid::Uuid::parse_str(&user.user_id) {
                Ok(uuid) => AppState::authenticated(uuid, user.is_admin),
                Err(_) => AppState::anonymous(),
            },
            None => AppState::anonymous(),
        }
    }

    /// Get server dependencies for direct access
    pub fn deps(&self) -> &ServerDeps {
        &self.server_deps
    }
}
