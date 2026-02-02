use std::sync::Arc;

use seesaw_core::Engine;
use sqlx::PgPool;
use twilio::TwilioService;

use crate::common::AppState;
use crate::domains::auth::JwtService;
use crate::kernel::{OpenAIClient, ServerDeps};
use crate::server::middleware::AuthUser;

/// The seesaw Engine type used by this application
///
/// In seesaw 0.6.0:
/// - State type (AppState) comes first
/// - Deps type (ServerDeps) comes second
/// - Engine is immutable after construction
/// - Use engine.activate(initial_state) per request
pub type AppEngine = Engine<AppState, ServerDeps>;

/// GraphQL request context
///
/// Contains shared resources available to all resolvers
#[derive(Clone)]
pub struct GraphQLContext {
    pub db_pool: PgPool,
    pub engine: Arc<AppEngine>,
    pub auth_user: Option<AuthUser>,
    pub twilio: Arc<TwilioService>,
    pub jwt_service: Arc<JwtService>,
    pub openai_client: Arc<OpenAIClient>,
}

impl juniper::Context for GraphQLContext {}

impl GraphQLContext {
    pub fn new(
        db_pool: PgPool,
        engine: Arc<AppEngine>,
        auth_user: Option<AuthUser>,
        twilio: Arc<TwilioService>,
        jwt_service: Arc<JwtService>,
        openai_client: Arc<OpenAIClient>,
    ) -> Self {
        Self {
            db_pool,
            engine,
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
}
