use axum::extract::FromRef;
use axum::http::request::Parts;

use crate::common::MemberId;
use crate::domains::auth::JwtService;

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
