use std::collections::HashSet;

use crate::common::SourceId;
use crate::domains::listings::commands::ListingCommand;
use crate::domains::listings::events::ListingEvent;

/// Listing state machine
/// Pure decision logic - NO IO, only state transitions
pub struct ListingMachine {
    /// Track pending scrapes to prevent duplicates
    pending_scrapes: HashSet<SourceId>,
}

impl ListingMachine {
    pub fn new() -> Self {
        Self {
            pending_scrapes: HashSet::new(),
        }
    }
}

impl Default for ListingMachine {
    fn default() -> Self {
        Self::new()
    }
}

impl seesaw_core::Machine for ListingMachine {
    type Event = ListingEvent;
    type Command = ListingCommand;

    fn decide(&mut self, event: &ListingEvent) -> Option<ListingCommand> {
        match event {
            // =========================================================================
            // Scraping workflow: Request → Scrape → Extract → Sync
            // =========================================================================
            ListingEvent::ScrapeSourceRequested {
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
                Some(ListingCommand::ScrapeSource {
                    source_id: *source_id,
                    job_id: *job_id,
                    requested_by: *requested_by,
                    is_admin: *is_admin,
                })
            }

            ListingEvent::SourceScraped {
                source_id,
                job_id,
                organization_name,
                content,
            } => {
                // Source was scraped, now extract listings from content
                Some(ListingCommand::ExtractListings {
                    source_id: *source_id,
                    job_id: *job_id,
                    organization_name: organization_name.clone(),
                    content: content.clone(),
                })
            }

            ListingEvent::ResourceLinkScraped {
                job_id,
                url,
                content,
                context,
                submitter_contact,
            } => {
                // Resource link was scraped, now extract listings from content
                Some(ListingCommand::ExtractListingsFromResourceLink {
                    job_id: *job_id,
                    url: url.clone(),
                    content: content.clone(),
                    context: context.clone(),
                    submitter_contact: submitter_contact.clone(),
                })
            }

            ListingEvent::ListingsExtracted {
                source_id,
                job_id,
                listings,
            } => {
                // Listings extracted, now sync with database
                Some(ListingCommand::SyncListings {
                    source_id: *source_id,
                    job_id: *job_id,
                    listings: listings.clone(),
                })
            }

            ListingEvent::ResourceLinkListingsExtracted {
                job_id,
                url,
                listings,
                context,
                submitter_contact,
            } => {
                // Listings extracted from resource link, create them as user_submitted
                Some(ListingCommand::CreateListingsFromResourceLink {
                    job_id: *job_id,
                    url: url.clone(),
                    listings: listings.clone(),
                    context: context.clone(),
                    submitter_contact: submitter_contact.clone(),
                })
            }

            ListingEvent::ListingsSynced { source_id, .. } => {
                // Sync complete, scrape workflow done
                self.pending_scrapes.remove(source_id);
                None
            }

            // =========================================================================
            // Failure events - terminal events that clean up workflow state
            // =========================================================================
            ListingEvent::ScrapeFailed { source_id, .. } => {
                // Scrape failed, clean up pending state so retries can happen
                self.pending_scrapes.remove(source_id);
                None
            }

            ListingEvent::ExtractFailed { source_id, .. } => {
                // Extract failed, clean up pending state so retries can happen
                self.pending_scrapes.remove(source_id);
                None
            }

            ListingEvent::SyncFailed { source_id, .. } => {
                // Sync failed, clean up pending state so retries can happen
                self.pending_scrapes.remove(source_id);
                None
            }

            ListingEvent::ResourceLinkScrapeFailed { .. } => {
                // Resource link scrape failed, terminal event
                None
            }

            // =========================================================================
            // Post management workflows: Request → Create/Update
            // =========================================================================
            ListingEvent::CreateCustomPostRequested {
                listing_id,
                custom_title,
                custom_description,
                custom_tldr,
                targeting_hints,
                expires_in_days,
                requested_by,
                is_admin,
            } => Some(ListingCommand::CreateCustomPost {
                listing_id: *listing_id,
                custom_title: custom_title.clone(),
                custom_description: custom_description.clone(),
                custom_tldr: custom_tldr.clone(),
                targeting_hints: targeting_hints.clone(),
                expires_in_days: *expires_in_days,
                created_by: *requested_by,
                requested_by: *requested_by,
                is_admin: *is_admin,
            }),

            ListingEvent::RepostListingRequested {
                listing_id,
                requested_by,
                is_admin,
            } => Some(ListingCommand::RepostListing {
                listing_id: *listing_id,
                created_by: *requested_by,
                requested_by: *requested_by,
                is_admin: *is_admin,
            }),

            ListingEvent::ExpirePostRequested {
                post_id,
                requested_by,
                is_admin,
            } => Some(ListingCommand::ExpirePost {
                post_id: *post_id,
                requested_by: *requested_by,
                is_admin: *is_admin,
            }),

            ListingEvent::ArchivePostRequested {
                post_id,
                requested_by,
                is_admin,
            } => Some(ListingCommand::ArchivePost {
                post_id: *post_id,
                requested_by: *requested_by,
                is_admin: *is_admin,
            }),

            ListingEvent::PostViewedRequested { post_id } => {
                Some(ListingCommand::IncrementPostView { post_id: *post_id })
            }

            ListingEvent::PostClickedRequested { post_id } => {
                Some(ListingCommand::IncrementPostClick { post_id: *post_id })
            }

            // =========================================================================
            // Resource link submission workflow: Request → Create Source → Scrape → Extract → Create listings
            // =========================================================================
            ListingEvent::SubmitResourceLinkRequested {
                url,
                context,
                submitter_contact,
            } => Some(ListingCommand::CreateOrganizationSourceFromLink {
                url: url.clone(),
                organization_name: context.clone().unwrap_or_else(|| "Submitted Resource".to_string()),
                submitter_contact: submitter_contact.clone(),
            }),

            // After source created, start scraping
            ListingEvent::OrganizationSourceCreatedFromLink {
                job_id,
                url,
                submitter_contact,
                ..
            } => Some(ListingCommand::ScrapeResourceLink {
                job_id: *job_id,
                url: url.clone(),
                context: None, // Already stored in source
                submitter_contact: submitter_contact.clone(),
            }),

            // =========================================================================
            // User submission workflow: Request → Create
            // =========================================================================
            ListingEvent::SubmitListingRequested {
                member_id,
                organization_name,
                title,
                description,
                contact_info,
                urgency,
                location,
                ip_address,
            } => Some(ListingCommand::CreateListing {
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

            ListingEvent::ListingCreated { listing_id, .. } => {
                // Listing created, generate embedding
                Some(ListingCommand::GenerateListingEmbedding { listing_id: *listing_id })
            }

            // =========================================================================
            // Approval workflows: Request → Update Status
            // =========================================================================
            ListingEvent::ApproveListingRequested {
                listing_id,
                requested_by,
                is_admin,
            } => Some(ListingCommand::UpdateListingStatus {
                listing_id: *listing_id,
                status: "active".to_string(),
                rejection_reason: None,
                requested_by: *requested_by,
                is_admin: *is_admin,
            }),

            ListingEvent::EditAndApproveListingRequested {
                listing_id,
                title,
                description,
                description_markdown,
                tldr,
                contact_info,
                urgency,
                location,
                requested_by,
                is_admin,
            } => Some(ListingCommand::UpdateListingAndApprove {
                listing_id: *listing_id,
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

            ListingEvent::RejectListingRequested {
                listing_id,
                reason,
                requested_by,
                is_admin,
            } => Some(ListingCommand::UpdateListingStatus {
                listing_id: *listing_id,
                status: "rejected".to_string(),
                rejection_reason: Some(reason.clone()),
                requested_by: *requested_by,
                is_admin: *is_admin,
            }),

            ListingEvent::DeleteListingRequested {
                listing_id,
                requested_by,
                is_admin,
            } => Some(ListingCommand::DeleteListing {
                listing_id: *listing_id,
                requested_by: *requested_by,
                is_admin: *is_admin,
            }),

            ListingEvent::AddScrapeUrlRequested {
                source_id,
                url,
                requested_by,
                is_admin,
            } => Some(ListingCommand::AddScrapeUrl {
                source_id: *source_id,
                url: url.clone(),
                requested_by: *requested_by,
                is_admin: *is_admin,
            }),

            ListingEvent::RemoveScrapeUrlRequested {
                source_id,
                url,
                requested_by,
                is_admin,
            } => Some(ListingCommand::RemoveScrapeUrl {
                source_id: *source_id,
                url: url.clone(),
                requested_by: *requested_by,
                is_admin: *is_admin,
            }),

            // Terminal events - no further commands needed
            ListingEvent::ListingApproved { listing_id } => {
                // When listing is approved, create a post
                Some(ListingCommand::CreatePost {
                    listing_id: *listing_id,
                    created_by: None, // TODO: Get from context
                    custom_title: None,
                    custom_description: None,
                    expires_in_days: None, // Default 5 days
                })
            }

            ListingEvent::ListingRejected { .. } => None,
            ListingEvent::ListingUpdated { .. } => None,
            ListingEvent::ListingDeleted { .. } => None,
            ListingEvent::PostCreated { .. } => None,
            ListingEvent::PostExpired { .. } => None,
            ListingEvent::PostArchived { .. } => None,
            ListingEvent::PostViewed { .. } => None,
            ListingEvent::PostClicked { .. } => None,
            ListingEvent::ScrapeUrlAdded { .. } => None,
            ListingEvent::ScrapeUrlRemoved { .. } => None,

            // Embedding events - no further action needed
            ListingEvent::ListingEmbeddingGenerated { .. } => None,
            ListingEvent::ListingEmbeddingFailed { .. } => None,

            // Authorization events - terminal events, no further action needed
            ListingEvent::AuthorizationDenied { .. } => None,

            // =========================================================================
            // Intelligent Crawler workflow: Request → Crawl → Detect → Extract → Relate
            // =========================================================================
            ListingEvent::SiteCrawlRequested { url, job_id } => {
                Some(ListingCommand::CrawlSite {
                    url: url.clone(),
                    job_id: *job_id,
                    page_limit: Some(15), // Default limit
                })
            }

            ListingEvent::SiteCrawled {
                url: _,
                job_id,
                snapshot_ids,
            } => {
                // Site was crawled, now detect information in the pages
                Some(ListingCommand::DetectInformation {
                    snapshot_ids: snapshot_ids.clone(),
                    job_id: *job_id,
                    detection_kind: "volunteer_opportunity".to_string(), // TODO: Make configurable
                })
            }

            ListingEvent::InformationDetected {
                job_id: _,
                detection_ids,
            } => {
                // Information detected, now extract structured data
                // For now, skip extraction if no detections found
                if detection_ids.is_empty() {
                    None
                } else {
                    // TODO: Get schema_id from configuration or detection kind
                    // For now, we'll skip extraction as it needs a real schema
                    None
                }
            }

            ListingEvent::DataExtracted {
                job_id,
                extraction_ids,
            } => {
                // Data extracted, now resolve relationships
                if extraction_ids.is_empty() {
                    None
                } else {
                    Some(ListingCommand::ResolveRelationships {
                        extraction_ids: extraction_ids.clone(),
                        job_id: *job_id,
                    })
                }
            }

            ListingEvent::RelationshipsResolved { .. } => {
                // Workflow complete
                None
            }

            // Intelligent crawler failure events - terminal events
            ListingEvent::SiteCrawlFailed { .. } => None,
            ListingEvent::InformationDetectionFailed { .. } => None,
            ListingEvent::DataExtractionFailed { .. } => None,
            ListingEvent::RelationshipResolutionFailed { .. } => None,
        }
    }
}
