//! Server functions for authentication
//!
//! These run on the server and handle session management.

use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

use crate::graphql::{GraphQLClient, SEND_VERIFICATION_CODE, VERIFY_CODE};
use crate::types::AuthUser;

/// Send a verification code to the user's phone or email
#[server]
pub async fn send_verification_code(identifier: String) -> Result<bool, ServerFnError> {
    let client = server_graphql_client();

    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Variables {
        phone_number: String,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Response {
        send_verification_code: bool,
    }

    let result: Response = client
        .mutate(SEND_VERIFICATION_CODE, Some(Variables { phone_number: identifier }))
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(result.send_verification_code)
}

/// Verify a code and establish a session
#[server]
pub async fn verify_code(identifier: String, code: String) -> Result<Option<String>, ServerFnError> {
    let client = server_graphql_client();

    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Variables {
        phone_number: String,
        code: String,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Response {
        verify_code: Option<String>,
    }

    let result: Response = client
        .mutate(
            VERIFY_CODE,
            Some(Variables {
                phone_number: identifier,
                code,
            }),
        )
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    // If we got a token, store it in the session
    if let Some(ref token) = result.verify_code {
        // Decode JWT claims and store in session
        if let Ok(user) = decode_jwt_to_user(token) {
            set_session_user(&user).await?;
        }
    }

    Ok(result.verify_code)
}

/// Get the current authenticated user from the session
#[server]
pub async fn get_current_user() -> Result<Option<AuthUser>, ServerFnError> {
    get_session_user().await
}

/// Logout - clear the session
#[server]
pub async fn logout() -> Result<(), ServerFnError> {
    clear_session().await
}

// ============================================================================
// Server-only helpers (not exposed as server functions)
// ============================================================================

#[cfg(feature = "server")]
fn server_graphql_client() -> GraphQLClient {
    let url = std::env::var("API_URL").unwrap_or_else(|_| "http://localhost:8080/graphql".to_string());
    GraphQLClient::new(url)
}

#[cfg(feature = "server")]
fn decode_jwt_to_user(token: &str) -> Result<AuthUser, ServerFnError> {
    // Simple JWT decoding (just base64 decode the payload)
    // In production, you'd want to verify the signature
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return Err(ServerFnError::new("Invalid JWT format"));
    }

    use base64::Engine;
    let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(parts[1])
        .map_err(|e| ServerFnError::new(format!("Failed to decode JWT: {}", e)))?;

    #[derive(Deserialize)]
    struct JwtClaims {
        member_id: uuid::Uuid,
        phone_number: String,
        is_admin: bool,
    }

    let claims: JwtClaims = serde_json::from_slice(&payload)
        .map_err(|e| ServerFnError::new(format!("Failed to parse JWT claims: {}", e)))?;

    Ok(AuthUser {
        member_id: claims.member_id,
        phone_number: claims.phone_number,
        is_admin: claims.is_admin,
    })
}

#[cfg(feature = "server")]
async fn set_session_user(user: &AuthUser) -> Result<(), ServerFnError> {
    use tower_sessions::Session;

    let session: Session = dioxus::fullstack::extract()
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to get session: {}", e)))?;

    session
        .insert("user", user)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to set session: {}", e)))?;

    Ok(())
}

#[cfg(feature = "server")]
async fn get_session_user() -> Result<Option<AuthUser>, ServerFnError> {
    use tower_sessions::Session;

    let session: Session = dioxus::fullstack::extract()
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to get session: {}", e)))?;

    session
        .get("user")
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to get user from session: {}", e)))
}

#[cfg(feature = "server")]
async fn clear_session() -> Result<(), ServerFnError> {
    use tower_sessions::Session;

    let session: Session = dioxus::fullstack::extract()
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to get session: {}", e)))?;

    session
        .flush()
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to clear session: {}", e)))?;

    Ok(())
}
