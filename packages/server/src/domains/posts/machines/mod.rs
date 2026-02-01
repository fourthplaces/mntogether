//! Post domain state machine
//!
//! Pure decision logic - NO IO, only state transitions.
//! This machine coordinates multiple workflows for the posts domain:
//!
//! - Scraping: Request → Scrape → Extract → Sync (single page)
//! - Submissions: User/resource link submissions
//! - Approval: Listing approval/rejection
//! - Post lifecycle: Create, expire, archive, view, click
//! - Reporting: Listing reports
//!
//! NOTE: Multi-page crawling workflows have been moved to the `crawling` domain.
//! See `crate::domains::crawling::machines` for crawling-related state transitions.

use std::collections::HashSet;

use crate::common::WebsiteId;
use crate::domains::posts::commands::PostCommand;
use crate::domains::posts::events::PostEvent;

/// Post domain state machine
pub struct PostMachine {
    /// Track pending scrapes to prevent duplicates
    pending_scrapes: HashSet<WebsiteId>,
}

impl PostMachine {
    pub fn new() -> Self {
        Self {
            pending_scrapes: HashSet::new(),
        }
    }

    /// Clean up pending state for a website
    fn cleanup_pending(&mut self, website_id: &WebsiteId) {
        self.pending_scrapes.remove(website_id);
    }

    /// Check if a scrape is already pending for this website
    fn is_scrape_pending(&self, website_id: &WebsiteId) -> bool {
        self.pending_scrapes.contains(website_id)
    }

    /// Mark a scrape as pending
    fn mark_scrape_pending(&mut self, website_id: WebsiteId) {
        self.pending_scrapes.insert(website_id);
    }
}

impl Default for PostMachine {
    fn default() -> Self {
        Self::new()
    }
}

impl seesaw_core::Machine for PostMachine {
    type Event = PostEvent;
    type Command = PostCommand;

    fn decide(&mut self, event: &PostEvent) -> Option<PostCommand> {
        match event {
            // =================================================================
            // SCRAPING WORKFLOW
            // =================================================================
            PostEvent::ScrapeSourceRequested {
                source_id,
                job_id,
                requested_by,
                is_admin,
            } => {
                if self.is_scrape_pending(source_id) {
                    return None;
                }
                self.mark_scrape_pending(*source_id);
                Some(PostCommand::ScrapeSource {
                    source_id: *source_id,
                    job_id: *job_id,
                    requested_by: *requested_by,
                    is_admin: *is_admin,
                })
            }

            PostEvent::SourceScraped {
                source_id,
                job_id,
                organization_name,
                content,
                ..
            } => Some(PostCommand::ExtractPosts {
                source_id: *source_id,
                job_id: *job_id,
                organization_name: organization_name.clone(),
                content: content.clone(),
            }),

            PostEvent::PostsExtracted {
                source_id,
                job_id,
                posts,
            } => Some(PostCommand::SyncPosts {
                source_id: *source_id,
                job_id: *job_id,
                posts: posts.clone(),
            }),

            PostEvent::PostsSynced { source_id, .. } => {
                self.cleanup_pending(source_id);
                None
            }

            // Scraping failures
            PostEvent::ScrapeFailed { source_id, .. }
            | PostEvent::ExtractFailed { source_id, .. }
            | PostEvent::SyncFailed { source_id, .. } => {
                self.cleanup_pending(source_id);
                None
            }

            // =================================================================
            // RESOURCE LINK SUBMISSION WORKFLOW
            // =================================================================
            PostEvent::SubmitResourceLinkRequested {
                url,
                context,
                submitter_contact,
            } => Some(PostCommand::CreateWebsiteFromLink {
                url: url.clone(),
                organization_name: context
                    .clone()
                    .unwrap_or_else(|| "Submitted Resource".to_string()),
                submitter_contact: submitter_contact.clone(),
            }),

            PostEvent::WebsitePendingApproval { .. } => None,

            PostEvent::WebsiteCreatedFromLink {
                job_id,
                url,
                submitter_contact,
                ..
            } => Some(PostCommand::ScrapeResourceLink {
                job_id: *job_id,
                url: url.clone(),
                context: None,
                submitter_contact: submitter_contact.clone(),
            }),

            PostEvent::ResourceLinkScraped {
                job_id,
                url,
                content,
                context,
                submitter_contact,
                ..
            } => Some(PostCommand::ExtractPostsFromResourceLink {
                job_id: *job_id,
                url: url.clone(),
                content: content.clone(),
                context: context.clone(),
                submitter_contact: submitter_contact.clone(),
            }),

            PostEvent::ResourceLinkPostsExtracted {
                job_id,
                url,
                posts,
                context,
                submitter_contact,
            } => Some(PostCommand::CreatePostsFromResourceLink {
                job_id: *job_id,
                url: url.clone(),
                posts: posts.clone(),
                context: context.clone(),
                submitter_contact: submitter_contact.clone(),
            }),

            PostEvent::ResourceLinkScrapeFailed { .. } => None,

            // =================================================================
            // USER SUBMISSION WORKFLOW
            // =================================================================
            PostEvent::SubmitListingRequested {
                member_id,
                organization_name,
                title,
                description,
                contact_info,
                urgency,
                location,
                ip_address,
            } => Some(PostCommand::CreatePostEntry {
                member_id: *member_id,
                organization_name: organization_name.clone(),
                title: title.clone(),
                description: description.clone(),
                contact_info: contact_info.clone(),
                urgency: urgency.clone(),
                location: location.clone(),
                ip_address: ip_address.clone(),
                submission_type: "user_submitted".to_string(),
            }),

            PostEvent::PostEntryCreated { post_id, .. } => {
                Some(PostCommand::GeneratePostEmbedding { post_id: *post_id })
            }

            PostEvent::GeneratePostEmbeddingRequested { post_id } => {
                Some(PostCommand::GeneratePostEmbedding { post_id: *post_id })
            }

            // =================================================================
            // APPROVAL WORKFLOW
            // =================================================================
            PostEvent::ApproveListingRequested {
                post_id,
                requested_by,
                is_admin,
            } => Some(PostCommand::UpdatePostStatus {
                post_id: *post_id,
                status: "active".to_string(),
                rejection_reason: None,
                requested_by: *requested_by,
                is_admin: *is_admin,
            }),

            PostEvent::EditAndApproveListingRequested {
                post_id,
                title,
                description,
                description_markdown,
                tldr,
                contact_info,
                urgency,
                location,
                requested_by,
                is_admin,
            } => Some(PostCommand::UpdatePostAndApprove {
                post_id: *post_id,
                title: title.clone(),
                description: description.clone(),
                description_markdown: description_markdown.clone(),
                tldr: tldr.clone(),
                contact_info: contact_info.clone(),
                urgency: urgency.clone(),
                location: location.clone(),
                requested_by: *requested_by,
                is_admin: *is_admin,
            }),

            PostEvent::RejectListingRequested {
                post_id,
                reason,
                requested_by,
                is_admin,
            } => Some(PostCommand::UpdatePostStatus {
                post_id: *post_id,
                status: "rejected".to_string(),
                rejection_reason: Some(reason.clone()),
                requested_by: *requested_by,
                is_admin: *is_admin,
            }),

            PostEvent::PostApproved { post_id } => Some(PostCommand::CreatePost {
                post_id: *post_id,
                created_by: None,
                custom_title: None,
                custom_description: None,
                expires_in_days: None,
            }),

            // =================================================================
            // POST LIFECYCLE
            // =================================================================
            PostEvent::CreateCustomPostRequested {
                post_id,
                custom_title,
                custom_description,
                custom_tldr,
                targeting_hints,
                expires_in_days,
                requested_by,
                is_admin,
            } => Some(PostCommand::CreateCustomPost {
                post_id: *post_id,
                custom_title: custom_title.clone(),
                custom_description: custom_description.clone(),
                custom_tldr: custom_tldr.clone(),
                targeting_hints: targeting_hints.clone(),
                expires_in_days: *expires_in_days,
                created_by: *requested_by,
                requested_by: *requested_by,
                is_admin: *is_admin,
            }),

            PostEvent::RepostPostRequested {
                post_id,
                requested_by,
                is_admin,
            } => Some(PostCommand::RepostPost {
                post_id: *post_id,
                created_by: *requested_by,
                requested_by: *requested_by,
                is_admin: *is_admin,
            }),

            PostEvent::ExpirePostRequested {
                post_id,
                requested_by,
                is_admin,
            } => Some(PostCommand::ExpirePost {
                post_id: *post_id,
                requested_by: *requested_by,
                is_admin: *is_admin,
            }),

            PostEvent::ArchivePostRequested {
                post_id,
                requested_by,
                is_admin,
            } => Some(PostCommand::ArchivePost {
                post_id: *post_id,
                requested_by: *requested_by,
                is_admin: *is_admin,
            }),

            PostEvent::DeletePostRequested {
                post_id,
                requested_by,
                is_admin,
            } => Some(PostCommand::DeletePost {
                post_id: *post_id,
                requested_by: *requested_by,
                is_admin: *is_admin,
            }),

            PostEvent::PostViewedRequested { post_id } => {
                Some(PostCommand::IncrementPostView { post_id: *post_id })
            }

            PostEvent::PostClickedRequested { post_id } => {
                Some(PostCommand::IncrementPostClick { post_id: *post_id })
            }

            // =================================================================
            // REPORTING WORKFLOW
            // =================================================================
            PostEvent::ReportListingRequested {
                post_id,
                reported_by,
                reporter_email,
                reason,
                category,
            } => Some(PostCommand::CreateReport {
                post_id: *post_id,
                reported_by: *reported_by,
                reporter_email: reporter_email.clone(),
                reason: reason.clone(),
                category: category.clone(),
            }),

            PostEvent::ResolveReportRequested {
                report_id,
                resolved_by,
                resolution_notes,
                action_taken,
                is_admin,
            } => Some(PostCommand::ResolveReport {
                report_id: *report_id,
                resolved_by: *resolved_by,
                resolution_notes: resolution_notes.clone(),
                action_taken: action_taken.clone(),
                is_admin: *is_admin,
            }),

            PostEvent::DismissReportRequested {
                report_id,
                resolved_by,
                resolution_notes,
                is_admin,
            } => Some(PostCommand::DismissReport {
                report_id: *report_id,
                resolved_by: *resolved_by,
                resolution_notes: resolution_notes.clone(),
                is_admin: *is_admin,
            }),

            // =================================================================
            // TERMINAL EVENTS (no further action)
            // =================================================================
            PostEvent::PostRejected { .. }
            | PostEvent::ListingUpdated { .. }
            | PostEvent::PostDeleted { .. }
            | PostEvent::PostReported { .. }
            | PostEvent::ReportResolved { .. }
            | PostEvent::ReportDismissed { .. }
            | PostEvent::PostCreated { .. }
            | PostEvent::PostExpired { .. }
            | PostEvent::PostArchived { .. }
            | PostEvent::PostViewed { .. }
            | PostEvent::PostClicked { .. }
            | PostEvent::PostEmbeddingGenerated { .. }
            | PostEvent::ListingEmbeddingFailed { .. }
            | PostEvent::AuthorizationDenied { .. }
            | PostEvent::DeduplicatePostsRequested { .. }
            | PostEvent::PostsDeduplicated { .. }
            | PostEvent::DeduplicationFailed { .. } => None,
        }
    }
}
