use crate::domains::listings::data::{EditListingInput, ListingType, ScrapeJobResult, SubmitListingInput, SubmitResourceLinkInput, SubmitResourceLinkResult};
use crate::common::{JobId, ListingId, MemberId, SourceId};
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
    let source_id = SourceId::from_uuid(source_id);
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

    // Dispatch request event and await OrganizationSourceCreatedFromLink event
    // This follows proper seesaw encapsulation - job_id is created in the effect
    let job_id = dispatch_request(
        ListingEvent::SubmitResourceLinkRequested {
            url: input.url.clone(),
            context: input.context,
            submitter_contact: input.submitter_contact,
        },
        &ctx.bus,
        |m| {
            m.try_match(|e: &ListingEvent| match e {
                ListingEvent::OrganizationSourceCreatedFromLink { job_id, .. } => {
                    Some(Ok(*job_id))
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
        status: "pending".to_string(),
        message: "Resource submitted successfully! We'll process it shortly.".to_string(),
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

/// Add a scrape URL to an organization source (admin only)
pub async fn add_organization_scrape_url(
    ctx: &GraphQLContext,
    source_id: Uuid,
    url: String,
) -> FieldResult<bool> {
    info!(source_id = %source_id, url = %url, "Add scrape URL requested");

    // Get user info (auth check moved to effect)
    let user = ctx
        .auth_user
        .as_ref()
        .ok_or_else(|| FieldError::new("Authentication required", juniper::Value::null()))?;

    // Convert to typed ID
    let source_id = SourceId::from_uuid(source_id);

    // Dispatch request event - effect will handle authorization and URL validation
    dispatch_request(
        ListingEvent::AddScrapeUrlRequested {
            source_id,
            url,
            requested_by: user.member_id,
            is_admin: user.is_admin,
        },
        &ctx.bus,
        |m| {
            m.try_match(|e: &ListingEvent| match e {
                ListingEvent::ScrapeUrlAdded { .. } => Some(Ok(true)),
                ListingEvent::AuthorizationDenied { .. } => {
                    Some(Err(anyhow::anyhow!("Only administrators can manage scrape URLs")))
                }
                _ => None,
            })
            .result()
        },
    )
    .await
    .map_err(|e| FieldError::new(format!("Failed to add scrape URL: {}", e), juniper::Value::null()))
}

/// Remove a scrape URL from an organization source (admin only)
pub async fn remove_organization_scrape_url(
    ctx: &GraphQLContext,
    source_id: Uuid,
    url: String,
) -> FieldResult<bool> {
    info!(source_id = %source_id, url = %url, "Remove scrape URL requested");

    // Get user info (auth check moved to effect)
    let user = ctx
        .auth_user
        .as_ref()
        .ok_or_else(|| FieldError::new("Authentication required", juniper::Value::null()))?;

    // Convert to typed ID
    let source_id = SourceId::from_uuid(source_id);

    // Dispatch request event - effect will handle authorization
    dispatch_request(
        ListingEvent::RemoveScrapeUrlRequested {
            source_id,
            url,
            requested_by: user.member_id,
            is_admin: user.is_admin,
        },
        &ctx.bus,
        |m| {
            m.try_match(|e: &ListingEvent| match e {
                ListingEvent::ScrapeUrlRemoved { .. } => Some(Ok(true)),
                ListingEvent::AuthorizationDenied { .. } => {
                    Some(Err(anyhow::anyhow!("Only administrators can manage scrape URLs")))
                }
                _ => None,
            })
            .result()
        },
    )
    .await
    .map_err(|e| FieldError::new(format!("Failed to remove scrape URL: {}", e), juniper::Value::null()))
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
