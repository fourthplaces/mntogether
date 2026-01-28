use std::collections::HashSet;

use crate::common::SourceId;
use crate::domains::organization::commands::OrganizationCommand;
use crate::domains::organization::events::OrganizationEvent;

/// Organization state machine
/// Pure decision logic - NO IO, only state transitions
pub struct OrganizationMachine {
    /// Track pending scrapes to prevent duplicates
    pending_scrapes: HashSet<SourceId>,
}

impl OrganizationMachine {
    pub fn new() -> Self {
        Self {
            pending_scrapes: HashSet::new(),
        }
    }
}

impl Default for OrganizationMachine {
    fn default() -> Self {
        Self::new()
    }
}

impl seesaw::Machine for OrganizationMachine {
    type Event = OrganizationEvent;
    type Command = OrganizationCommand;

    fn decide(&mut self, event: &OrganizationEvent) -> Option<OrganizationCommand> {
        match event {
            // =========================================================================
            // Scraping workflow: Request → Scrape → Extract → Sync
            // =========================================================================
            OrganizationEvent::ScrapeSourceRequested {
                source_id,
                job_id,
                requested_by,
                is_admin,
            } => {
                // Prevent duplicate scrapes
                if self.pending_scrapes.contains(source_id) {
                    return None;
                }

                self.pending_scrapes.insert(*source_id);
                Some(OrganizationCommand::ScrapeSource {
                    source_id: *source_id,
                    job_id: *job_id,
                    requested_by: *requested_by,
                    is_admin: *is_admin,
                })
            }

            OrganizationEvent::SourceScraped {
                source_id,
                job_id,
                organization_name,
                content,
            } => {
                // Source was scraped, now extract needs from content
                Some(OrganizationCommand::ExtractNeeds {
                    source_id: *source_id,
                    job_id: *job_id,
                    organization_name: organization_name.clone(),
                    content: content.clone(),
                })
            }

            OrganizationEvent::NeedsExtracted {
                source_id,
                job_id,
                needs,
            } => {
                // Needs extracted, now sync with database
                Some(OrganizationCommand::SyncNeeds {
                    source_id: *source_id,
                    job_id: *job_id,
                    needs: needs.clone(),
                })
            }

            OrganizationEvent::NeedsSynced { source_id, .. } => {
                // Sync complete, scrape workflow done
                self.pending_scrapes.remove(source_id);
                None
            }

            // =========================================================================
            // Failure events - terminal events that clean up workflow state
            // =========================================================================
            OrganizationEvent::ScrapeFailed { source_id, .. } => {
                // Scrape failed, clean up pending state so retries can happen
                self.pending_scrapes.remove(source_id);
                None
            }

            OrganizationEvent::ExtractFailed { source_id, .. } => {
                // Extract failed, clean up pending state so retries can happen
                self.pending_scrapes.remove(source_id);
                None
            }

            OrganizationEvent::SyncFailed { source_id, .. } => {
                // Sync failed, clean up pending state so retries can happen
                self.pending_scrapes.remove(source_id);
                None
            }

            // =========================================================================
            // Post management workflows: Request → Create/Update
            // =========================================================================
            OrganizationEvent::CreateCustomPostRequested {
                need_id,
                custom_title,
                custom_description,
                custom_tldr,
                targeting_hints,
                expires_in_days,
                requested_by,
                is_admin,
            } => Some(OrganizationCommand::CreateCustomPost {
                need_id: *need_id,
                custom_title: custom_title.clone(),
                custom_description: custom_description.clone(),
                custom_tldr: custom_tldr.clone(),
                targeting_hints: targeting_hints.clone(),
                expires_in_days: *expires_in_days,
                created_by: *requested_by,
                requested_by: *requested_by,
                is_admin: *is_admin,
            }),

            OrganizationEvent::RepostNeedRequested {
                need_id,
                requested_by,
                is_admin,
            } => Some(OrganizationCommand::RepostNeed {
                need_id: *need_id,
                created_by: *requested_by,
                requested_by: *requested_by,
                is_admin: *is_admin,
            }),

            OrganizationEvent::ExpirePostRequested {
                post_id,
                requested_by,
                is_admin,
            } => Some(OrganizationCommand::ExpirePost {
                post_id: *post_id,
                requested_by: *requested_by,
                is_admin: *is_admin,
            }),

            OrganizationEvent::ArchivePostRequested {
                post_id,
                requested_by,
                is_admin,
            } => Some(OrganizationCommand::ArchivePost {
                post_id: *post_id,
                requested_by: *requested_by,
                is_admin: *is_admin,
            }),

            OrganizationEvent::PostViewedRequested { post_id } => {
                Some(OrganizationCommand::IncrementPostView { post_id: *post_id })
            }

            OrganizationEvent::PostClickedRequested { post_id } => {
                Some(OrganizationCommand::IncrementPostClick { post_id: *post_id })
            }

            // =========================================================================
            // User submission workflow: Request → Create
            // =========================================================================
            OrganizationEvent::SubmitNeedRequested {
                member_id,
                organization_name,
                title,
                description,
                contact_info,
                urgency,
                location,
                ip_address,
            } => Some(OrganizationCommand::CreateNeed {
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

            OrganizationEvent::NeedCreated { need_id, .. } => {
                // Need created, generate embedding
                Some(OrganizationCommand::GenerateNeedEmbedding { need_id: *need_id })
            }

            // =========================================================================
            // Approval workflows: Request → Update Status
            // =========================================================================
            OrganizationEvent::ApproveNeedRequested {
                need_id,
                requested_by,
                is_admin,
            } => Some(OrganizationCommand::UpdateNeedStatus {
                need_id: *need_id,
                status: "active".to_string(),
                rejection_reason: None,
                requested_by: *requested_by,
                is_admin: *is_admin,
            }),

            OrganizationEvent::EditAndApproveNeedRequested {
                need_id,
                title,
                description,
                description_markdown,
                tldr,
                contact_info,
                urgency,
                location,
                requested_by,
                is_admin,
            } => Some(OrganizationCommand::UpdateNeedAndApprove {
                need_id: *need_id,
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

            OrganizationEvent::RejectNeedRequested {
                need_id,
                reason,
                requested_by,
                is_admin,
            } => Some(OrganizationCommand::UpdateNeedStatus {
                need_id: *need_id,
                status: "rejected".to_string(),
                rejection_reason: Some(reason.clone()),
                requested_by: *requested_by,
                is_admin: *is_admin,
            }),

            // Terminal events - no further commands needed
            OrganizationEvent::NeedApproved { need_id } => {
                // When need is approved, create a post
                Some(OrganizationCommand::CreatePost {
                    need_id: *need_id,
                    created_by: None, // TODO: Get from context
                    custom_title: None,
                    custom_description: None,
                    expires_in_days: None, // Default 5 days
                })
            }

            OrganizationEvent::NeedRejected { .. } => None,
            OrganizationEvent::NeedUpdated { .. } => None,
            OrganizationEvent::PostCreated { .. } => None,
            OrganizationEvent::PostExpired { .. } => None,
            OrganizationEvent::PostArchived { .. } => None,
            OrganizationEvent::PostViewed { .. } => None,
            OrganizationEvent::PostClicked { .. } => None,

            // Embedding events - no further action needed
            OrganizationEvent::NeedEmbeddingGenerated { .. } => None,
            OrganizationEvent::NeedEmbeddingFailed { .. } => None,

            // Authorization events - terminal events, no further action needed
            OrganizationEvent::AuthorizationDenied { .. } => None,
        }
    }
}
