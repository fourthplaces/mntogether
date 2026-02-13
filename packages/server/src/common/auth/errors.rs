use thiserror::Error;

/// Authorization errors for the MN Digital Aid platform
#[derive(Error, Debug)]
pub enum AuthError {
    #[error("Authentication required")]
    AuthenticationRequired,

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Admin access required")]
    AdminRequired,

    #[error("Invalid or expired token")]
    InvalidToken,

    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),

    #[error("Internal error: {0}")]
    InternalError(#[from] anyhow::Error),
}
