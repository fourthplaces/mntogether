//! Typed ID definitions for all domain entities.
//!
//! This module defines type aliases for each domain entity, providing
//! compile-time type safety for ID usage throughout the application.
//!
//! # Example
//!
//! ```rust
//! use crate::common::{MemberId, NeedId, PostId};
//!
//! // These are incompatible types - compiler prevents mixing them up
//! let member_id: MemberId = MemberId::new();
//! let need_id: NeedId = NeedId::new();
//!
//! // This would be a compile error:
//! // let wrong: NeedId = member_id;
//! ```

// Re-export the core Id type and version markers
pub use super::id::{Id, V4, V7};

// ============================================================================
// Entity marker types
// ============================================================================

/// Marker type for Member entities (users).
pub struct Member;

/// Marker type for OrganizationNeed entities (needs/opportunities).
pub struct OrganizationNeed;

/// Marker type for Post entities (published posts for matching).
pub struct Post;

/// Marker type for OrganizationSource entities (website sources).
pub struct OrganizationSource;

/// Marker type for ScrapeJob entities (scraping jobs).
pub struct ScrapeJob;

// ============================================================================
// Type aliases - the primary API
// ============================================================================

/// Typed ID for Member entities.
pub type MemberId = Id<Member>;

/// Typed ID for OrganizationNeed entities.
pub type NeedId = Id<OrganizationNeed>;

/// Typed ID for Post entities.
pub type PostId = Id<Post>;

/// Typed ID for OrganizationSource entities.
pub type SourceId = Id<OrganizationSource>;

/// Typed ID for ScrapeJob entities.
pub type JobId = Id<ScrapeJob>;
