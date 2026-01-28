use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use clerk_rs::{clerk::Clerk, ClerkConfiguration};
use std::sync::Arc;

/// Authenticated user information from Clerk
#[derive(Clone, Debug)]
pub struct AuthUser {
    pub user_id: String,
    pub is_admin: bool,
}

/// Extension key for storing authenticated user
#[derive(Clone, Debug)]
pub struct AuthContext(pub Option<AuthUser>);

/// Middleware to verify Clerk JWT and extract user info
///
/// This middleware:
/// 1. Extracts JWT from Authorization header
/// 2. Verifies JWT with Clerk
/// 3. Checks user metadata for admin role
/// 4. Stores AuthContext in request extensions
///
/// Note: This middleware does NOT block requests - it only extracts auth info.
/// Authorization checks happen in GraphQL resolvers.
pub async fn clerk_auth_middleware(
    clerk: Arc<Clerk>,
    mut request: Request,
    next: Next,
) -> Response {
    let auth_context = extract_auth_user(&request, &clerk).await;
    
    // Store auth context in request extensions
    request.extensions_mut().insert(AuthContext(auth_context));
    
    next.run(request).await
}

/// Extract and verify auth user from request
async fn extract_auth_user(request: &Request, clerk: &Clerk) -> Option<AuthUser> {
    // Extract JWT from Authorization header
    let auth_header = request.headers().get("authorization")?;
    let auth_str = auth_header.to_str().ok()?;
    
    // Remove "Bearer " prefix
    let token = auth_str.strip_prefix("Bearer ").unwrap_or(auth_str);
    
    // Verify JWT with Clerk
    let session = clerk.verify_session_token(token).await.ok()?;
    
    // Get user metadata to check admin role
    let user = clerk.users().get_user(&session.user_id).await.ok()?;
    
    // Check if user has admin role in public metadata
    let is_admin = user
        .public_metadata
        .as_ref()
        .and_then(|metadata| metadata.get("role"))
        .and_then(|role| role.as_str())
        .map(|role| role == "admin")
        .unwrap_or(false);
    
    Some(AuthUser {
        user_id: session.user_id,
        is_admin,
    })
}

/// Helper function to create auth middleware layer
pub fn clerk_auth_layer(clerk_secret_key: String) -> Arc<Clerk> {
    let config = ClerkConfiguration::new(None, None, Some(clerk_secret_key), None);
    Arc::new(Clerk::new(config))
}

/// Error response for unauthorized requests
pub fn unauthorized_response() -> Response {
    (
        StatusCode::UNAUTHORIZED,
        "Unauthorized: Admin access required",
    )
        .into_response()
}

/// Error response for unauthenticated requests
pub fn unauthenticated_response() -> Response {
    (
        StatusCode::UNAUTHORIZED,
        "Unauthenticated: Valid JWT required",
    )
        .into_response()
}
