use crate::common::MemberId;
use crate::domains::auth::JwtService;
use axum::{middleware::Next, response::Response};
use std::sync::Arc;
use tracing::debug;

/// Authenticated user information from JWT
#[derive(Clone, Debug)]
pub struct AuthUser {
    pub user_id: String,
    pub member_id: MemberId,
    pub phone_number: String,
    pub is_admin: bool,
}

/// JWT authentication middleware
///
/// Extracts JWT token from Authorization header, verifies it, and adds AuthUser to request extensions.
/// If no token or invalid token, request continues without AuthUser (public access).
pub async fn jwt_auth_middleware(
    jwt_service: Arc<JwtService>,
    mut request: axum::http::Request<axum::body::Body>,
    next: Next,
) -> Response {
    let auth_user = extract_auth_user(&request, &jwt_service);

    if let Some(user) = auth_user {
        debug!(
            "Authenticated user: {} (admin: {})",
            user.member_id, user.is_admin
        );
        request.extensions_mut().insert(user);
    } else {
        debug!("No valid authentication token");
    }

    next.run(request).await
}

/// Extract and verify JWT token from request
fn extract_auth_user(
    request: &axum::http::Request<axum::body::Body>,
    jwt_service: &JwtService,
) -> Option<AuthUser> {
    // Get Authorization header
    let auth_header = request.headers().get("authorization")?;
    let auth_str = auth_header.to_str().ok()?;

    // Extract token (handle both "Bearer <token>" and raw token)
    let token = auth_str.strip_prefix("Bearer ").unwrap_or(auth_str);

    // Verify token
    let claims = jwt_service.verify_token(token).ok()?;

    Some(AuthUser {
        user_id: claims.member_id.to_string(),
        member_id: MemberId::from_uuid(claims.member_id),
        phone_number: claims.phone_number,
        is_admin: claims.is_admin,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_extract_token_with_bearer() {
        let jwt_service = JwtService::new("test_secret", "test_issuer".to_string());
        let member_id = Uuid::new_v4();
        let token = jwt_service
            .create_token(member_id, "+1234567890".to_string(), true)
            .unwrap();

        let request = axum::http::Request::builder()
            .header("authorization", format!("Bearer {}", token))
            .body(axum::body::Body::empty())
            .unwrap();

        let auth_user = extract_auth_user(&request, &jwt_service);
        assert!(auth_user.is_some());
        assert_eq!(auth_user.unwrap().member_id, MemberId::from_uuid(member_id));
    }

    #[test]
    fn test_extract_token_without_bearer() {
        let jwt_service = JwtService::new("test_secret", "test_issuer".to_string());
        let member_id = Uuid::new_v4();
        let token = jwt_service
            .create_token(member_id, "+1234567890".to_string(), false)
            .unwrap();

        let request = axum::http::Request::builder()
            .header("authorization", token)
            .body(axum::body::Body::empty())
            .unwrap();

        let auth_user = extract_auth_user(&request, &jwt_service);
        assert!(auth_user.is_some());
        assert_eq!(auth_user.unwrap().member_id, MemberId::from_uuid(member_id));
    }

    #[test]
    fn test_no_auth_header() {
        let jwt_service = JwtService::new("test_secret", "test_issuer".to_string());
        let request = axum::http::Request::builder()
            .body(axum::body::Body::empty())
            .unwrap();

        let auth_user = extract_auth_user(&request, &jwt_service);
        assert!(auth_user.is_none());
    }

    #[test]
    fn test_invalid_token() {
        let jwt_service = JwtService::new("test_secret", "test_issuer".to_string());
        let request = axum::http::Request::builder()
            .header("authorization", "Bearer invalid_token")
            .body(axum::body::Body::empty())
            .unwrap();

        let auth_user = extract_auth_user(&request, &jwt_service);
        assert!(auth_user.is_none());
    }
}
