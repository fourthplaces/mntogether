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
/// DEPRECATED: Use Listing instead.
pub struct OrganizationNeed;

/// Marker type for Listing entities (services, opportunities, businesses).
pub struct Listing;

/// Marker type for Organization entities.
pub struct Organization;

/// Marker type for Domain entities (websites we scrape).
pub struct Domain;

/// Marker type for Post entities (published posts for matching).
pub struct Post;

/// Marker type for OrganizationSource entities (website sources).
/// DEPRECATED: Use Domain instead.
pub struct OrganizationSource;

/// Marker type for ScrapeJob entities (scraping jobs).
pub struct ScrapeJob;

/// Marker type for Tag entities (universal tags).
pub struct Tag;

/// Marker type for Taggable entities (polymorphic tag associations).
pub struct Taggable;

/// Marker type for Chatroom entities (anonymous conversations).
pub struct Chatroom;

/// Marker type for Message entities (chat messages).
pub struct Message;

/// Marker type for ReferralDocument entities (generated referral documents).
pub struct ReferralDocument;

/// Marker type for ReferralDocumentTranslation entities.
pub struct ReferralDocumentTranslation;

/// Marker type for DocumentReference entities (document references for staleness detection).
pub struct DocumentReference;

// ============================================================================
// Type aliases - the primary API
// ============================================================================

/// Typed ID for Member entities.
pub type MemberId = Id<Member>;

/// Typed ID for OrganizationNeed entities.
/// DEPRECATED: Use ListingId instead.
pub type NeedId = Id<OrganizationNeed>;

/// Typed ID for Listing entities (services, opportunities, businesses).
pub type ListingId = Id<Listing>;

/// Typed ID for Organization entities.
pub type OrganizationId = Id<Organization>;

/// Typed ID for Domain entities (websites we scrape).
pub type DomainId = Id<Domain>;

/// Typed ID for Post entities.
pub type PostId = Id<Post>;

/// Typed ID for OrganizationSource entities.
/// DEPRECATED: Use DomainId instead.
pub type SourceId = Id<OrganizationSource>;

/// Typed ID for ScrapeJob entities.
pub type JobId = Id<ScrapeJob>;

/// Typed ID for Tag entities.
pub type TagId = Id<Tag>;

/// Typed ID for Taggable entities.
pub type TaggableId = Id<Taggable>;

/// Typed ID for Chatroom entities.
pub type ChatroomId = Id<Chatroom>;

/// Typed ID for Message entities.
pub type MessageId = Id<Message>;

/// Typed ID for ReferralDocument entities.
pub type DocumentId = Id<ReferralDocument>;

/// Typed ID for ReferralDocumentTranslation entities.
pub type DocumentTranslationId = Id<ReferralDocumentTranslation>;

/// Typed ID for DocumentReference entities.
pub type DocumentReferenceId = Id<DocumentReference>;
