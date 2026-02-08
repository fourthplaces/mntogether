//! Restate handler authentication helpers
//!
//! Extract and validate JWT from Restate handler request headers.
//! Used by all Restate services, virtual objects, and workflows that require auth.

use crate::common::MemberId;
use crate::domains::auth::JwtService;
use restate_sdk::prelude::*;

/// Authenticated user extracted from Restate handler headers
#[derive(Clone, Debug)]
pub struct AuthUser {
    pub user_id: String,
    pub member_id: MemberId,
    pub phone_number: String,
    pub is_admin: bool,
}

/// Extract and validate JWT from Restate handler headers.
///
/// Returns `Ok(AuthUser)` if a valid Bearer token is present.
/// Returns `Err(TerminalError)` if the token is missing or invalid.
pub fn authenticate(headers: &HeaderMap, jwt_service: &JwtService) -> Result<AuthUser, HandlerError> {
    // Prefer X-User-Token (set by web app when Restate Cloud consumes Authorization header)
    // Fall back to Authorization header for direct calls
    let auth_str = headers
        .get("x-user-token")
        .or_else(|| headers.get("authorization"))
        .ok_or_else(|| TerminalError::new("Missing authentication header"))?;

    let token = auth_str.strip_prefix("Bearer ").unwrap_or(auth_str);

    let claims = jwt_service
        .verify_token(token)
        .map_err(|e| TerminalError::new(format!("Invalid token: {}", e)))?;

    Ok(AuthUser {
        user_id: claims.member_id.to_string(),
        member_id: MemberId::from_uuid(claims.member_id),
        phone_number: claims.phone_number,
        is_admin: claims.is_admin,
    })
}

/// Extract and validate JWT, requiring admin access.
///
/// Returns `Ok(AuthUser)` if a valid Bearer token is present and user is admin.
/// Returns `Err(TerminalError)` otherwise.
pub fn require_admin(
    headers: &HeaderMap,
    jwt_service: &JwtService,
) -> Result<AuthUser, HandlerError> {
    let user = authenticate(headers, jwt_service)?;
    if !user.is_admin {
        return Err(TerminalError::new("Admin access required").into());
    }
    Ok(user)
}

/// Optionally extract auth user from headers.
///
/// Returns `Some(AuthUser)` if a valid token is present, `None` otherwise.
/// Does not error on missing/invalid token (for public endpoints).
pub fn optional_auth(headers: &HeaderMap, jwt_service: &JwtService) -> Option<AuthUser> {
    authenticate(headers, jwt_service).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domains::auth::JwtService;
    use uuid::Uuid;

    fn make_headers(token: Option<&str>) -> HeaderMap {
        let mut map = HeaderMap::default();
        if let Some(t) = token {
            map.insert(
                reqwest::header::AUTHORIZATION,
                format!("Bearer {}", t),
            );
        }
        map
    }

    #[test]
    fn test_authenticate_valid_token() {
        let jwt = JwtService::new("test_secret", "test_issuer".to_string());
        let member_id = Uuid::new_v4();
        let token = jwt
            .create_token(member_id, "+1234567890".to_string(), false)
            .unwrap();

        let headers = make_headers(Some(&token));
        let user = authenticate(&headers, &jwt).unwrap();
        assert_eq!(user.member_id, MemberId::from_uuid(member_id));
        assert!(!user.is_admin);
    }

    #[test]
    fn test_authenticate_missing_header() {
        let jwt = JwtService::new("test_secret", "test_issuer".to_string());
        let headers = make_headers(None);
        assert!(authenticate(&headers, &jwt).is_err());
    }

    #[test]
    fn test_require_admin_non_admin() {
        let jwt = JwtService::new("test_secret", "test_issuer".to_string());
        let member_id = Uuid::new_v4();
        let token = jwt
            .create_token(member_id, "+1234567890".to_string(), false)
            .unwrap();

        let headers = make_headers(Some(&token));
        assert!(require_admin(&headers, &jwt).is_err());
    }

    #[test]
    fn test_require_admin_admin() {
        let jwt = JwtService::new("test_secret", "test_issuer".to_string());
        let member_id = Uuid::new_v4();
        let token = jwt
            .create_token(member_id, "+1234567890".to_string(), true)
            .unwrap();

        let headers = make_headers(Some(&token));
        let user = require_admin(&headers, &jwt).unwrap();
        assert!(user.is_admin);
    }

    #[test]
    fn test_optional_auth_no_token() {
        let jwt = JwtService::new("test_secret", "test_issuer".to_string());
        let headers = make_headers(None);
        assert!(optional_auth(&headers, &jwt).is_none());
    }

    #[test]
    fn test_optional_auth_valid_token() {
        let jwt = JwtService::new("test_secret", "test_issuer".to_string());
        let member_id = Uuid::new_v4();
        let token = jwt
            .create_token(member_id, "+1234567890".to_string(), false)
            .unwrap();

        let headers = make_headers(Some(&token));
        assert!(optional_auth(&headers, &jwt).is_some());
    }
}
