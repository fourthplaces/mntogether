use crate::common::{JobId, PostId, MemberId, WebsiteId};
use crate::domains::posts::data::post_report::PostReport;
use crate::domains::posts::data::{
    EditPostInput, PostType, ScrapeJobResult, SubmitPostInput, SubmitResourceLinkInput,
    SubmitResourceLinkResult,
};
use crate::domains::posts::events::PostEvent;
use crate::domains::posts::models::post_report::{PostReportId, PostReportRecord};
use crate::domains::posts::models::Post;
use crate::server::graphql::context::GraphQLContext;
use juniper::{FieldError, FieldResult};
use seesaw_core::dispatch_request;
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

    // Dispatch request event and await completion (PostsSynced or failure)
    let result = dispatch_request(
        PostEvent::ScrapeSourceRequested {
            source_id,
            job_id,
            requested_by: user.member_id,
            is_admin: user.is_admin,
        },
        &ctx.bus,
        |m| {
            m.try_match(|e: &PostEvent| match e {
                // Success - scraping workflow complete
                PostEvent::PostsSynced {
                    source_id: synced_source_id,
                    job_id: synced_job_id,
                    new_count,
                    updated_count,
                    unchanged_count,
                } if *synced_source_id == source_id && *synced_job_id == job_id => Some(Ok((
                    "completed".to_string(),
                    format!(
                        "Scraping complete! Found {} new, {} updated, {} unchanged",
                        new_count, updated_count, unchanged_count
                    ),
                ))),
                // Failure events
                PostEvent::ScrapeFailed {
                    source_id: failed_source_id,
                    job_id: failed_job_id,
                    reason,
                } if *failed_source_id == source_id && *failed_job_id == job_id => {
                    Some(Err(anyhow::anyhow!("Scrape failed: {}", reason)))
                }
                PostEvent::ExtractFailed {
                    source_id: failed_source_id,
                    job_id: failed_job_id,
                    reason,
                } if *failed_source_id == source_id && *failed_job_id == job_id => {
                    Some(Err(anyhow::anyhow!("Extraction failed: {}", reason)))
                }
                PostEvent::SyncFailed {
                    source_id: failed_source_id,
                    job_id: failed_job_id,
                    reason,
                } if *failed_source_id == source_id && *failed_job_id == job_id => {
                    Some(Err(anyhow::anyhow!("Sync failed: {}", reason)))
                }
                PostEvent::AuthorizationDenied {
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
pub async fn submit_post(
    ctx: &GraphQLContext,
    input: SubmitPostInput,
    member_id: Option<Uuid>,
    ip_address: Option<String>,
) -> FieldResult<PostType> {
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

    // Dispatch request event and await PostEntryCreated fact event
    let post_id = dispatch_request(
        PostEvent::SubmitListingRequested {
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
            m.try_match(|e: &PostEvent| match e {
                PostEvent::PostEntryCreated {
                    post_id,
                    submission_type,
                    ..
                } if submission_type == "user_submitted" => Some(Ok(*post_id)),
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
    let post = Post::find_by_id(post_id, &ctx.db_pool)
        .await
        .map_err(|_| FieldError::new("Failed to fetch listing", juniper::Value::null()))?
        .ok_or_else(|| FieldError::new("Listing not found", juniper::Value::null()))?;

    Ok(PostType::from(post))
}

/// Approve a listing (human-in-the-loop)
/// Following seesaw pattern: dispatch request event, await fact event
pub async fn approve_post(ctx: &GraphQLContext, post_id: Uuid) -> FieldResult<PostType> {
    info!(post_id = %post_id, "Approving listing (triggers matching)");

    // Get user info (authorization will be checked in effect)
    let user = ctx
        .auth_user
        .as_ref()
        .ok_or_else(|| FieldError::new("Authentication required", juniper::Value::null()))?;

    // Convert to typed ID
    let post_id = PostId::from_uuid(post_id);

    // Dispatch request event and await PostApproved fact event
    dispatch_request(
        PostEvent::ApproveListingRequested {
            post_id,
            requested_by: user.member_id,
            is_admin: user.is_admin,
        },
        &ctx.bus,
        |m| {
            m.try_match(|e: &PostEvent| match e {
                PostEvent::PostApproved { post_id: lid } if *lid == post_id => {
                    Some(Ok(()))
                }
                PostEvent::AuthorizationDenied { reason, .. } => {
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
    let post = Post::find_by_id(post_id, &ctx.db_pool)
        .await
        .map_err(|_| FieldError::new("Failed to fetch listing", juniper::Value::null()))?
        .ok_or_else(|| FieldError::new("Listing not found", juniper::Value::null()))?;

    Ok(PostType::from(post))
}

/// Edit and approve a listing (fix AI mistakes or improve user-submitted content)
/// Following seesaw pattern: dispatch request event, await fact event
pub async fn edit_and_approve_post(
    ctx: &GraphQLContext,
    post_id: Uuid,
    input: EditPostInput,
) -> FieldResult<PostType> {
    info!(post_id = %post_id, title = ?input.title, "Editing and approving listing (triggers matching)");

    // Get user info (authorization will be checked in effect)
    let user = ctx
        .auth_user
        .as_ref()
        .ok_or_else(|| FieldError::new("Authentication required", juniper::Value::null()))?;

    // Convert to typed ID
    let post_id = PostId::from_uuid(post_id);

    // Dispatch request event and await PostApproved fact event
    dispatch_request(
        PostEvent::EditAndApproveListingRequested {
            post_id,
            title: input.title,
            description: input.description,
            description_markdown: input.description_markdown,
            tldr: input.tldr,
            contact_info: None, // Contact info not in EditPostInput for listings
            urgency: input.urgency,
            location: input.location,
            requested_by: user.member_id,
            is_admin: user.is_admin,
        },
        &ctx.bus,
        |m| {
            m.try_match(|e: &PostEvent| match e {
                PostEvent::PostApproved { post_id: lid } if *lid == post_id => {
                    Some(Ok(()))
                }
                PostEvent::AuthorizationDenied { reason, .. } => {
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
    let post = Post::find_by_id(post_id, &ctx.db_pool)
        .await
        .map_err(|_| FieldError::new("Failed to fetch listing", juniper::Value::null()))?
        .ok_or_else(|| FieldError::new("Listing not found", juniper::Value::null()))?;

    Ok(PostType::from(post))
}

/// Reject a listing (hide forever)
/// Following seesaw pattern: dispatch request event, await fact event
pub async fn reject_post(
    ctx: &GraphQLContext,
    post_id: Uuid,
    reason: String,
) -> FieldResult<bool> {
    info!(post_id = %post_id, reason = %reason, "Rejecting listing");

    // Get user info (authorization will be checked in effect)
    let user = ctx
        .auth_user
        .as_ref()
        .ok_or_else(|| FieldError::new("Authentication required", juniper::Value::null()))?;

    // Convert to typed ID
    let post_id = PostId::from_uuid(post_id);

    // Dispatch request event and await PostRejected fact event
    dispatch_request(
        PostEvent::RejectListingRequested {
            post_id,
            reason,
            requested_by: user.member_id,
            is_admin: user.is_admin,
        },
        &ctx.bus,
        |m| {
            m.try_match(|e: &PostEvent| match e {
                PostEvent::PostRejected {
                    post_id: lid, ..
                } if *lid == post_id => Some(Ok(())),
                PostEvent::AuthorizationDenied { reason, .. } => {
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
        PostEvent::SubmitResourceLinkRequested {
            url: input.url.clone(),
            context: input.context,
            submitter_contact: input.submitter_contact,
        },
        &ctx.bus,
        |m| {
            m.try_match(|e: &PostEvent| match e {
                // Website approved - scraping started with job_id
                PostEvent::WebsiteCreatedFromLink { job_id, .. } => {
                    Some(Ok((
                        *job_id,
                        "pending".to_string(),
                        "Resource submitted successfully! We'll process it shortly.".to_string()
                    )))
                }
                // Website pending approval - return website_id as job_id placeholder
                PostEvent::WebsitePendingApproval { website_id, .. } => {
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
pub async fn delete_post(ctx: &GraphQLContext, post_id: Uuid) -> FieldResult<bool> {
    info!(post_id = %post_id, "Delete listing requested");

    // Get user info (auth check moved to effect)
    let user = ctx
        .auth_user
        .as_ref()
        .ok_or_else(|| FieldError::new("Authentication required", juniper::Value::null()))?;

    // Convert to typed ID
    let post_id = PostId::from_uuid(post_id);

    // Dispatch request event - effect will handle authorization and deletion
    dispatch_request(
        PostEvent::DeletePostRequested {
            post_id,
            requested_by: user.member_id,
            is_admin: user.is_admin,
        },
        &ctx.bus,
        |m| {
            m.try_match(|e: &PostEvent| match e {
                PostEvent::PostDeleted { post_id: _ } => Some(Ok(true)),
                PostEvent::AuthorizationDenied { .. } => Some(Err(anyhow::anyhow!(
                    "Only administrators can delete listings"
                ))),
                _ => None,
            })
            .result()
        },
    )
    .await
    .map_err(|e| {
        FieldError::new(
            format!("Failed to delete listing: {}", e),
            juniper::Value::null(),
        )
    })
}

/// Repost a listing (create new post for existing active listing)
/// Note: This function is deprecated - the announcement model was removed
pub async fn repost_post(
    _ctx: &GraphQLContext,
    _post_id: Uuid,
) -> FieldResult<crate::domains::posts::data::types::RepostResult> {
    // The announcement model was removed, so reposting is no longer supported
    Err(FieldError::new(
        "Reposting is not currently supported",
        juniper::Value::null(),
    ))
}

/// Expire a post
pub async fn expire_post(
    ctx: &GraphQLContext,
    post_id: Uuid,
) -> FieldResult<crate::domains::posts::data::PostData> {
    use crate::common::PostId;
    use crate::domains::posts::data::PostData;

    info!(post_id = %post_id, "Expiring post");

    let user = ctx
        .auth_user
        .as_ref()
        .ok_or_else(|| FieldError::new("Authentication required", juniper::Value::null()))?;

    let post_id = PostId::from_uuid(post_id);

    dispatch_request(
        PostEvent::ExpirePostRequested {
            post_id,
            requested_by: user.member_id,
            is_admin: user.is_admin,
        },
        &ctx.bus,
        |m| {
            m.try_match(|e: &PostEvent| match e {
                PostEvent::PostExpired {
                    post_id: expired_id,
                } if *expired_id == post_id => Some(Ok(())),
                PostEvent::AuthorizationDenied { .. } => {
                    Some(Err(anyhow::anyhow!("Only administrators can expire posts")))
                }
                _ => None,
            })
            .result()
        },
    )
    .await
    .map_err(|e| {
        FieldError::new(
            format!("Failed to expire post: {}", e),
            juniper::Value::null(),
        )
    })?;

    // Fetch the updated post
    let post = crate::domains::posts::models::Post::find_by_id(post_id, &ctx.db_pool)
        .await
        .map_err(|e| {
            FieldError::new(
                format!("Failed to fetch post: {}", e),
                juniper::Value::null(),
            )
        })?
        .ok_or_else(|| FieldError::new("Post not found after expiring", juniper::Value::null()))?;

    Ok(PostData::from(post))
}

/// Archive a post
pub async fn archive_post(
    ctx: &GraphQLContext,
    post_id: Uuid,
) -> FieldResult<crate::domains::posts::data::PostData> {
    use crate::common::PostId;
    use crate::domains::posts::data::PostData;

    info!(post_id = %post_id, "Archiving post");

    let user = ctx
        .auth_user
        .as_ref()
        .ok_or_else(|| FieldError::new("Authentication required", juniper::Value::null()))?;

    let post_id = PostId::from_uuid(post_id);

    dispatch_request(
        PostEvent::ArchivePostRequested {
            post_id,
            requested_by: user.member_id,
            is_admin: user.is_admin,
        },
        &ctx.bus,
        |m| {
            m.try_match(|e: &PostEvent| match e {
                PostEvent::PostArchived {
                    post_id: archived_id,
                } if *archived_id == post_id => Some(Ok(())),
                PostEvent::AuthorizationDenied { .. } => Some(Err(anyhow::anyhow!(
                    "Only administrators can archive posts"
                ))),
                _ => None,
            })
            .result()
        },
    )
    .await
    .map_err(|e| {
        FieldError::new(
            format!("Failed to archive post: {}", e),
            juniper::Value::null(),
        )
    })?;

    // Fetch the updated post
    let post = crate::domains::posts::models::Post::find_by_id(post_id, &ctx.db_pool)
        .await
        .map_err(|e| {
            FieldError::new(
                format!("Failed to fetch post: {}", e),
                juniper::Value::null(),
            )
        })?
        .ok_or_else(|| FieldError::new("Post not found after archiving", juniper::Value::null()))?;

    Ok(PostData::from(post))
}

/// Track post view (analytics)
pub async fn track_post_view(ctx: &GraphQLContext, post_id: Uuid) -> FieldResult<bool> {
    use crate::common::PostId;

    let post_id = PostId::from_uuid(post_id);

    dispatch_request(
        PostEvent::PostViewedRequested { post_id },
        &ctx.bus,
        |m| {
            m.try_match(|e: &PostEvent| match e {
                PostEvent::PostViewed { post_id: viewed_id } if *viewed_id == post_id => {
                    Some(Ok(true))
                }
                _ => None,
            })
            .result()
        },
    )
    .await
    .map_err(|e| {
        FieldError::new(
            format!("Failed to track post view: {}", e),
            juniper::Value::null(),
        )
    })
}

/// Track post click (analytics)
pub async fn track_post_click(ctx: &GraphQLContext, post_id: Uuid) -> FieldResult<bool> {
    use crate::common::PostId;

    let post_id = PostId::from_uuid(post_id);

    dispatch_request(
        PostEvent::PostClickedRequested { post_id },
        &ctx.bus,
        |m| {
            m.try_match(|e: &PostEvent| match e {
                PostEvent::PostClicked {
                    post_id: clicked_id,
                } if *clicked_id == post_id => Some(Ok(true)),
                _ => None,
            })
            .result()
        },
    )
    .await
    .map_err(|e| {
        FieldError::new(
            format!("Failed to track post click: {}", e),
            juniper::Value::null(),
        )
    })
}

// Re-export website mutations from the website domain
pub use crate::domains::website::edges::mutation::{
    approve_website, crawl_website, refresh_page_snapshot, reject_website, suspend_website,
};

/// Submit a report for a listing (public or authenticated)
pub async fn report_post(
    ctx: &GraphQLContext,
    post_id: Uuid,
    reason: String,
    category: String,
    reporter_email: Option<String>,
) -> FieldResult<PostReport> {
    let post_id = PostId::from_uuid(post_id);

    // Get user context if authenticated
    let reported_by = ctx.auth_user.as_ref().map(|u| u.member_id);

    let report_id = Uuid::new_v4();
    let report_id_typed = PostReportId::from_uuid(report_id);

    dispatch_request(
        PostEvent::ReportListingRequested {
            post_id,
            reported_by,
            reporter_email,
            reason,
            category,
        },
        &ctx.bus,
        |m| {
            m.try_match(|e: &PostEvent| match e {
                PostEvent::PostReported { report_id: rid, .. } if *rid == report_id_typed => {
                    Some(Ok(()))
                }
                _ => None,
            })
            .result()
        },
    )
    .await
    .map_err(|e| {
        FieldError::new(
            format!("Failed to report listing: {}", e),
            juniper::Value::null(),
        )
    })?;

    // Fetch the created report
    let report = PostReportRecord::query_for_post(post_id, &ctx.db_pool)
        .await
        .map_err(|e| {
            FieldError::new(
                format!("Failed to fetch report: {}", e),
                juniper::Value::null(),
            )
        })?
        .into_iter()
        .find(|r| r.id == report_id_typed)
        .ok_or_else(|| {
            FieldError::new("Report not found after creation", juniper::Value::null())
        })?;

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
    let report_id = PostReportId::from_uuid(report_id);

    dispatch_request(
        PostEvent::ResolveReportRequested {
            report_id,
            resolved_by: user.member_id,
            resolution_notes,
            action_taken,
            is_admin: user.is_admin,
        },
        &ctx.bus,
        |m| {
            m.try_match(|e: &PostEvent| match e {
                PostEvent::ReportResolved { report_id: rid, .. } if *rid == report_id => {
                    Some(Ok(()))
                }
                PostEvent::AuthorizationDenied { reason, .. } => {
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
            format!("Failed to resolve report: {}", e),
            juniper::Value::null(),
        )
    })?;

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
    let report_id = PostReportId::from_uuid(report_id);

    dispatch_request(
        PostEvent::DismissReportRequested {
            report_id,
            resolved_by: user.member_id,
            resolution_notes,
            is_admin: user.is_admin,
        },
        &ctx.bus,
        |m| {
            m.try_match(|e: &PostEvent| match e {
                PostEvent::ReportDismissed { report_id: rid } if *rid == report_id => {
                    Some(Ok(()))
                }
                PostEvent::AuthorizationDenied { reason, .. } => {
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
            format!("Failed to dismiss report: {}", e),
            juniper::Value::null(),
        )
    })?;

    Ok(true)
}

/// Result type for discovery search
#[derive(Debug, Clone, juniper::GraphQLObject)]
pub struct DiscoverySearchResult {
    pub queries_run: i32,
    pub total_results: i32,
    pub websites_created: i32,
}

/// Run discovery search manually (admin only)
/// Executes all static discovery queries via Tavily and creates pending websites
pub async fn run_discovery_search(ctx: &GraphQLContext) -> FieldResult<DiscoverySearchResult> {
    // Verify admin access
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

    info!(
        requested_by = %user.member_id,
        "Admin triggering manual discovery search"
    );

    // Get config for Tavily API key
    let config = crate::config::Config::from_env().map_err(|e| {
        FieldError::new(
            format!("Failed to load config: {}", e),
            juniper::Value::null(),
        )
    })?;

    // Create Tavily client
    let search_service = crate::kernel::TavilyClient::new(config.tavily_api_key).map_err(|e| {
        FieldError::new(
            format!("Failed to create search client: {}", e),
            juniper::Value::null(),
        )
    })?;

    // Run discovery searches
    let result =
        crate::domains::posts::effects::run_discovery_searches(&search_service, &ctx.db_pool)
            .await
            .map_err(|e| {
                FieldError::new(
                    format!("Discovery search failed: {}", e),
                    juniper::Value::null(),
                )
            })?;

    info!(
        queries_run = result.queries_run,
        total_results = result.total_results,
        websites_created = result.websites_created,
        "Manual discovery search completed"
    );

    Ok(DiscoverySearchResult {
        queries_run: result.queries_run as i32,
        total_results: result.total_results as i32,
        websites_created: result.websites_created as i32,
    })
}

/// Generate embedding for a single post (admin only)
pub async fn generate_post_embedding(ctx: &GraphQLContext, post_id: Uuid) -> FieldResult<bool> {
    // Verify admin access
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

    let post_id = PostId::from_uuid(post_id);

    info!(post_id = %post_id.as_uuid(), "Generating embedding for post");

    dispatch_request(
        PostEvent::GeneratePostEmbeddingRequested { post_id },
        &ctx.bus,
        |m| {
            m.try_match(|e: &PostEvent| match e {
                PostEvent::PostEmbeddingGenerated {
                    post_id: gen_id, ..
                } if *gen_id == post_id => Some(Ok(true)),
                PostEvent::ListingEmbeddingFailed {
                    post_id: fail_id,
                    reason,
                } if *fail_id == post_id => Some(Err(anyhow::anyhow!("Embedding failed: {}", reason))),
                _ => None,
            })
            .result()
        },
    )
    .await
    .map_err(|e| {
        FieldError::new(
            format!("Failed to generate embedding: {}", e),
            juniper::Value::null(),
        )
    })
}

/// Result type for deduplication
#[derive(Debug, Clone, juniper::GraphQLObject)]
pub struct DeduplicationResult {
    pub job_id: Uuid,
    pub duplicates_found: i32,
    pub posts_merged: i32,
    pub posts_deleted: i32,
}

/// Deduplicate posts using embedding similarity (admin only)
/// Finds posts with similar embeddings and merges them (keeps oldest, deletes others)
pub async fn deduplicate_posts(
    ctx: &GraphQLContext,
    similarity_threshold: Option<f64>,
) -> FieldResult<DeduplicationResult> {
    // Verify admin access
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

    let job_id = JobId::new();
    let threshold = similarity_threshold.unwrap_or(0.95) as f32;

    info!(
        job_id = %job_id,
        similarity_threshold = %threshold,
        requested_by = %user.member_id,
        "Admin triggering post deduplication"
    );

    let result = dispatch_request(
        PostEvent::DeduplicatePostsRequested {
            job_id,
            similarity_threshold: threshold,
            requested_by: user.member_id,
            is_admin: user.is_admin,
        },
        &ctx.bus,
        |m| {
            m.try_match(|e: &PostEvent| match e {
                PostEvent::PostsDeduplicated {
                    job_id: completed_job_id,
                    duplicates_found,
                    posts_merged,
                    posts_deleted,
                } if *completed_job_id == job_id => Some(Ok(DeduplicationResult {
                    job_id: job_id.into_uuid(),
                    duplicates_found: *duplicates_found as i32,
                    posts_merged: *posts_merged as i32,
                    posts_deleted: *posts_deleted as i32,
                })),
                PostEvent::DeduplicationFailed {
                    job_id: failed_job_id,
                    reason,
                } if *failed_job_id == job_id => {
                    Some(Err(anyhow::anyhow!("Deduplication failed: {}", reason)))
                }
                PostEvent::AuthorizationDenied { reason, .. } => {
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
            format!("Failed to deduplicate posts: {}", e),
            juniper::Value::null(),
        )
    })?;

    info!(
        duplicates_found = result.duplicates_found,
        posts_deleted = result.posts_deleted,
        "Post deduplication completed"
    );

    Ok(result)
}
