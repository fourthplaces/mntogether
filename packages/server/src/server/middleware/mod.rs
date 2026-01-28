// HTTP middleware
pub mod ip_extractor;
pub mod jwt_auth;
// pub mod session_auth; // REMOVED: Replaced with jwt_auth (Phase 5)

pub use ip_extractor::*;
pub use jwt_auth::*;
// pub use session_auth::*; // REMOVED: Replaced with jwt_auth (Phase 5)
