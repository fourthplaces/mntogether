use std::collections::HashSet;

use crate::common::WebsiteId;
use crate::domains::posts::commands::PostCommand;
use crate::domains::posts::events::PostEvent;

/// Listing state machine
/// Pure decision logic - NO IO, only state transitions
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
            // =========================================================================
            // Scraping workflow: Request → Scrape → Extract → Sync
            // =========================================================================
            PostEvent::ScrapeSourceRequested {
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
                page_snapshot_id: _,
            } => {
                // Source was scraped, now extract listings from content
                Some(PostCommand::ExtractListings {
                    source_id: *source_id,
                    job_id: *job_id,
                    organization_name: organization_name.clone(),
                    content: content.clone(),
                })
            }

            PostEvent::ResourceLinkScraped {
                job_id,
                url,
                content,
                context,
                submitter_contact,
                page_snapshot_id: _,
            } => {
                // Resource link was scraped, now extract listings from content
                Some(PostCommand::ExtractListingsFromResourceLink {
                    job_id: *job_id,
                    url: url.clone(),
                    content: content.clone(),
                    context: context.clone(),
                    submitter_contact: submitter_contact.clone(),
                })
            }

            PostEvent::ListingsExtracted {
                source_id,
                job_id,
                listings,
            } => {
                // Listings extracted, now sync with database
                Some(PostCommand::SyncListings {
                    source_id: *source_id,
                    job_id: *job_id,
                    listings: listings.clone(),
                })
            }

            PostEvent::ResourceLinkListingsExtracted {
                job_id,
                url,
                listings,
                context,
                submitter_contact,
            } => {
                // Listings extracted from resource link, create them as user_submitted
                Some(PostCommand::CreateListingsFromResourceLink {
                    job_id: *job_id,
                    url: url.clone(),
                    listings: listings.clone(),
                    context: context.clone(),
                    submitter_contact: submitter_contact.clone(),
                })
            }

            PostEvent::ListingsSynced { source_id, .. } => {
                // Sync complete, scrape workflow done
                self.pending_scrapes.remove(source_id);
                None
            }

            // =========================================================================
            // Failure events - terminal events that clean up workflow state
            // =========================================================================
            PostEvent::ScrapeFailed { source_id, .. } => {
                // Scrape failed, clean up pending state so retries can happen
                self.pending_scrapes.remove(source_id);
                None
            }

            PostEvent::ExtractFailed { source_id, .. } => {
                // Extract failed, clean up pending state so retries can happen
                self.pending_scrapes.remove(source_id);
                None
            }

            PostEvent::SyncFailed { source_id, .. } => {
                // Sync failed, clean up pending state so retries can happen
                self.pending_scrapes.remove(source_id);
                None
            }

            PostEvent::ResourceLinkScrapeFailed { .. } => {
                // Resource link scrape failed, terminal event
                None
            }

            // =========================================================================
            // Post management workflows: Request → Create/Update
            // =========================================================================
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

            PostEvent::RepostListingRequested {
                post_id,
                requested_by,
                is_admin,
            } => Some(PostCommand::RepostListing {
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

            PostEvent::PostViewedRequested { post_id } => {
                Some(PostCommand::IncrementPostView { post_id: *post_id })
            }

            PostEvent::PostClickedRequested { post_id } => {
                Some(PostCommand::IncrementPostClick { post_id: *post_id })
            }

            // =========================================================================
            // Resource link submission workflow: Request → Create Source → Scrape → Extract → Create listings
            // =========================================================================
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

            // Website pending approval - no scraping yet
            PostEvent::WebsitePendingApproval { .. } => None,

            // After source created, start scraping
            PostEvent::WebsiteCreatedFromLink {
                job_id,
                url,
                submitter_contact,
                ..
            } => Some(PostCommand::ScrapeResourceLink {
                job_id: *job_id,
                url: url.clone(),
                context: None, // Already stored in source
                submitter_contact: submitter_contact.clone(),
            }),

            // =========================================================================
            // User submission workflow: Request → Create
            // =========================================================================
            PostEvent::SubmitListingRequested {
                member_id,
                organization_name,
                title,
                description,
                contact_info,
                urgency,
                location,
                ip_address,
            } => Some(PostCommand::CreateListing {
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

            PostEvent::ListingCreated { post_id, .. } => {
                // Listing created, generate embedding
                Some(PostCommand::GenerateListingEmbedding {
                    post_id: *post_id,
                })
            }

            // Request to generate embedding for a single post (admin action)
            PostEvent::GenerateListingEmbeddingRequested { post_id } => {
                Some(PostCommand::GenerateListingEmbedding {
                    post_id: *post_id,
                })
            }

            // =========================================================================
            // Approval workflows: Request → Update Status
            // =========================================================================
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
            } => Some(PostCommand::UpdateListingAndApprove {
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

            PostEvent::DeleteListingRequested {
                post_id,
                requested_by,
                is_admin,
            } => Some(PostCommand::DeleteListing {
                post_id: *post_id,
                requested_by: *requested_by,
                is_admin: *is_admin,
            }),

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

            // Terminal events - no further commands needed
            PostEvent::ListingApproved { post_id } => {
                // When listing is approved, create a post
                Some(PostCommand::CreatePost {
                    post_id: *post_id,
                    created_by: None, // TODO: Get from context
                    custom_title: None,
                    custom_description: None,
                    expires_in_days: None, // Default 5 days
                })
            }

            PostEvent::ListingRejected { .. } => None,
            PostEvent::ListingUpdated { .. } => None,
            PostEvent::ListingDeleted { .. } => None,
            PostEvent::PostReported { .. } => None,
            PostEvent::ReportResolved { .. } => None,
            PostEvent::ReportDismissed { .. } => None,
            PostEvent::PostCreated { .. } => None,
            PostEvent::PostExpired { .. } => None,
            PostEvent::PostArchived { .. } => None,
            PostEvent::PostViewed { .. } => None,
            PostEvent::PostClicked { .. } => None,

            // Embedding events - no further action needed
            PostEvent::ListingEmbeddingGenerated { .. } => None,
            PostEvent::ListingEmbeddingFailed { .. } => None,

            // Authorization events - terminal events, no further action needed
            PostEvent::AuthorizationDenied { .. } => None,

            // =========================================================================
            // Website Crawl workflow: Request → Crawl → Extract → Sync
            // =========================================================================
            PostEvent::CrawlWebsiteRequested {
                website_id,
                job_id,
                requested_by,
                is_admin,
            } => Some(PostCommand::CrawlWebsite {
                website_id: *website_id,
                job_id: *job_id,
                requested_by: *requested_by,
                is_admin: *is_admin,
            }),

            // =========================================================================
            // Website Crawl workflow: Crawl → Extract → Sync (or retry)
            // =========================================================================
            PostEvent::WebsiteCrawled {
                website_id,
                job_id,
                pages,
            } => {
                // Website was crawled, now extract listings from all pages
                Some(PostCommand::ExtractListingsFromPages {
                    website_id: *website_id,
                    job_id: *job_id,
                    pages: pages.clone(),
                })
            }

            PostEvent::ListingsExtractedFromPages {
                website_id,
                job_id,
                listings,
                page_results,
            } => {
                // Listings extracted from crawled pages, now sync to database
                Some(PostCommand::SyncCrawledListings {
                    website_id: *website_id,
                    job_id: *job_id,
                    listings: listings.clone(),
                    page_results: page_results.clone(),
                })
            }

            PostEvent::WebsiteCrawlNoListings {
                website_id,
                job_id,
                should_retry,
                ..
            } => {
                if *should_retry {
                    // Retry the crawl
                    Some(PostCommand::RetryWebsiteCrawl {
                        website_id: *website_id,
                        job_id: *job_id,
                    })
                } else {
                    // Max retries reached, mark as no listings
                    Some(PostCommand::MarkWebsiteNoListings {
                        website_id: *website_id,
                        job_id: *job_id,
                    })
                }
            }

            // Terminal crawl events - no further action needed
            PostEvent::WebsiteMarkedNoListings { website_id, .. } => {
                // Clean up pending state
                self.pending_scrapes.remove(website_id);
                None
            }

            PostEvent::WebsiteCrawlFailed { website_id, .. } => {
                // Clean up pending state on failure
                self.pending_scrapes.remove(website_id);
                None
            }

            // =========================================================================
            // Regenerate Posts workflow: Request → Extract from existing snapshots → Sync
            // =========================================================================
            PostEvent::RegeneratePostsRequested {
                website_id,
                job_id,
                requested_by,
                is_admin,
            } => Some(PostCommand::RegeneratePosts {
                website_id: *website_id,
                job_id: *job_id,
                requested_by: *requested_by,
                is_admin: *is_admin,
            }),

            // =========================================================================
            // Regenerate Page Summaries workflow: Request → Regenerate summaries
            // =========================================================================
            PostEvent::RegeneratePageSummariesRequested {
                website_id,
                job_id,
                requested_by,
                is_admin,
            } => Some(PostCommand::RegeneratePageSummaries {
                website_id: *website_id,
                job_id: *job_id,
                requested_by: *requested_by,
                is_admin: *is_admin,
            }),

            // Terminal event - page summaries regenerated
            PostEvent::PageSummariesRegenerated { .. } => None,

            // =========================================================================
            // Single Page Regeneration workflows
            // =========================================================================
            PostEvent::RegeneratePageSummaryRequested {
                page_snapshot_id,
                job_id,
                requested_by,
                is_admin,
            } => Some(PostCommand::RegeneratePageSummary {
                page_snapshot_id: *page_snapshot_id,
                job_id: *job_id,
                requested_by: *requested_by,
                is_admin: *is_admin,
            }),

            PostEvent::RegeneratePagePostsRequested {
                page_snapshot_id,
                job_id,
                requested_by,
                is_admin,
            } => Some(PostCommand::RegeneratePagePosts {
                page_snapshot_id: *page_snapshot_id,
                job_id: *job_id,
                requested_by: *requested_by,
                is_admin: *is_admin,
            }),

            // Terminal events
            PostEvent::PageSummaryRegenerated { .. } => None,
            PostEvent::PagePostsRegenerated { .. } => None,
        }
    }
}
