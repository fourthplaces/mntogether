use restate_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tracing::{info, warn};
use uuid::Uuid;

use crate::common::auth::restate_auth::require_admin;
use crate::common::{EmptyRequest, OrganizationId, WebsiteId};
use crate::domains::crawling::activities::{
    create_social_profiles_for_org, extract_and_create_organization, extract_organization_info,
};
use crate::domains::crawling::models::ExtractionPage;
use crate::domains::organization::restate::workflows::clean_up_org_posts::{
    CleanUpOrgPostsRequest, CleanUpOrgPostsWorkflowClient,
};
use crate::domains::organization::restate::workflows::extract_org_posts::{
    ExtractOrgPostsRequest, ExtractOrgPostsWorkflowClient,
};
use crate::domains::organization::models::Organization;
use crate::domains::posts::models::Post;
use crate::domains::posts::restate::services::posts::{PublicPostResult, PublicTagResult};
use crate::domains::source::models::Source;
use crate::domains::tag::models::Tag;
use crate::domains::website::models::Website;
use crate::impl_restate_serde;
use crate::kernel::ServerDeps;

// =============================================================================
// Request types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateOrganizationRequest {
    pub name: String,
    pub description: Option<String>,
}

impl_restate_serde!(CreateOrganizationRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateOrganizationRequest {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
}

impl_restate_serde!(UpdateOrganizationRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetOrganizationRequest {
    pub id: Uuid,
}

impl_restate_serde!(GetOrganizationRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteOrganizationRequest {
    pub id: Uuid,
}

impl_restate_serde!(DeleteOrganizationRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegenerateOrganizationRequest {
    pub id: Uuid,
}

impl_restate_serde!(RegenerateOrganizationRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApproveOrganizationRequest {
    pub id: Uuid,
}

impl_restate_serde!(ApproveOrganizationRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RejectOrganizationRequest {
    pub id: Uuid,
    pub reason: String,
}

impl_restate_serde!(RejectOrganizationRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuspendOrganizationRequest {
    pub id: Uuid,
    pub reason: String,
}

impl_restate_serde!(SuspendOrganizationRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegenerateOrganizationResult {
    pub organization_id: Option<String>,
    pub websites_processed: i64,
    pub status: String,
}

impl_restate_serde!(RegenerateOrganizationResult);

// =============================================================================
// Response types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganizationResult {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub status: String,
    pub website_count: i64,
    pub social_profile_count: i64,
    pub snapshot_count: i64,
    pub created_at: String,
    pub updated_at: String,
}

impl_restate_serde!(OrganizationResult);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganizationListResult {
    pub organizations: Vec<OrganizationResult>,
}

impl_restate_serde!(OrganizationListResult);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackfillResult {
    pub processed: i64,
    pub succeeded: i64,
    pub failed: i64,
}

impl_restate_serde!(BackfillResult);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganizationDetailResult {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub posts: Vec<PublicPostResult>,
}

impl_restate_serde!(OrganizationDetailResult);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledExtractionResult {
    pub orgs_triggered: i32,
    pub status: String,
}

impl_restate_serde!(ScheduledExtractionResult);

// =============================================================================
// Service definition
// =============================================================================

#[restate_sdk::service]
#[name = "Organizations"]
pub trait OrganizationsService {
    async fn public_list(req: EmptyRequest) -> Result<OrganizationListResult, HandlerError>;
    async fn public_get(
        req: GetOrganizationRequest,
    ) -> Result<OrganizationDetailResult, HandlerError>;
    async fn list(req: EmptyRequest) -> Result<OrganizationListResult, HandlerError>;
    async fn get(req: GetOrganizationRequest) -> Result<OrganizationResult, HandlerError>;
    async fn create(req: CreateOrganizationRequest) -> Result<OrganizationResult, HandlerError>;
    async fn update(req: UpdateOrganizationRequest) -> Result<OrganizationResult, HandlerError>;
    async fn delete(req: DeleteOrganizationRequest) -> Result<EmptyRequest, HandlerError>;
    async fn approve(req: ApproveOrganizationRequest) -> Result<OrganizationResult, HandlerError>;
    async fn reject(req: RejectOrganizationRequest) -> Result<OrganizationResult, HandlerError>;
    async fn suspend(req: SuspendOrganizationRequest) -> Result<OrganizationResult, HandlerError>;
    async fn regenerate(
        req: RegenerateOrganizationRequest,
    ) -> Result<RegenerateOrganizationResult, HandlerError>;
    async fn backfill_organizations(req: EmptyRequest) -> Result<BackfillResult, HandlerError>;
    async fn extract_org_posts(
        req: RegenerateOrganizationRequest,
    ) -> Result<RegenerateOrganizationResult, HandlerError>;
    async fn run_scheduled_extraction(
        req: EmptyRequest,
    ) -> Result<ScheduledExtractionResult, HandlerError>;
    async fn clean_up_org_posts(
        req: RegenerateOrganizationRequest,
    ) -> Result<RegenerateOrganizationResult, HandlerError>;
}

pub struct OrganizationsServiceImpl {
    deps: Arc<ServerDeps>,
}

impl OrganizationsServiceImpl {
    pub fn with_deps(deps: Arc<ServerDeps>) -> Self {
        Self { deps }
    }
}

impl OrganizationsService for OrganizationsServiceImpl {
    async fn public_list(
        &self,
        _ctx: Context<'_>,
        _req: EmptyRequest,
    ) -> Result<OrganizationListResult, HandlerError> {
        let orgs = Organization::find_approved(&self.deps.db_pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        let mut results = Vec::with_capacity(orgs.len());
        for org in orgs {
            results.push(OrganizationResult {
                id: org.id.to_string(),
                name: org.name,
                description: org.description,
                status: org.status,
                website_count: 0,
                social_profile_count: 0,
                snapshot_count: 0,
                created_at: org.created_at.to_rfc3339(),
                updated_at: org.updated_at.to_rfc3339(),
            });
        }

        Ok(OrganizationListResult {
            organizations: results,
        })
    }

    async fn public_get(
        &self,
        _ctx: Context<'_>,
        req: GetOrganizationRequest,
    ) -> Result<OrganizationDetailResult, HandlerError> {
        let org = Organization::find_by_id(OrganizationId::from(req.id), &self.deps.db_pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        if org.status != "approved" {
            return Err(TerminalError::new("Organization not found").into());
        }

        let posts = Post::find_by_organization_id(req.id, &self.deps.db_pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        // Batch-load public tags
        let post_ids: Vec<uuid::Uuid> = posts.iter().map(|p| p.id.into_uuid()).collect();
        let tag_rows = Tag::find_public_for_post_ids(&post_ids, &self.deps.db_pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        let mut tags_by_post: std::collections::HashMap<uuid::Uuid, Vec<PublicTagResult>> =
            std::collections::HashMap::new();
        for row in tag_rows {
            tags_by_post
                .entry(row.taggable_id)
                .or_default()
                .push(PublicTagResult {
                    kind: row.tag.kind,
                    value: row.tag.value,
                    display_name: row.tag.display_name,
                    color: row.tag.color,
                });
        }

        Ok(OrganizationDetailResult {
            id: org.id.to_string(),
            name: org.name,
            description: org.description,
            posts: posts
                .into_iter()
                .map(|p| {
                    let id = p.id.into_uuid();
                    PublicPostResult {
                        id,
                        title: p.title,
                        summary: p.summary,
                        description: p.description,
                        location: p.location,
                        source_url: p.source_url,
                        post_type: p.post_type,
                        category: p.category,
                        created_at: p.created_at.to_rfc3339(),
                        published_at: p.published_at.map(|dt| dt.to_rfc3339()),
                        tags: tags_by_post.remove(&id).unwrap_or_default(),
                        urgent_notes: Vec::new(),
                    }
                })
                .collect(),
        })
    }

    async fn list(
        &self,
        ctx: Context<'_>,
        _req: EmptyRequest,
    ) -> Result<OrganizationListResult, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        let orgs = Organization::list(&self.deps.db_pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        let mut results = Vec::with_capacity(orgs.len());
        for org in orgs {
            let website_count = sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM sources WHERE source_type = 'website' AND organization_id = $1",
            )
            .bind(org.id)
            .fetch_one(&self.deps.db_pool)
            .await
            .unwrap_or(0);

            let social_profile_count = sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM sources WHERE source_type != 'website' AND organization_id = $1",
            )
            .bind(org.id)
            .fetch_one(&self.deps.db_pool)
            .await
            .unwrap_or(0);

            results.push(OrganizationResult {
                id: org.id.to_string(),
                name: org.name,
                description: org.description,
                status: org.status,
                website_count,
                social_profile_count,
                snapshot_count: 0,
                created_at: org.created_at.to_rfc3339(),
                updated_at: org.updated_at.to_rfc3339(),
            });
        }

        Ok(OrganizationListResult {
            organizations: results,
        })
    }

    async fn get(
        &self,
        ctx: Context<'_>,
        req: GetOrganizationRequest,
    ) -> Result<OrganizationResult, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        let org = Organization::find_by_id(OrganizationId::from(req.id), &self.deps.db_pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        let website_count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM sources WHERE source_type = 'website' AND organization_id = $1",
        )
        .bind(org.id)
        .fetch_one(&self.deps.db_pool)
        .await
        .unwrap_or(0);

        let social_profile_count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM sources WHERE source_type != 'website' AND organization_id = $1",
        )
        .bind(org.id)
        .fetch_one(&self.deps.db_pool)
        .await
        .unwrap_or(0);

        // Compute snapshot (extraction page) count across all sources
        let mut snapshot_count: i64 = 0;
        let sources = Source::find_by_organization(org.id, &self.deps.db_pool)
            .await
            .unwrap_or_default();
        for source in &sources {
            if let Ok(site_url) = source.site_url(&self.deps.db_pool).await {
                if let Ok(count) = ExtractionPage::count_by_domain(&site_url, &self.deps.db_pool).await {
                    snapshot_count += count as i64;
                }
            }
        }

        Ok(OrganizationResult {
            id: org.id.to_string(),
            name: org.name,
            description: org.description,
            status: org.status,
            website_count,
            social_profile_count,
            snapshot_count,
            created_at: org.created_at.to_rfc3339(),
            updated_at: org.updated_at.to_rfc3339(),
        })
    }

    async fn create(
        &self,
        ctx: Context<'_>,
        req: CreateOrganizationRequest,
    ) -> Result<OrganizationResult, HandlerError> {
        let user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        let org = Organization::create(&req.name, req.description.as_deref(), "admin", &self.deps.db_pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        // Admin-created orgs are auto-approved
        let org = Organization::approve(org.id, user.member_id, &self.deps.db_pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        Ok(OrganizationResult {
            id: org.id.to_string(),
            name: org.name,
            description: org.description,
            status: org.status,
            website_count: 0,
            social_profile_count: 0,
            snapshot_count: 0,
            created_at: org.created_at.to_rfc3339(),
            updated_at: org.updated_at.to_rfc3339(),
        })
    }

    async fn update(
        &self,
        ctx: Context<'_>,
        req: UpdateOrganizationRequest,
    ) -> Result<OrganizationResult, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        let org = Organization::update(
            OrganizationId::from(req.id),
            &req.name,
            req.description.as_deref(),
            &self.deps.db_pool,
        )
        .await
        .map_err(|e| TerminalError::new(e.to_string()))?;

        let website_count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM sources WHERE source_type = 'website' AND organization_id = $1",
        )
        .bind(org.id)
        .fetch_one(&self.deps.db_pool)
        .await
        .unwrap_or(0);

        let social_profile_count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM sources WHERE source_type != 'website' AND organization_id = $1",
        )
        .bind(org.id)
        .fetch_one(&self.deps.db_pool)
        .await
        .unwrap_or(0);

        Ok(OrganizationResult {
            id: org.id.to_string(),
            name: org.name,
            description: org.description,
            status: org.status,
            website_count,
            social_profile_count,
            snapshot_count: 0,
            created_at: org.created_at.to_rfc3339(),
            updated_at: org.updated_at.to_rfc3339(),
        })
    }

    async fn delete(
        &self,
        ctx: Context<'_>,
        req: DeleteOrganizationRequest,
    ) -> Result<EmptyRequest, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        Organization::delete(OrganizationId::from(req.id), &self.deps.db_pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        Ok(EmptyRequest {})
    }

    async fn approve(
        &self,
        ctx: Context<'_>,
        req: ApproveOrganizationRequest,
    ) -> Result<OrganizationResult, HandlerError> {
        let user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        let org = Organization::approve(
            OrganizationId::from(req.id),
            user.member_id,
            &self.deps.db_pool,
        )
        .await
        .map_err(|e| TerminalError::new(e.to_string()))?;

        info!(org_id = %org.id, "Organization approved");

        Ok(OrganizationResult {
            id: org.id.to_string(),
            name: org.name,
            description: org.description,
            status: org.status,
            website_count: 0,
            social_profile_count: 0,
            snapshot_count: 0,
            created_at: org.created_at.to_rfc3339(),
            updated_at: org.updated_at.to_rfc3339(),
        })
    }

    async fn reject(
        &self,
        ctx: Context<'_>,
        req: RejectOrganizationRequest,
    ) -> Result<OrganizationResult, HandlerError> {
        let user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        let org = Organization::reject(
            OrganizationId::from(req.id),
            user.member_id,
            req.reason,
            &self.deps.db_pool,
        )
        .await
        .map_err(|e| TerminalError::new(e.to_string()))?;

        info!(org_id = %org.id, "Organization rejected");

        Ok(OrganizationResult {
            id: org.id.to_string(),
            name: org.name,
            description: org.description,
            status: org.status,
            website_count: 0,
            social_profile_count: 0,
            snapshot_count: 0,
            created_at: org.created_at.to_rfc3339(),
            updated_at: org.updated_at.to_rfc3339(),
        })
    }

    async fn suspend(
        &self,
        ctx: Context<'_>,
        req: SuspendOrganizationRequest,
    ) -> Result<OrganizationResult, HandlerError> {
        let user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        let org = Organization::suspend(
            OrganizationId::from(req.id),
            user.member_id,
            req.reason,
            &self.deps.db_pool,
        )
        .await
        .map_err(|e| TerminalError::new(e.to_string()))?;

        info!(org_id = %org.id, "Organization suspended");

        Ok(OrganizationResult {
            id: org.id.to_string(),
            name: org.name,
            description: org.description,
            status: org.status,
            website_count: 0,
            social_profile_count: 0,
            snapshot_count: 0,
            created_at: org.created_at.to_rfc3339(),
            updated_at: org.updated_at.to_rfc3339(),
        })
    }

    async fn regenerate(
        &self,
        ctx: Context<'_>,
        req: RegenerateOrganizationRequest,
    ) -> Result<RegenerateOrganizationResult, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;
        let org_id = OrganizationId::from(req.id);
        let pool = &self.deps.db_pool;

        // Verify org exists
        let _org = Organization::find_by_id(org_id, pool)
            .await
            .map_err(|e| TerminalError::new(format!("Organization not found: {}", e)))?;

        // Find all websites linked to this org
        let website_ids: Vec<WebsiteId> = sqlx::query_scalar::<_, Uuid>(
            "SELECT id FROM sources WHERE source_type = 'website' AND organization_id = $1",
        )
        .bind(req.id)
        .fetch_all(pool)
        .await
        .map_err(|e| TerminalError::new(e.to_string()))?
        .into_iter()
        .map(WebsiteId::from_uuid)
        .collect();

        let websites_processed = website_ids.len() as i64;

        if website_ids.is_empty() {
            return Err(TerminalError::new("No websites linked to this organization").into());
        }

        info!(org_id = %org_id, websites = websites_processed, "Regenerating organization in-place");

        // Re-run extraction on each website (no delete — update in place)
        let mut best_name: Option<String> = None;
        let mut best_desc: Option<String> = None;

        for website_id in &website_ids {
            match extract_organization_info(*website_id, &self.deps).await {
                Ok((name, desc, social_links)) => {
                    info!(website_id = %website_id, org_name = %name, "Extraction succeeded");
                    // Use the first successful extraction for name/description
                    if best_name.is_none() {
                        best_name = Some(name);
                        best_desc = desc;
                    }
                    // Create/find social profiles for every website's links
                    create_social_profiles_for_org(org_id, &social_links, pool).await;
                }
                Err(e) => {
                    warn!(website_id = %website_id, error = %e, "Re-extraction failed for website");
                }
            }
        }

        // Update org name/description if we got a successful extraction
        if let Some(name) = &best_name {
            Organization::update(org_id, name, best_desc.as_deref(), pool)
                .await
                .map_err(|e| TerminalError::new(e.to_string()))?;
        }

        Ok(RegenerateOrganizationResult {
            organization_id: Some(org_id.into_uuid().to_string()),
            websites_processed,
            status: "completed".to_string(),
        })
    }

    async fn backfill_organizations(
        &self,
        ctx: Context<'_>,
        _req: EmptyRequest,
    ) -> Result<BackfillResult, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        let websites = Website::find_without_organization(&self.deps.db_pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        let processed = websites.len() as i64;
        let mut succeeded: i64 = 0;
        let mut failed: i64 = 0;

        info!(count = processed, "Starting organization backfill");

        for website in &websites {
            let website_id = website.id;
            match extract_and_create_organization(website_id, &self.deps).await {
                Ok(org_id) => {
                    info!(website_id = %website_id, org_id = %org_id, "Backfill: organization created");
                    succeeded += 1;
                }
                Err(e) => {
                    warn!(website_id = %website_id, error = %e, "Backfill: organization extraction failed");
                    failed += 1;
                }
            }
        }

        info!(processed, succeeded, failed, "Organization backfill complete");

        Ok(BackfillResult {
            processed,
            succeeded,
            failed,
        })
    }

    async fn extract_org_posts(
        &self,
        ctx: Context<'_>,
        req: RegenerateOrganizationRequest,
    ) -> Result<RegenerateOrganizationResult, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;
        let org_id = req.id;

        let workflow_id = format!("extract-org-{}-{}", org_id, chrono::Utc::now().timestamp());

        ctx.workflow_client::<ExtractOrgPostsWorkflowClient>(workflow_id.clone())
            .run(ExtractOrgPostsRequest {
                organization_id: org_id,
            })
            .send();

        info!(org_id = %org_id, workflow_id = %workflow_id, "Triggered org-level extraction");

        Ok(RegenerateOrganizationResult {
            organization_id: Some(org_id.to_string()),
            websites_processed: 0,
            status: format!("started:{}", workflow_id),
        })
    }

    async fn run_scheduled_extraction(
        &self,
        ctx: Context<'_>,
        _req: EmptyRequest,
    ) -> Result<ScheduledExtractionResult, HandlerError> {
        info!("Running scheduled org extraction check");

        let pool = &self.deps.db_pool;
        let org_ids = Organization::find_needing_extraction(pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        if org_ids.is_empty() {
            info!("No organizations need extraction");
            // Self-schedule for 15 minutes
            ctx.service_client::<OrganizationsServiceClient>()
                .run_scheduled_extraction(EmptyRequest {})
                .send_after(Duration::from_secs(900));

            return Ok(ScheduledExtractionResult {
                orgs_triggered: 0,
                status: "no_orgs_due".to_string(),
            });
        }

        info!(count = org_ids.len(), "Triggering extraction for organizations");

        for org_id in &org_ids {
            let wf_key = org_id.into_uuid().to_string();
            ctx.workflow_client::<ExtractOrgPostsWorkflowClient>(wf_key)
                .run(ExtractOrgPostsRequest {
                    organization_id: org_id.into_uuid(),
                })
                .send();
        }

        let triggered = org_ids.len() as i32;

        // Self-schedule for 15 minutes
        ctx.service_client::<OrganizationsServiceClient>()
            .run_scheduled_extraction(EmptyRequest {})
            .send_after(Duration::from_secs(900));

        Ok(ScheduledExtractionResult {
            orgs_triggered: triggered,
            status: "completed".to_string(),
        })
    }

    async fn clean_up_org_posts(
        &self,
        ctx: Context<'_>,
        req: RegenerateOrganizationRequest,
    ) -> Result<RegenerateOrganizationResult, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;
        let org_id = req.id;

        let workflow_id = format!("cleanup-org-{}-{}", org_id, chrono::Utc::now().timestamp());

        ctx.workflow_client::<CleanUpOrgPostsWorkflowClient>(workflow_id.clone())
            .run(CleanUpOrgPostsRequest {
                organization_id: org_id,
            })
            .send();

        info!(org_id = %org_id, workflow_id = %workflow_id, "Triggered org-level cleanup");

        Ok(RegenerateOrganizationResult {
            organization_id: Some(org_id.to_string()),
            websites_processed: 0,
            status: format!("started:{}", workflow_id),
        })
    }
}
