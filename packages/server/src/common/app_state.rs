//! Application state for the seesaw engine.

use uuid::Uuid;

/// Application state passed to engine.activate().
///
/// This is the same for all domains - just tracks request-scoped data
/// like visitor ID. Domain-specific results come from action return values.
#[derive(Clone, Default)]
pub struct AppState {
    /// The authenticated user's member ID, if any.
    pub visitor_id: Option<Uuid>,
    /// Whether the visitor has admin privileges.
    pub is_admin: bool,
}

impl AppState {
    /// Create state for an authenticated visitor.
    pub fn authenticated(visitor_id: Uuid, is_admin: bool) -> Self {
        Self {
            visitor_id: Some(visitor_id),
            is_admin,
        }
    }

    /// Create state for an unauthenticated/anonymous request.
    pub fn anonymous() -> Self {
        Self::default()
    }

    /// Check if the visitor is authenticated.
    pub fn is_authenticated(&self) -> bool {
        self.visitor_id.is_some()
    }

    /// Check if the visitor is an admin.
    /// Returns false for unauthenticated users.
    pub fn is_admin(&self) -> bool {
        self.visitor_id.is_some() && self.is_admin
    }

    /// Require the visitor to be an admin.
    /// Returns an error suitable for GraphQL if not admin.
    pub fn require_admin(&self) -> anyhow::Result<Uuid> {
        let visitor_id = self
            .visitor_id
            .ok_or_else(|| anyhow::anyhow!("Unauthenticated: Valid JWT required"))?;
        if !self.is_admin {
            anyhow::bail!("Unauthorized: Admin access required");
        }
        Ok(visitor_id)
    }

    /// Require the visitor to be authenticated.
    /// Returns the visitor_id or an error.
    pub fn require_auth(&self) -> anyhow::Result<Uuid> {
        self.visitor_id
            .ok_or_else(|| anyhow::anyhow!("Unauthenticated: Valid JWT required"))
    }
}
