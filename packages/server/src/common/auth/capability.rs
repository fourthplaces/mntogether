/// Capabilities in the MN Digital Aid platform
///
/// This is a simplified model focused on admin operations since the platform
/// is primarily admin-managed with automated matching for volunteers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdminCapability {
    /// Approve or reject needs
    ManageNeeds,

    /// Create and manage posts
    ManagePosts,

    /// Trigger scraping operations
    TriggerScraping,

    /// Manage member status
    ManageMembers,

    /// Manage listings (create, update, delete, verify)
    ManageListings,

    /// Full admin access to all operations
    FullAdmin,
}

impl AdminCapability {
    /// Check if this capability requires admin access
    pub fn requires_admin(&self) -> bool {
        // All capabilities in this system require admin access
        true
    }
}
