/// Authorization module for MN Digital Aid
///
/// Provides a fluent API for authorization checks in effect code:
///
/// ```rust
/// use crate::common::auth::{Actor, AdminCapability};
///
/// // In an effect:
/// Actor::new(actor_id)
///     .can(AdminCapability::ManageNeeds)
///     .check(ctx.deps())
///     .await?;
/// ```
///
/// This pattern keeps authorization logic in the effect layer where it belongs,
/// not in the GraphQL resolver layer.

mod errors;
mod capability;
mod builder;

pub use errors::AuthError;
pub use capability::AdminCapability;
pub use builder::{Actor, CapabilityBuilder, HasAuthContext};
