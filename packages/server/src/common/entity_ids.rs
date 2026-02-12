//! Typed ID definitions for all domain entities.
//!
//! This module defines type aliases for each domain entity, providing
//! compile-time type safety for ID usage throughout the application.
//!
//! # Example
//!
//! ```rust
//! use crate::common::{MemberId, PostId};
//!
//! // These are incompatible types - compiler prevents mixing them up
//! let member_id: MemberId = MemberId::new();
//! let post_id: PostId = PostId::new();
//!
//! // This would be a compile error:
//! // let wrong: PostId = member_id;
//! ```

// Re-export the core Id type and version markers
pub use super::id::{Id, V4, V7};

// ============================================================================
// Entity marker types
// ============================================================================

/// Marker type for Member entities (users).
pub struct Member;

/// Marker type for Post entities (services, opportunities, businesses).
pub struct Post;

/// Marker type for Website entities (websites we scrape).
pub struct Website;

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

/// Marker type for Location entities (physical/virtual service delivery points).
pub struct Location;

/// Marker type for PostLocation entities (post-location join).
pub struct PostLocation;

/// Marker type for Schedule entities (operating hours and calendar events).
pub struct Schedule;

/// Marker type for TaxonomyCrosswalk entities (external taxonomy mapping).
pub struct TaxonomyCrosswalk;

/// Marker type for SyncBatch entities (groups of AI-proposed changes).
pub struct SyncBatch;

/// Marker type for SyncProposal entities (individual AI-proposed operations).
pub struct SyncProposal;

/// Marker type for ProposalComment entities (admin feedback on proposals).
pub struct ProposalComment;

/// Marker type for Organization entities (groups of related sources).
pub struct Organization;

/// Marker type for SocialProfile entities (social media profiles to scrape).
pub struct SocialProfile;

/// Marker type for Source entities (unified content sources).
pub struct Source;

/// Marker type for WebsiteSource entities (website-specific source details).
pub struct WebsiteSource;

/// Marker type for SocialSource entities (social-media-specific source details).
pub struct SocialSource;

/// Marker type for Note entities (attachable alerts and context).
pub struct Note;

/// Marker type for Noteable entities (polymorphic note associations).
pub struct Noteable;

/// Marker type for PostSource entities (unified post source tracking).
pub struct PostSource;

/// Marker type for Locatable entities (polymorphic location associations).
pub struct Locatable;

/// Marker type for HeatMapPoint entities (heat map snapshot points).
pub struct HeatMapPoint;

// ============================================================================
// Type aliases - the primary API
// ============================================================================

/// Typed ID for Member entities.
pub type MemberId = Id<Member>;

/// Typed ID for Post entities (services, opportunities, businesses).
pub type PostId = Id<Post>;

/// Typed ID for Website entities (websites we scrape).
pub type WebsiteId = Id<Website>;

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

/// Typed ID for Location entities.
pub type LocationId = Id<Location>;

/// Typed ID for PostLocation entities.
pub type PostLocationId = Id<PostLocation>;

/// Typed ID for Schedule entities.
pub type ScheduleId = Id<Schedule>;

/// Typed ID for TaxonomyCrosswalk entities.
pub type TaxonomyCrosswalkId = Id<TaxonomyCrosswalk>;

/// Typed ID for SyncBatch entities.
pub type SyncBatchId = Id<SyncBatch>;

/// Typed ID for SyncProposal entities.
pub type SyncProposalId = Id<SyncProposal>;

/// Typed ID for ProposalComment entities.
pub type ProposalCommentId = Id<ProposalComment>;

/// Typed ID for Organization entities.
pub type OrganizationId = Id<Organization>;

/// Typed ID for SocialProfile entities.
pub type SocialProfileId = Id<SocialProfile>;

/// Typed ID for Source entities (unified content sources).
pub type SourceId = Id<Source>;

/// Typed ID for WebsiteSource entities.
pub type WebsiteSourceId = Id<WebsiteSource>;

/// Typed ID for SocialSource entities.
pub type SocialSourceId = Id<SocialSource>;

/// Typed ID for Note entities.
pub type NoteId = Id<Note>;

/// Typed ID for Noteable entities.
pub type NoteableId = Id<Noteable>;

/// Typed ID for PostSource entities.
pub type PostSourceId = Id<PostSource>;

/// Typed ID for Locatable entities.
pub type LocatableId = Id<Locatable>;

/// Typed ID for HeatMapPoint entities.
pub type HeatMapPointId = Id<HeatMapPoint>;
