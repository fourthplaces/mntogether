mod builder;
mod capability;
/// Authorization module for MN Digital Aid
///
/// Provides a fluent API for authorization checks in effect code:
///
/// ```rust
/// use crate::common::auth::{Actor, AdminCapability};
///
/// // In an effect:
/// Actor::new(actor_id, is_admin)
///     .can(AdminCapability::ManageNeeds)
///     .check(ctx.deps())
///     .await?;
/// ```
///
/// The `is_admin` flag comes from the JWT token, which was validated during
/// OTP verification by checking the phone number against admin_identifiers.
///
/// This pattern keeps authorization logic in the effect layer where it belongs,
/// not in the API handler layer.
mod errors;
pub mod restate_auth;

pub use builder::{Actor, CapabilityBuilder, HasAuthContext};
pub use capability::AdminCapability;
pub use errors::AuthError;
