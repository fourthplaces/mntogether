use std::collections::HashMap;
use uuid::Uuid;

use crate::domains::organization::commands::OrganizationCommand;
use crate::domains::organization::events::OrganizationEvent;

/// Organization state machine
/// Pure decision logic - NO IO, only state transitions
pub struct OrganizationMachine {
    /// Track pending scrapes to prevent duplicates
    pending_scrapes: HashMap<Uuid, ()>,
}

impl OrganizationMachine {
    pub fn new() -> Self {
        Self {
            pending_scrapes: HashMap::new(),
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
            OrganizationEvent::ScrapeSourceRequested { source_id, job_id } => {
                // Prevent duplicate scrapes
                if self.pending_scrapes.contains_key(source_id) {
                    return None;
                }

                self.pending_scrapes.insert(*source_id, ());
                Some(OrganizationCommand::ScrapeSource {
                    source_id: *source_id,
                    job_id: *job_id,
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
            // User submission workflow: Request → Create
            // =========================================================================
            OrganizationEvent::SubmitNeedRequested {
                volunteer_id,
                organization_name,
                title,
                description,
                contact_info,
                urgency,
                location,
                ip_address,
            } => Some(OrganizationCommand::CreateNeed {
                volunteer_id: *volunteer_id,
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
            OrganizationEvent::ApproveNeedRequested { need_id } => {
                Some(OrganizationCommand::UpdateNeedStatus {
                    need_id: *need_id,
                    status: "active".to_string(),
                    rejection_reason: None,
                })
            }

            OrganizationEvent::EditAndApproveNeedRequested {
                need_id,
                title,
                description,
                description_markdown,
                tldr,
                contact_info,
                urgency,
                location,
            } => Some(OrganizationCommand::UpdateNeedAndApprove {
                need_id: *need_id,
                title: title.clone(),
                description: description.clone(),
                description_markdown: description_markdown.clone(),
                tldr: tldr.clone(),
                contact_info: contact_info.clone(),
                urgency: urgency.clone(),
                location: location.clone(),
            }),

            OrganizationEvent::RejectNeedRequested { need_id, reason } => {
                Some(OrganizationCommand::UpdateNeedStatus {
                    need_id: *need_id,
                    status: "rejected".to_string(),
                    rejection_reason: Some(reason.clone()),
                })
            }

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

            // Embedding events - no further action needed
            OrganizationEvent::NeedEmbeddingGenerated { .. } => None,
            OrganizationEvent::NeedEmbeddingFailed { .. } => None,
        }
    }
}
