use std::sync::Arc;

use seesaw_core::QueueEngine;
use seesaw_postgres::PostgresStore;
use sqlx::PgPool;
use twilio::TwilioService;

use crate::common::AppState;
use crate::domains::auth::JwtService;
use crate::kernel::{OpenAIClient, ServerDeps};
use crate::server::middleware::AuthUser;

/// The seesaw QueueEngine type used by this application.
///
/// All events (sync + async) go through the queue-backed engine.
/// EventWorkers process events, run reducers, and dispatch to effects.
/// EffectWorkers execute queued effects with retry/timeout.
pub type AppQueueEngine = QueueEngine<AppState, ServerDeps, PostgresStore>;

/// GraphQL request context
///
/// Contains shared resources available to all resolvers.
/// Mutations call actions directly with `ctx.deps()`, then publish
/// fact events via `ctx.queue_engine.process(event)` for cascading effects.
#[derive(Clone)]
pub struct GraphQLContext {
    pub db_pool: PgPool,
    pub queue_engine: Arc<AppQueueEngine>,
    pub server_deps: Arc<ServerDeps>,
    pub auth_user: Option<AuthUser>,
    pub twilio: Arc<TwilioService>,
    pub jwt_service: Arc<JwtService>,
    pub openai_client: Arc<OpenAIClient>,
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
    ) -> Self {
        Self {
            db_pool,
            queue_engine,
            server_deps,
            auth_user,
            twilio,
            jwt_service,
            openai_client,
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
            Some(user) => {
                match uuid::Uuid::parse_str(&user.user_id) {
                    Ok(uuid) => AppState::authenticated(uuid, user.is_admin),
                    Err(_) => AppState::anonymous(),
                }
            }
            None => AppState::anonymous(),
        }
    }

    /// Get server dependencies for direct access
    pub fn deps(&self) -> &ServerDeps {
        &self.server_deps
    }
}
