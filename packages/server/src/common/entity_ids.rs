//! Typed ID definitions for all domain entities.
//!
//! This module defines type aliases for each domain entity, providing
//! compile-time type safety for ID usage throughout the application.
//!
//! # Example
//!
//! ```rust
//! use crate::common::{MemberId, ListingId, PostId};
//!
//! // These are incompatible types - compiler prevents mixing them up
//! let member_id: MemberId = MemberId::new();
//! let listing_id: ListingId = ListingId::new();
//!
//! // This would be a compile error:
//! // let wrong: ListingId = member_id;
//! ```

// Re-export the core Id type and version markers
pub use super::id::{Id, V4, V7};

// ============================================================================
// Entity marker types
// ============================================================================

/// Marker type for Member entities (users).
pub struct Member;

/// Marker type for Listing entities (services, opportunities, businesses).
pub struct Listing;

/// Marker type for Organization entities.
pub struct Organization;

/// Marker type for Website entities (websites we scrape).
pub struct Website;

/// Marker type for Post entities (published posts for matching).
pub struct Post;

/// Marker type for ScrapeJob entities (scraping jobs).
pub struct ScrapeJob;

/// Marker type for Tag entities (universal tags).
pub struct Tag;

/// Marker type for Taggable entities (polymorphic tag associations).
pub struct Taggable;

/// Marker type for Container entities (message containers for chat, comments, discussions).
pub struct Container;

/// Marker type for Message entities (chat messages).
pub struct Message;

/// Marker type for ReferralDocument entities (generated referral documents).
pub struct ReferralDocument;

/// Marker type for ReferralDocumentTranslation entities.
pub struct ReferralDocumentTranslation;

/// Marker type for DocumentReference entities (document references for staleness detection).
pub struct DocumentReference;

/// Marker type for Provider entities (professionals in the provider directory).
pub struct Provider;

/// Marker type for Contact entities (polymorphic contact information).
pub struct Contact;

/// Marker type for Resource entities (extracted services/programs from websites).
pub struct Resource;

/// Marker type for ResourceSource entities (links resources to source pages).
pub struct ResourceSource;

/// Marker type for ResourceVersion entities (audit trail for resource changes).
pub struct ResourceVersion;

// ============================================================================
// Type aliases - the primary API
// ============================================================================

/// Typed ID for Member entities.
pub type MemberId = Id<Member>;

/// Typed ID for Listing entities (services, opportunities, businesses).
pub type ListingId = Id<Listing>;

/// Typed ID for Organization entities.
pub type OrganizationId = Id<Organization>;

/// Typed ID for Website entities (websites we scrape).
pub type WebsiteId = Id<Website>;

/// Typed ID for Post entities.
pub type PostId = Id<Post>;

/// Typed ID for ScrapeJob entities.
pub type JobId = Id<ScrapeJob>;

/// Typed ID for Tag entities.
pub type TagId = Id<Tag>;

/// Typed ID for Taggable entities.
pub type TaggableId = Id<Taggable>;

/// Typed ID for Container entities (message containers for chat, comments, discussions).
pub type ContainerId = Id<Container>;

/// Typed ID for Message entities.
pub type MessageId = Id<Message>;

/// Typed ID for ReferralDocument entities.
pub type DocumentId = Id<ReferralDocument>;

/// Typed ID for ReferralDocumentTranslation entities.
pub type DocumentTranslationId = Id<ReferralDocumentTranslation>;

/// Typed ID for DocumentReference entities.
pub type DocumentReferenceId = Id<DocumentReference>;

/// Typed ID for Provider entities.
pub type ProviderId = Id<Provider>;

/// Typed ID for Contact entities.
pub type ContactId = Id<Contact>;

/// Typed ID for Resource entities (extracted services/programs).
pub type ResourceId = Id<Resource>;

/// Typed ID for ResourceSource entities.
pub type ResourceSourceId = Id<ResourceSource>;

/// Typed ID for ResourceVersion entities.
pub type ResourceVersionId = Id<ResourceVersion>;
