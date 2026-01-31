use crate::domains::listings::data::{EditListingInput, ListingType, ScrapeJobResult, SubmitListingInput, SubmitResourceLinkInput, SubmitResourceLinkResult};
use crate::domains::listings::data::listing_report::{ListingReport, ListingReportDetail};
use crate::domains::listings::models::listing_report::{ListingReportId, ListingReportRecord};
use crate::common::{JobId, ListingId, MemberId, WebsiteId};
use crate::domains::listings::events::ListingEvent;
use crate::domains::listings::models::Listing;
use crate::server::graphql::context::GraphQLContext;
use juniper::{FieldError, FieldResult};
use seesaw_core::{dispatch_request, EnvelopeMatch};
use tracing::info;
use uuid::Uuid;

/// Scrape an organization source (synchronous)
/// Waits for scraping to complete before returning
/// Following seesaw pattern: dispatch request event, await completion event
pub async fn scrape_organization(
    ctx: &GraphQLContext,
    source_id: Uuid,
) -> FieldResult<ScrapeJobResult> {
    info!(source_id = %source_id, "Scraping organization source");

    // Get user info (authorization will be checked in effect)
    let user = ctx
        .auth_user
        .as_ref()
        .ok_or_else(|| FieldError::new("Authentication required", juniper::Value::null()))?;

    // Convert to typed IDs
    let source_id = WebsiteId::from_uuid(source_id);
    let job_id = JobId::new();

    // Dispatch request event and await completion (ListingsSynced or failure)
    let result = dispatch_request(
        ListingEvent::ScrapeSourceRequested {
            source_id,
            job_id,
            requested_by: user.member_id,
            is_admin: user.is_admin,
        },
        &ctx.bus,
        |m| {
            m.try_match(|e: &ListingEvent| match e {
                // Success - scraping workflow complete
                ListingEvent::ListingsSynced {
                    source_id: synced_source_id,
                    job_id: synced_job_id,
                    new_count,
                    changed_count,
                    disappeared_count,
                } if *synced_source_id == source_id && *synced_job_id == job_id => {
                    Some(Ok((
                        "completed".to_string(),
                        format!(
                            "Scraping complete! Found {} new, {} changed, {} disappeared",
                            new_count, changed_count, disappeared_count
                        ),
                    )))
                }
                // Failure events
                ListingEvent::ScrapeFailed {
                    source_id: failed_source_id,
                    job_id: failed_job_id,
                    reason,
                } if *failed_source_id == source_id && *failed_job_id == job_id => {
                    Some(Err(anyhow::anyhow!("Scrape failed: {}", reason)))
                }
                ListingEvent::ExtractFailed {
                    source_id: failed_source_id,
                    job_id: failed_job_id,
                    reason,
                } if *failed_source_id == source_id && *failed_job_id == job_id => {
                    Some(Err(anyhow::anyhow!("Extraction failed: {}", reason)))
                }
                ListingEvent::SyncFailed {
                    source_id: failed_source_id,
                    job_id: failed_job_id,
                    reason,
                } if *failed_source_id == source_id && *failed_job_id == job_id => {
                    Some(Err(anyhow::anyhow!("Sync failed: {}", reason)))
                }
                ListingEvent::AuthorizationDenied {
                    user_id,
                    action,
                    reason,
                } if *user_id == user.member_id && action == "ScrapeSource" => {
                    Some(Err(anyhow::anyhow!("Authorization denied: {}", reason)))
                }
                _ => None,
            })
            .result()
        },
    )
    .await
    .map_err(|e| FieldError::new(format!("Scrape failed: {}", e), juniper::Value::null()))?;

    let (status, message) = result;

    Ok(ScrapeJobResult {
        job_id: job_id.into_uuid(),
        source_id: source_id.into_uuid(),
        status,
        message: Some(message),
    })
}

/// Submit a listing from a member (user-submitted, goes to pending_approval)
/// Following seesaw pattern: dispatch request event, await fact event
pub async fn submit_listing(
    ctx: &GraphQLContext,
    input: SubmitListingInput,
    member_id: Option<Uuid>,
    ip_address: Option<String>,
) -> FieldResult<ListingType> {
    info!(
        org = %input.organization_name,
        title = %input.title,
        member_id = ?member_id,
        "Submitting user listing"
    );

    let contact_json = input
        .contact_info
        .and_then(|c| serde_json::to_value(c).ok());

    // Convert to typed ID
    let member_id_typed = member_id.map(MemberId::from_uuid);

    // Dispatch request event and await ListingCreated fact event
    let listing_id = dispatch_request(
        ListingEvent::SubmitListingRequested {
            member_id: member_id_typed,
            organization_name: input.organization_name,
            title: input.title,
            description: input.description,
            contact_info: contact_json,
            urgency: input.urgency,
            location: input.location,
            ip_address,
        },
        &ctx.bus,
        |m| {
            m.try_match(|e: &ListingEvent| match e {
                ListingEvent::ListingCreated {
                    listing_id,
                    submission_type,
                    ..
                } if submission_type == "user_submitted" => Some(Ok(*listing_id)),
                _ => None,
            })
            .result()
        },
    )
    .await
    .map_err(|e| {
        FieldError::new(
            format!("Failed to submit listing: {}", e),
            juniper::Value::null(),
        )
    })?;

    // Query result from database (read queries OK in edges)
    let listing = Listing::find_by_id(listing_id, &ctx.db_pool)
        .await
        .map_err(|_| FieldError::new("Failed to fetch listing", juniper::Value::null()))?;

    Ok(ListingType::from(listing))
}

/// Approve a listing (human-in-the-loop)
/// Following seesaw pattern: dispatch request event, await fact event
pub async fn approve_listing(ctx: &GraphQLContext, listing_id: Uuid) -> FieldResult<ListingType> {
    info!(listing_id = %listing_id, "Approving listing (triggers matching)");

    // Get user info (authorization will be checked in effect)
    let user = ctx
        .auth_user
        .as_ref()
        .ok_or_else(|| FieldError::new("Authentication required", juniper::Value::null()))?;

    // Convert to typed ID
    let listing_id = ListingId::from_uuid(listing_id);

    // Dispatch request event and await ListingApproved fact event
    dispatch_request(
        ListingEvent::ApproveListingRequested {
            listing_id,
            requested_by: user.member_id,
            is_admin: user.is_admin,
        },
        &ctx.bus,
        |m| {
            m.try_match(|e: &ListingEvent| match e {
                ListingEvent::ListingApproved { listing_id: lid } if *lid == listing_id => Some(Ok(())),
                ListingEvent::AuthorizationDenied { reason, .. } => {
                    Some(Err(anyhow::anyhow!("Authorization denied: {}", reason)))
                }
                _ => None,
            })
            .result()
        },
    )
    .await
    .map_err(|e| {
        FieldError::new(
            format!("Failed to approve listing: {}", e),
            juniper::Value::null(),
        )
    })?;

    // Query result from database (read queries OK in edges)
    let listing = Listing::find_by_id(listing_id, &ctx.db_pool)
        .await
        .map_err(|_| FieldError::new("Failed to fetch listing", juniper::Value::null()))?;

    Ok(ListingType::from(listing))
}

/// Edit and approve a listing (fix AI mistakes or improve user-submitted content)
/// Following seesaw pattern: dispatch request event, await fact event
pub async fn edit_and_approve_listing(
    ctx: &GraphQLContext,
    listing_id: Uuid,
    input: EditListingInput,
) -> FieldResult<ListingType> {
    info!(listing_id = %listing_id, title = ?input.title, "Editing and approving listing (triggers matching)");

    // Get user info (authorization will be checked in effect)
    let user = ctx
        .auth_user
        .as_ref()
        .ok_or_else(|| FieldError::new("Authentication required", juniper::Value::null()))?;

    // Convert to typed ID
    let listing_id = ListingId::from_uuid(listing_id);

    // Dispatch request event and await ListingApproved fact event
    dispatch_request(
        ListingEvent::EditAndApproveListingRequested {
            listing_id,
            title: input.title,
            description: input.description,
            description_markdown: input.description_markdown,
            tldr: input.tldr,
            contact_info: None, // Contact info not in EditListingInput for listings
            urgency: input.urgency,
            location: input.location,
            requested_by: user.member_id,
            is_admin: user.is_admin,
        },
        &ctx.bus,
        |m| {
            m.try_match(|e: &ListingEvent| match e {
                ListingEvent::ListingApproved { listing_id: lid } if *lid == listing_id => Some(Ok(())),
                ListingEvent::AuthorizationDenied { reason, .. } => {
                    Some(Err(anyhow::anyhow!("Authorization denied: {}", reason)))
                }
                _ => None,
            })
            .result()
        },
    )
    .await
    .map_err(|e| {
        FieldError::new(
            format!("Failed to edit and approve listing: {}", e),
            juniper::Value::null(),
        )
    })?;

    // Query result from database (read queries OK in edges)
    let listing = Listing::find_by_id(listing_id, &ctx.db_pool)
        .await
        .map_err(|_| FieldError::new("Failed to fetch listing", juniper::Value::null()))?;

    Ok(ListingType::from(listing))
}

/// Reject a listing (hide forever)
/// Following seesaw pattern: dispatch request event, await fact event
pub async fn reject_listing(ctx: &GraphQLContext, listing_id: Uuid, reason: String) -> FieldResult<bool> {
    info!(listing_id = %listing_id, reason = %reason, "Rejecting listing");

    // Get user info (authorization will be checked in effect)
    let user = ctx
        .auth_user
        .as_ref()
        .ok_or_else(|| FieldError::new("Authentication required", juniper::Value::null()))?;

    // Convert to typed ID
    let listing_id = ListingId::from_uuid(listing_id);

    // Dispatch request event and await ListingRejected fact event
    dispatch_request(
        ListingEvent::RejectListingRequested {
            listing_id,
            reason,
            requested_by: user.member_id,
            is_admin: user.is_admin,
        },
        &ctx.bus,
        |m| {
            m.try_match(|e: &ListingEvent| match e {
                ListingEvent::ListingRejected { listing_id: lid, .. } if *lid == listing_id => {
                    Some(Ok(()))
                }
                ListingEvent::AuthorizationDenied { reason, .. } => {
                    Some(Err(anyhow::anyhow!("Authorization denied: {}", reason)))
                }
                _ => None,
            })
            .result()
        },
    )
    .await
    .map_err(|e| {
        FieldError::new(
            format!("Failed to reject listing: {}", e),
            juniper::Value::null(),
        )
    })?;

    Ok(true)
}

/// Submit a resource link (URL) from the public for scraping
/// Returns job_id immediately - user can check progress
/// Following seesaw pattern: dispatch request event, await result event with job_id
pub async fn submit_resource_link(
    ctx: &GraphQLContext,
    input: SubmitResourceLinkInput,
) -> FieldResult<SubmitResourceLinkResult> {
    info!(url = %input.url, context = ?input.context, "Submitting resource link for scraping");

    // URL validation moved to effect (SubmitResourceLinkRequested handler)
    // Edge just dispatches the event

    // Dispatch request event and await WebsiteCreatedFromLink or WebsitePendingApproval event
    // This follows proper seesaw encapsulation - job_id is created in the effect
    let (job_id, status, message) = dispatch_request(
        ListingEvent::SubmitResourceLinkRequested {
            url: input.url.clone(),
            context: input.context,
            submitter_contact: input.submitter_contact,
        },
        &ctx.bus,
        |m| {
            m.try_match(|e: &ListingEvent| match e {
                // Website approved - scraping started with job_id
                ListingEvent::WebsiteCreatedFromLink { job_id, .. } => {
                    Some(Ok((
                        *job_id,
                        "pending".to_string(),
                        "Resource submitted successfully! We'll process it shortly.".to_string()
                    )))
                }
                // Website pending approval - return website_id as job_id placeholder
                ListingEvent::WebsitePendingApproval { website_id, .. } => {
                    Some(Ok((
                        JobId::from_uuid(website_id.into_uuid()),
                        "pending_review".to_string(),
                        "Resource submitted! The website is pending admin approval before we can scrape it.".to_string()
                    )))
                }
                _ => None,
            })
            .result()
        },

    )
    .await
    .map_err(|e| {
        FieldError::new(
            format!("Failed to submit resource link: {}", e),
            juniper::Value::null(),
        )
    })?;

    Ok(SubmitResourceLinkResult {
        job_id: job_id.into_uuid(),
        status,
        message,
    })
}

/// Delete a listing (admin only)
pub async fn delete_listing(
    ctx: &GraphQLContext,
    listing_id: Uuid,
) -> FieldResult<bool> {
    info!(listing_id = %listing_id, "Delete listing requested");

    // Get user info (auth check moved to effect)
    let user = ctx
        .auth_user
        .as_ref()
        .ok_or_else(|| FieldError::new("Authentication required", juniper::Value::null()))?;

    // Convert to typed ID
    let listing_id = ListingId::from_uuid(listing_id);

    // Dispatch request event - effect will handle authorization and deletion
    dispatch_request(
        ListingEvent::DeleteListingRequested {
            listing_id,
            requested_by: user.member_id,
            is_admin: user.is_admin,
        },
        &ctx.bus,
        |m| {
            m.try_match(|e: &ListingEvent| match e {
                ListingEvent::ListingDeleted { listing_id: _ } => Some(Ok(true)),
                ListingEvent::AuthorizationDenied { .. } => {
                    Some(Err(anyhow::anyhow!("Only administrators can delete listings")))
                }
                _ => None,
            })
            .result()
        },
    )
    .await
    .map_err(|e| FieldError::new(format!("Failed to delete listing: {}", e), juniper::Value::null()))
}

/// Repost a listing (create new post for existing active listing)
pub async fn repost_listing(
    ctx: &GraphQLContext,
    listing_id: Uuid,
) -> FieldResult<crate::domains::organization::data::post_types::RepostResult> {
    use crate::domains::organization::data::PostData;
    
    info!(listing_id = %listing_id, "Reposting listing");

    let user = ctx
        .auth_user
        .as_ref()
        .ok_or_else(|| FieldError::new("Authentication required", juniper::Value::null()))?;

    let listing_id = ListingId::from_uuid(listing_id);

    dispatch_request(
        ListingEvent::RepostListingRequested {
            listing_id,
            requested_by: user.member_id,
            is_admin: user.is_admin,
        },
        &ctx.bus,
        |m| {
            m.try_match(|e: &ListingEvent| match e {
                ListingEvent::PostCreated { post_id, .. } => {
                    Some(Ok(*post_id))
                }
                ListingEvent::AuthorizationDenied { .. } => {
                    Some(Err(anyhow::anyhow!("Only administrators can repost listings")))
                }
                _ => None,
            })
            .result()
        },
    )
    .await
    .map_err(|e| FieldError::new(format!("Failed to repost listing: {}", e), juniper::Value::null()))?;

    // Fetch the created post
    let post = crate::domains::organization::models::Post::find_by_listing_id(listing_id, &ctx.db_pool)
        .await
        .map_err(|e| FieldError::new(format!("Failed to fetch post: {}", e), juniper::Value::null()))?
        .into_iter()
        .next()
        .ok_or_else(|| FieldError::new("Post not found after creation", juniper::Value::null()))?;

    Ok(crate::domains::organization::data::post_types::RepostResult {
        post: PostData::from(post),
        message: "Listing reposted successfully".to_string(),
    })
}

/// Expire a post
pub async fn expire_post(
    ctx: &GraphQLContext,
    post_id: Uuid,
) -> FieldResult<crate::domains::organization::data::PostData> {
    use crate::common::PostId;
    use crate::domains::organization::data::PostData;
    
    info!(post_id = %post_id, "Expiring post");

    let user = ctx
        .auth_user
        .as_ref()
        .ok_or_else(|| FieldError::new("Authentication required", juniper::Value::null()))?;

    let post_id = PostId::from_uuid(post_id);

    dispatch_request(
        ListingEvent::ExpirePostRequested {
            post_id,
            requested_by: user.member_id,
            is_admin: user.is_admin,
        },
        &ctx.bus,
        |m| {
            m.try_match(|e: &ListingEvent| match e {
                ListingEvent::PostExpired { post_id: expired_id } if *expired_id == post_id => {
                    Some(Ok(()))
                }
                ListingEvent::AuthorizationDenied { .. } => {
                    Some(Err(anyhow::anyhow!("Only administrators can expire posts")))
                }
                _ => None,
            })
            .result()
        },
    )
    .await
    .map_err(|e| FieldError::new(format!("Failed to expire post: {}", e), juniper::Value::null()))?;

    // Fetch the updated post
    let post = crate::domains::organization::models::Post::find_by_id(post_id, &ctx.db_pool)
        .await
        .map_err(|e| FieldError::new(format!("Failed to fetch post: {}", e), juniper::Value::null()))?
        .ok_or_else(|| FieldError::new("Post not found after expiring", juniper::Value::null()))?;

    Ok(PostData::from(post))
}

/// Archive a post
pub async fn archive_post(
    ctx: &GraphQLContext,
    post_id: Uuid,
) -> FieldResult<crate::domains::organization::data::PostData> {
    use crate::common::PostId;
    use crate::domains::organization::data::PostData;
    
    info!(post_id = %post_id, "Archiving post");

    let user = ctx
        .auth_user
        .as_ref()
        .ok_or_else(|| FieldError::new("Authentication required", juniper::Value::null()))?;

    let post_id = PostId::from_uuid(post_id);

    dispatch_request(
        ListingEvent::ArchivePostRequested {
            post_id,
            requested_by: user.member_id,
            is_admin: user.is_admin,
        },
        &ctx.bus,
        |m| {
            m.try_match(|e: &ListingEvent| match e {
                ListingEvent::PostArchived { post_id: archived_id } if *archived_id == post_id => {
                    Some(Ok(()))
                }
                ListingEvent::AuthorizationDenied { .. } => {
                    Some(Err(anyhow::anyhow!("Only administrators can archive posts")))
                }
                _ => None,
            })
            .result()
        },
    )
    .await
    .map_err(|e| FieldError::new(format!("Failed to archive post: {}", e), juniper::Value::null()))?;

    // Fetch the updated post
    let post = crate::domains::organization::models::Post::find_by_id(post_id, &ctx.db_pool)
        .await
        .map_err(|e| FieldError::new(format!("Failed to fetch post: {}", e), juniper::Value::null()))?
        .ok_or_else(|| FieldError::new("Post not found after archiving", juniper::Value::null()))?;

    Ok(PostData::from(post))
}

/// Track post view (analytics)
pub async fn track_post_view(ctx: &GraphQLContext, post_id: Uuid) -> FieldResult<bool> {
    use crate::common::PostId;
    
    let post_id = PostId::from_uuid(post_id);

    dispatch_request(
        ListingEvent::PostViewedRequested { post_id },
        &ctx.bus,
        |m| {
            m.try_match(|e: &ListingEvent| match e {
                ListingEvent::PostViewed { post_id: viewed_id } if *viewed_id == post_id => {
                    Some(Ok(true))
                }
                _ => None,
            })
            .result()
        },
    )
    .await
    .map_err(|e| FieldError::new(format!("Failed to track post view: {}", e), juniper::Value::null()))
}

/// Track post click (analytics)
pub async fn track_post_click(ctx: &GraphQLContext, post_id: Uuid) -> FieldResult<bool> {
    use crate::common::PostId;
    
    let post_id = PostId::from_uuid(post_id);

    dispatch_request(
        ListingEvent::PostClickedRequested { post_id },
        &ctx.bus,
        |m| {
            m.try_match(|e: &ListingEvent| match e {
                ListingEvent::PostClicked { post_id: clicked_id } if *clicked_id == post_id => {
                    Some(Ok(true))
                }
                _ => None,
            })
            .result()
        },
    )
    .await
    .map_err(|e| FieldError::new(format!("Failed to track post click: {}", e), juniper::Value::null()))
}

// =============================================================================
// Domain Management Mutations
// =============================================================================

/// Approve a website for crawling (admin only)
/// Direct database operation - no event bus needed for approval workflow
pub async fn approve_domain(
    ctx: &GraphQLContext,
    domain_id: String,
) -> FieldResult<crate::domains::organization::data::SourceData> {
    use crate::common::WebsiteId;
    use crate::domains::scraping::models::Website;
    use uuid::Uuid;

    info!(domain_id = %domain_id, "Approving website");

    // Get user info - must be admin
    let user = ctx
        .auth_user
        .as_ref()
        .ok_or_else(|| FieldError::new("Authentication required", juniper::Value::null()))?;

    if !user.is_admin {
        return Err(FieldError::new(
            "Admin authorization required",
            juniper::Value::null(),
        ));
    }

    // Parse website ID
    let uuid = Uuid::parse_str(&domain_id)
        .map_err(|_| FieldError::new("Invalid website ID", juniper::Value::null()))?;
    let website_id = WebsiteId::from_uuid(uuid);

    // Approve using model method
    let website = Website::approve(website_id, user.member_id, &ctx.db_pool)
        .await
        .map_err(|e| {
            FieldError::new(
                format!("Failed to approve website: {}", e),
                juniper::Value::null(),
            )
        })?;

    Ok(crate::domains::organization::data::SourceData::from(website))
}

/// Reject a domain submission (admin only)
/// Direct database operation - no event bus needed for approval workflow
pub async fn reject_domain(
    ctx: &GraphQLContext,
    domain_id: String,
    reason: String,
) -> FieldResult<crate::domains::organization::data::SourceData> {
    use crate::domains::scraping::models::Website;
    use uuid::Uuid;

    info!(domain_id = %domain_id, reason = %reason, "Rejecting website");

    // Get user info - must be admin
    let user = ctx
        .auth_user
        .as_ref()
        .ok_or_else(|| FieldError::new("Authentication required", juniper::Value::null()))?;

    if !user.is_admin {
        return Err(FieldError::new(
            "Admin authorization required",
            juniper::Value::null(),
        ));
    }

    // Parse website ID
    let uuid = Uuid::parse_str(&domain_id)
        .map_err(|_| FieldError::new("Invalid website ID", juniper::Value::null()))?;
    let website_id = WebsiteId::from_uuid(uuid);

    // Reject using model method
    let website = Website::reject(website_id, user.member_id, reason, &ctx.db_pool)
        .await
        .map_err(|e| {
            FieldError::new(
                format!("Failed to reject website: {}", e),
                juniper::Value::null(),
            )
        })?;

    Ok(crate::domains::organization::data::SourceData::from(website))
}

/// Suspend a website (admin only)
/// Direct database operation - no event bus needed for approval workflow
pub async fn suspend_domain(
    ctx: &GraphQLContext,
    domain_id: String,
    reason: String,
) -> FieldResult<crate::domains::organization::data::SourceData> {
    use crate::common::WebsiteId;
    use crate::domains::scraping::models::Website;
    use uuid::Uuid;

    info!(domain_id = %domain_id, reason = %reason, "Suspending website");

    // Get user info - must be admin
    let user = ctx
        .auth_user
        .as_ref()
        .ok_or_else(|| FieldError::new("Authentication required", juniper::Value::null()))?;

    if !user.is_admin {
        return Err(FieldError::new(
            "Admin authorization required",
            juniper::Value::null(),
        ));
    }

    // Parse website ID
    let uuid = Uuid::parse_str(&domain_id)
        .map_err(|_| FieldError::new("Invalid website ID", juniper::Value::null()))?;
    let website_id = WebsiteId::from_uuid(uuid);

    // Suspend using model method
    let website = Website::suspend(website_id, user.member_id, reason, &ctx.db_pool)
        .await
        .map_err(|e| {
            FieldError::new(
                format!("Failed to suspend website: {}", e),
                juniper::Value::null(),
            )
        })?;

    Ok(crate::domains::organization::data::SourceData::from(website))
}

/// Refresh a page snapshot by re-scraping (admin only)
/// Re-scrapes a specific domain snapshot to update listings when page content changes
pub async fn refresh_page_snapshot(
    ctx: &GraphQLContext,
    snapshot_id: String,
) -> FieldResult<ScrapeJobResult> {
    use crate::domains::scraping::models::WebsiteSnapshot;
    use uuid::Uuid;

    info!(snapshot_id = %snapshot_id, "Refreshing page snapshot");

    // Get user info - must be admin
    let user = ctx
        .auth_user
        .as_ref()
        .ok_or_else(|| FieldError::new("Authentication required", juniper::Value::null()))?;

    if !user.is_admin {
        return Err(FieldError::new(
            "Admin authorization required",
            juniper::Value::null(),
        ));
    }

    // Parse snapshot ID
    let snapshot_uuid = Uuid::parse_str(&snapshot_id)
        .map_err(|_| FieldError::new("Invalid snapshot ID", juniper::Value::null()))?;

    // Get the domain snapshot
    let snapshot = WebsiteSnapshot::find_by_id(&ctx.db_pool, snapshot_uuid)
        .await
        .map_err(|e| {
            FieldError::new(
                format!("Failed to find snapshot: {}", e),
                juniper::Value::null(),
            )
        })?;

    // Get the website to verify it's approved
    let website = crate::domains::scraping::models::Website::find_by_id(
        snapshot.get_website_id(),
        &ctx.db_pool,
    )
    .await
    .map_err(|e| {
        FieldError::new(
            format!("Failed to find website: {}", e),
            juniper::Value::null(),
        )
    })?;

    if website.status != "approved" {
        return Err(FieldError::new(
            "Website must be approved before refreshing",
            juniper::Value::null(),
        ));
    }

    // Trigger re-scrape by dispatching event (same as scrapeOrganization)
    let source_id = snapshot.get_website_id();
    let job_id = JobId::new();

    // Dispatch request event and await completion
    let result = dispatch_request(
        ListingEvent::ScrapeSourceRequested {
            source_id,
            job_id,
            requested_by: user.member_id,
            is_admin: user.is_admin,
        },
        &ctx.bus,
        |m| {
            m.try_match(|e: &ListingEvent| match e {
                // Success - scraping workflow complete
                ListingEvent::ListingsSynced {
                    source_id: synced_source_id,
                    job_id: synced_job_id,
                    new_count,
                    changed_count,
                    disappeared_count,
                } if *synced_source_id == source_id && *synced_job_id == job_id => {
                    Some(Ok((
                        "completed".to_string(),
                        format!(
                            "Refresh complete! Found {} new, {} changed, {} disappeared",
                            new_count, changed_count, disappeared_count
                        ),
                    )))
                }
                // Failure events
                ListingEvent::ScrapeFailed {
                    source_id: failed_source_id,
                    job_id: failed_job_id,
                    reason,
                } if *failed_source_id == source_id && *failed_job_id == job_id => {
                    Some(Err(anyhow::anyhow!("Scrape failed: {}", reason)))
                }
                ListingEvent::ExtractFailed {
                    source_id: failed_source_id,
                    job_id: failed_job_id,
                    reason,
                } if *failed_source_id == source_id && *failed_job_id == job_id => {
                    Some(Err(anyhow::anyhow!("Extraction failed: {}", reason)))
                }
                ListingEvent::SyncFailed {
                    source_id: failed_source_id,
                    job_id: failed_job_id,
                    reason,
                } if *failed_source_id == source_id && *failed_job_id == job_id => {
                    Some(Err(anyhow::anyhow!("Sync failed: {}", reason)))
                }
                ListingEvent::AuthorizationDenied {
                    user_id,
                    action,
                    reason,
                } if *user_id == user.member_id && action == "ScrapeSource" => {
                    Some(Err(anyhow::anyhow!("Authorization denied: {}", reason)))
                }
                _ => None,
            })
            .result()
        },
    )
    .await
    .map_err(|e| FieldError::new(format!("Refresh failed: {}", e), juniper::Value::null()))?;

    let (status, message) = result;

    Ok(ScrapeJobResult {
        job_id: job_id.into_uuid(),
        source_id: source_id.into_uuid(),
        status,
        message: Some(message),
    })
}

/// Submit a report for a listing (public or authenticated)
pub async fn report_listing(
    ctx: &GraphQLContext,
    listing_id: Uuid,
    reason: String,
    category: String,
    reporter_email: Option<String>,
) -> FieldResult<ListingReport> {
    let listing_id = ListingId::from_uuid(listing_id);

    // Get user context if authenticated
    let reported_by = ctx.auth_user.as_ref().map(|u| u.member_id);

    let report_id = Uuid::new_v4();
    let report_id_typed = ListingReportId::from_uuid(report_id);

    dispatch_request(
        ListingEvent::ReportListingRequested {
            listing_id,
            reported_by,
            reporter_email,
            reason,
            category,
        },
        &ctx.bus,
        |m| {
            m.try_match(|e: &ListingEvent| match e {
                ListingEvent::ListingReported { report_id: rid, .. } if *rid == report_id_typed => {
                    Some(Ok(()))
                }
                _ => None,
            })
            .result()
        },
    )
    .await
    .map_err(|e| FieldError::new(format!("Failed to report listing: {}", e), juniper::Value::null()))?;

    // Fetch the created report
    let report = ListingReportRecord::query_for_listing(listing_id, &ctx.db_pool)
        .await
        .map_err(|e| FieldError::new(format!("Failed to fetch report: {}", e), juniper::Value::null()))?
        .into_iter()
        .find(|r| r.id == report_id_typed)
        .ok_or_else(|| FieldError::new("Report not found after creation", juniper::Value::null()))?;

    Ok(report.into())
}

/// Resolve a report and optionally take action (admin only)
pub async fn resolve_report(
    ctx: &GraphQLContext,
    report_id: Uuid,
    resolution_notes: Option<String>,
    action_taken: String,
) -> FieldResult<bool> {
    let user = ctx
        .auth_user
        .as_ref()
        .ok_or_else(|| FieldError::new("Authentication required", juniper::Value::null()))?;
    let report_id = ListingReportId::from_uuid(report_id);

    dispatch_request(
        ListingEvent::ResolveReportRequested {
            report_id,
            resolved_by: user.member_id,
            resolution_notes,
            action_taken,
            is_admin: user.is_admin,
        },
        &ctx.bus,
        |m| {
            m.try_match(|e: &ListingEvent| match e {
                ListingEvent::ReportResolved { report_id: rid, .. } if *rid == report_id => {
                    Some(Ok(()))
                }
                ListingEvent::AuthorizationDenied { reason, .. } => {
                    Some(Err(anyhow::anyhow!("Authorization denied: {}", reason)))
                }
                _ => None,
            })
            .result()
        },
    )
    .await
    .map_err(|e| FieldError::new(format!("Failed to resolve report: {}", e), juniper::Value::null()))?;

    Ok(true)
}

/// Dismiss a report without taking action (admin only)
pub async fn dismiss_report(
    ctx: &GraphQLContext,
    report_id: Uuid,
    resolution_notes: Option<String>,
) -> FieldResult<bool> {
    let user = ctx
        .auth_user
        .as_ref()
        .ok_or_else(|| FieldError::new("Authentication required", juniper::Value::null()))?;
    let report_id = ListingReportId::from_uuid(report_id);

    dispatch_request(
        ListingEvent::DismissReportRequested {
            report_id,
            resolved_by: user.member_id,
            resolution_notes,
            is_admin: user.is_admin,
        },
        &ctx.bus,
        |m| {
            m.try_match(|e: &ListingEvent| match e {
                ListingEvent::ReportDismissed { report_id: rid } if *rid == report_id => {
                    Some(Ok(()))
                }
                ListingEvent::AuthorizationDenied { reason, .. } => {
                    Some(Err(anyhow::anyhow!("Authorization denied: {}", reason)))
                }
                _ => None,
            })
            .result()
        },
    )
    .await
    .map_err(|e| FieldError::new(format!("Failed to dismiss report: {}", e), juniper::Value::null()))?;

    Ok(true)
}
