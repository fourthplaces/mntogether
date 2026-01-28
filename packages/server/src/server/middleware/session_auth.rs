use crate::server::auth::SessionStore;
use axum::{extract::{Request, State}, middleware::Next, response::Response};
use std::sync::Arc;
use uuid::Uuid;

/// Authenticated user information from session
#[derive(Clone, Debug)]
pub struct AuthUser {
    pub user_id: String,
    pub member_id: Uuid,
    pub phone_number: String,
    pub is_admin: bool,
}

/// Middleware to extract session and populate auth user
///
/// This middleware:
/// 1. Extracts session token from Authorization header
/// 2. Looks up session in SessionStore
/// 3. Stores AuthUser in request extensions
///
/// Note: This middleware does NOT block requests - it only extracts auth info.
/// Authorization checks happen in GraphQL resolvers.
pub async fn session_auth_middleware(
    State(session_store): State<Arc<SessionStore>>,
    mut request: Request,
    next: Next,
) -> Response {
    let auth_user = extract_auth_user(&request, session_store.as_ref()).await;

    // Store auth user in request extensions
    if let Some(user) = auth_user {
        request.extensions_mut().insert(user);
    }

    next.run(request).await
}

/// Extract and verify auth user from request
async fn extract_auth_user(request: &Request, session_store: &SessionStore) -> Option<AuthUser> {
    // Extract session token from Authorization header
    let auth_header = request.headers().get("authorization")?;
    let auth_str = auth_header.to_str().ok()?;

    // Remove "Bearer " prefix
    let token = auth_str.strip_prefix("Bearer ").unwrap_or(auth_str);

    // Look up session
    let session = session_store.get_session(token).await?;

    Some(AuthUser {
        user_id: session.member_id.to_string(),
        member_id: session.member_id,
        phone_number: session.phone_number,
        is_admin: session.is_admin,
    })
}
