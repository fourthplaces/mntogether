use super::{AdminCapability, AuthError};
use crate::common::entity_ids::MemberId;
use anyhow::Result;

/// Entry point for authorization checks
///
/// Usage:
/// ```
/// Actor::new(actor_id, is_admin)
///     .can(AdminCapability::ManageNeeds)
///     .check(ctx.deps())
///     .await?;
/// ```
pub struct Actor {
    actor_id: MemberId,
    is_admin: bool,
}

impl Actor {
    /// Create a new actor for authorization checks
    ///
    /// # Arguments
    /// * `actor_id` - The member ID of the actor
    /// * `is_admin` - Admin flag from JWT/session (already validated during authentication)
    pub fn new(actor_id: MemberId, is_admin: bool) -> Self {
        Self { actor_id, is_admin }
    }

    /// Specify what capability the actor needs
    pub fn can(self, capability: AdminCapability) -> CapabilityBuilder {
        CapabilityBuilder {
            actor_id: self.actor_id,
            is_admin: self.is_admin,
            capability,
        }
    }
}

/// Builder after specifying capability
pub struct CapabilityBuilder {
    actor_id: MemberId,
    is_admin: bool,
    capability: AdminCapability,
}

impl CapabilityBuilder {
    /// Perform the authorization check
    pub async fn check<D>(self, deps: &D) -> Result<(), AuthError>
    where
        D: HasAuthContext,
    {
        check_admin_permission(self.actor_id, self.is_admin, self.capability, deps).await
    }
}

/// Trait for dependencies that can perform auth checks
pub trait HasAuthContext: Send + Sync {
    fn admin_identifiers(&self) -> &[String];
    fn test_identifier_enabled(&self) -> bool;
}

/// Core permission check function
///
/// This function verifies admin capabilities. The `is_admin` flag comes from the JWT token,
/// which was already validated during OTP verification by checking if the phone number
/// is in the admin_identifiers list. We trust this flag since:
/// 1. JWT tokens are cryptographically signed and verified
/// 2. The flag was set during authentication by checking against admin_identifiers
/// 3. Tokens expire after 24 hours, limiting the window for stale permissions
async fn check_admin_permission<D>(
    _actor_id: MemberId,
    is_admin: bool,
    _capability: AdminCapability,
    _deps: &D,
) -> Result<(), AuthError>
where
    D: HasAuthContext,
{
    // In MN Digital Aid, all admin capabilities require admin status.
    // The is_admin flag comes from the JWT and was validated during OTP verification
    // by checking the phone number against the admin_identifiers list.

    if !is_admin {
        return Err(AuthError::AdminRequired);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestDeps {
        admin_identifiers: Vec<String>,
    }

    impl HasAuthContext for TestDeps {
        fn admin_identifiers(&self) -> &[String] {
            &self.admin_identifiers
        }

        fn test_identifier_enabled(&self) -> bool {
            false
        }
    }

    #[tokio::test]
    async fn test_admin_check() {
        let deps = TestDeps {
            admin_identifiers: vec!["admin@example.com".to_string()],
        };

        let actor_id = MemberId::new();
        let result = Actor::new(actor_id, true) // is_admin = true
            .can(AdminCapability::ManageNeeds)
            .check(&deps)
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_non_admin_rejected() {
        let deps = TestDeps {
            admin_identifiers: vec![],
        };

        let actor_id = MemberId::new();
        let result = Actor::new(actor_id, false) // is_admin = false
            .can(AdminCapability::ManageNeeds)
            .check(&deps)
            .await;

        assert!(matches!(result, Err(AuthError::AdminRequired)));
    }
}
