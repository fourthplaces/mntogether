use axum::extract::FromRef;
use axum::http::request::Parts;

use crate::common::{ApiKeyId, MemberId};
use crate::domains::auth::JwtService;
use crate::domains::posts::models::ApiKey;

use super::error::ApiError;
use super::state::AppState;

/// Authenticated user extracted from request headers.
#[derive(Clone, Debug)]
pub struct AuthUser {
    pub user_id: String,
    pub member_id: MemberId,
    pub phone_number: String,
    pub is_admin: bool,
}

/// Extract JWT from `X-User-Token` or `Authorization` header.
fn extract_token(parts: &Parts) -> Option<String> {
    let value = parts
        .headers
        .get("x-user-token")
        .or_else(|| parts.headers.get("authorization"))
        .and_then(|v| v.to_str().ok())?;

    let token = value.strip_prefix("Bearer ").unwrap_or(value);
    Some(token.to_string())
}

/// Extract the raw Authorization Bearer token (separate from `extract_token`
/// which also honours `X-User-Token` for the JWT flow). Service clients
/// authenticate via Bearer only — no fallback header — so the ingest
/// endpoint can't be invoked with a user token masquerading as an API key.
fn extract_bearer_token(parts: &Parts) -> Option<String> {
    parts
        .headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(|t| t.to_string())
}

fn validate_token(token: &str, jwt_service: &JwtService) -> Result<AuthUser, ApiError> {
    let claims = jwt_service
        .verify_token(token)
        .map_err(|e| ApiError::Unauthorized(format!("Invalid token: {}", e)))?;

    Ok(AuthUser {
        user_id: claims.member_id.to_string(),
        member_id: MemberId::from_uuid(claims.member_id),
        phone_number: claims.phone_number,
        is_admin: claims.is_admin,
    })
}

// ---------------------------------------------------------------------------
// Axum extractors
// ---------------------------------------------------------------------------

/// Requires a valid JWT. Rejects with 401 if missing or invalid.
pub struct AuthenticatedUser(pub AuthUser);

impl<S> axum::extract::FromRequestParts<S> for AuthenticatedUser
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = ApiError;

    fn from_request_parts(
        parts: &mut Parts,
        state: &S,
    ) -> impl std::future::Future<Output = Result<Self, Self::Rejection>> + Send {
        async move {
            let app_state = AppState::from_ref(state);
            let token = extract_token(parts)
                .ok_or_else(|| ApiError::Unauthorized("Missing authentication header".into()))?;
            let user = validate_token(&token, &app_state.deps.jwt_service)?;
            Ok(AuthenticatedUser(user))
        }
    }
}

/// Requires a valid JWT AND admin privileges. Rejects with 401/403.
pub struct AdminUser(pub AuthUser);

impl<S> axum::extract::FromRequestParts<S> for AdminUser
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = ApiError;

    fn from_request_parts(
        parts: &mut Parts,
        state: &S,
    ) -> impl std::future::Future<Output = Result<Self, Self::Rejection>> + Send {
        async move {
            let app_state = AppState::from_ref(state);
            let token = extract_token(parts)
                .ok_or_else(|| ApiError::Unauthorized("Missing authentication header".into()))?;
            let user = validate_token(&token, &app_state.deps.jwt_service)?;
            if !user.is_admin {
                return Err(ApiError::Forbidden("Admin access required".into()));
            }
            Ok(AdminUser(user))
        }
    }
}

/// Optionally extracts auth. Never rejects — returns `None` on missing/invalid token.
pub struct OptionalUser(pub Option<AuthUser>);

impl<S> axum::extract::FromRequestParts<S> for OptionalUser
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    fn from_request_parts(
        parts: &mut Parts,
        state: &S,
    ) -> impl std::future::Future<Output = Result<Self, Self::Rejection>> + Send {
        async move {
            let app_state = AppState::from_ref(state);
            let user = extract_token(parts)
                .and_then(|token| validate_token(&token, &app_state.deps.jwt_service).ok());
            Ok(OptionalUser(user))
        }
    }
}

// ---------------------------------------------------------------------------
// Service-client (Root Signal) extractor — spec §14.1.
// ---------------------------------------------------------------------------

/// A machine client authenticated via `Authorization: Bearer rsk_{env}_<body>`.
#[derive(Clone, Debug)]
pub struct ServiceClient {
    pub id: ApiKeyId,
    pub client_name: String,
    pub scopes: Vec<String>,
}

impl ServiceClient {
    pub fn has_scope(&self, scope: &str) -> bool {
        self.scopes.iter().any(|s| s == scope)
    }
}

/// Validates a Bearer token against `api_keys`. Rejects unauthenticated
/// requests with a terse 401 that does not reveal why the key failed —
/// prevents scope/enumeration probing (spec §14.1).
///
/// Call sites should also gate on `.has_scope("posts:create")` (or whichever
/// scope applies); scope failures raise 403 after the key is otherwise valid.
pub struct ServiceClientAuth(pub ServiceClient);

impl<S> axum::extract::FromRequestParts<S> for ServiceClientAuth
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = ApiError;

    fn from_request_parts(
        parts: &mut Parts,
        state: &S,
    ) -> impl std::future::Future<Output = Result<Self, Self::Rejection>> + Send {
        async move {
            let app_state = AppState::from_ref(state);
            let token = extract_bearer_token(parts)
                .ok_or_else(|| ApiError::Unauthorized("Invalid credentials".into()))?;

            let token_hash = ApiKey::hash_token(&token);
            let key = ApiKey::find_active_by_hash(&token_hash, &app_state.deps.db_pool)
                .await?
                .ok_or_else(|| ApiError::Unauthorized("Invalid credentials".into()))?;

            // Update last_used_at. Best-effort — don't reject the request if
            // this fails (e.g. read-replica in test), just log.
            let pool = app_state.deps.db_pool.clone();
            let key_id = key.id;
            tokio::spawn(async move {
                if let Err(e) = ApiKey::touch_last_used(key_id, &pool).await {
                    tracing::warn!(error = %e, api_key_id = %key_id.as_uuid(), "failed to update last_used_at");
                }
            });

            Ok(ServiceClientAuth(ServiceClient {
                id: key.id,
                client_name: key.client_name,
                scopes: key.scopes,
            }))
        }
    }
}
