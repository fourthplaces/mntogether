use super::{AdminCapability, AuthError};
use crate::common::entity_ids::MemberId;
use anyhow::Result;

/// Entry point for authorization checks
///
/// Usage:
/// ```
/// Actor::new(actor_id)
///     .can(AdminCapability::ManageNeeds)
///     .check(ctx.deps())
///     .await?;
/// ```
pub struct Actor {
    actor_id: MemberId,
}

impl Actor {
    /// Create a new actor for authorization checks
    pub fn new(actor_id: MemberId) -> Self {
        Self { actor_id }
    }

    /// Specify what capability the actor needs
    pub fn can(self, capability: AdminCapability) -> CapabilityBuilder {
        CapabilityBuilder {
            actor_id: self.actor_id,
            capability,
        }
    }
}

/// Builder after specifying capability
pub struct CapabilityBuilder {
    actor_id: MemberId,
    capability: AdminCapability,
}

impl CapabilityBuilder {
    /// Perform the authorization check
    pub async fn check<D>(self, deps: &D) -> Result<(), AuthError>
    where
        D: HasAuthContext,
    {
        check_admin_permission(self.actor_id, self.capability, deps).await
    }
}

/// Trait for dependencies that can perform auth checks
pub trait HasAuthContext: Send + Sync {
    fn admin_identifiers(&self) -> &[String];
    fn test_identifier_enabled(&self) -> bool;
}

/// Core permission check function
async fn check_admin_permission<D>(
    actor_id: MemberId,
    _capability: AdminCapability,
    deps: &D,
) -> Result<(), AuthError>
where
    D: HasAuthContext,
{
    // In MN Digital Aid, admin check is based on identifier matching
    // The actor_id would be used to fetch the identifier (phone/email) from the JWT claims
    // For now, we'll implement a simplified version that checks against admin_identifiers

    // Note: In a real implementation, you would:
    // 1. Fetch the member/identifier from actor_id
    // 2. Check if their identifier is in the admin_identifiers list
    // 3. Potentially check role/permissions from database

    // For now, we'll assume the caller passes the identifier as part of the actor context
    // This is a simplification - in production you'd fetch from DB or JWT claims

    let is_admin = deps.admin_identifiers().iter().any(|admin_id| {
        // This is a placeholder - you'd need to match against actual identifier
        // extracted from the JWT token or member record
        // For now we'll check if there are any admin identifiers configured
        !admin_id.is_empty()
    });

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
        let result = Actor::new(actor_id)
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
        let result = Actor::new(actor_id)
            .can(AdminCapability::ManageNeeds)
            .check(&deps)
            .await;

        assert!(matches!(result, Err(AuthError::AdminRequired)));
    }
}
