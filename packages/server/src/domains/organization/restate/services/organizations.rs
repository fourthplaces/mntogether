use restate_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;
use uuid::Uuid;

use crate::common::auth::restate_auth::require_admin;
use crate::common::{EmptyRequest, OrganizationId};
use crate::domains::organization::models::{Organization, OrganizationChecklistItem};
use crate::domains::organization::models::organization_checklist::{CHECKLIST_KEYS, CHECKLIST_LABELS};
use crate::domains::notes::models::Note;
use crate::domains::posts::models::Post;
use crate::domains::posts::restate::services::posts::{PublicPostResult, PublicTagResult};
use crate::domains::tag::models::Tag;
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
pub struct RemoveAllResult {
    pub organization_id: String,
    pub deleted_count: i64,
    pub status: String,
}

impl_restate_serde!(RemoveAllResult);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetStatusRequest {
    pub id: Uuid,
    pub status: String,
    pub reason: Option<String>,
}

impl_restate_serde!(SetStatusRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToggleChecklistRequest {
    pub organization_id: Uuid,
    pub checklist_key: String,
    pub checked: bool,
}

impl_restate_serde!(ToggleChecklistRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChecklistItemResult {
    pub key: String,
    pub label: String,
    pub checked: bool,
    pub checked_by: Option<String>,
    pub checked_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChecklistResult {
    pub items: Vec<ChecklistItemResult>,
    pub all_checked: bool,
}

impl_restate_serde!(ChecklistResult);

// =============================================================================
// Response types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganizationResult {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub status: String,
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
pub struct OrganizationDetailResult {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub posts: Vec<PublicPostResult>,
}

impl_restate_serde!(OrganizationDetailResult);

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
    async fn remove_all_posts(
        req: RegenerateOrganizationRequest,
    ) -> Result<RemoveAllResult, HandlerError>;
    async fn remove_all_notes(
        req: RegenerateOrganizationRequest,
    ) -> Result<RemoveAllResult, HandlerError>;
    async fn set_status(req: SetStatusRequest) -> Result<OrganizationResult, HandlerError>;
    async fn get_checklist(
        req: GetOrganizationRequest,
    ) -> Result<ChecklistResult, HandlerError>;
    async fn toggle_checklist_item(
        req: ToggleChecklistRequest,
    ) -> Result<ChecklistResult, HandlerError>;
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

        let org_uuid = org.id.into_uuid();
        let org_name = org.name.clone();
        Ok(OrganizationDetailResult {
            id: org_uuid.to_string(),
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
                        distance_miles: None,
                        organization_id: Some(org_uuid),
                        organization_name: Some(org_name.clone()),
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
            results.push(OrganizationResult {
                id: org.id.to_string(),
                name: org.name,
                description: org.description,
                status: org.status,
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

        Ok(OrganizationResult {
            id: org.id.to_string(),
            name: org.name,
            description: org.description,
            status: org.status,
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

        let org = Organization::create(
            &req.name,
            req.description.as_deref(),
            "admin",
            &self.deps.db_pool,
        )
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

        Ok(OrganizationResult {
            id: org.id.to_string(),
            name: org.name,
            description: org.description,
            status: org.status,
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

        // Reset checklist on rejection
        OrganizationChecklistItem::reset(org.id, &self.deps.db_pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        info!(org_id = %org.id, "Organization rejected");

        Ok(OrganizationResult {
            id: org.id.to_string(),
            name: org.name,
            description: org.description,
            status: org.status,
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
            created_at: org.created_at.to_rfc3339(),
            updated_at: org.updated_at.to_rfc3339(),
        })
    }

    async fn remove_all_posts(
        &self,
        ctx: Context<'_>,
        req: RegenerateOrganizationRequest,
    ) -> Result<RemoveAllResult, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;
        let org_id = req.id;

        let deleted = Post::delete_all_for_organization(org_id, &self.deps.db_pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        info!(org_id = %org_id, deleted = deleted, "Removed all posts for organization");

        Ok(RemoveAllResult {
            organization_id: org_id.to_string(),
            deleted_count: deleted,
            status: "completed".to_string(),
        })
    }

    async fn remove_all_notes(
        &self,
        ctx: Context<'_>,
        req: RegenerateOrganizationRequest,
    ) -> Result<RemoveAllResult, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;
        let org_id = req.id;

        let deleted = Note::delete_all_for_organization(org_id, &self.deps.db_pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        info!(org_id = %org_id, deleted = deleted, "Removed all notes for organization");

        Ok(RemoveAllResult {
            organization_id: org_id.to_string(),
            deleted_count: deleted,
            status: "completed".to_string(),
        })
    }

    async fn set_status(
        &self,
        ctx: Context<'_>,
        req: SetStatusRequest,
    ) -> Result<OrganizationResult, HandlerError> {
        let user = require_admin(ctx.headers(), &self.deps.jwt_service)?;
        let org_id = OrganizationId::from(req.id);
        let pool = &self.deps.db_pool;

        let org = match req.status.as_str() {
            "pending_review" => {
                let org = Organization::move_to_pending(org_id, pool)
                    .await
                    .map_err(|e| TerminalError::new(e.to_string()))?;
                // Clear checklist when moving back to pending
                OrganizationChecklistItem::reset(org_id, pool)
                    .await
                    .map_err(|e| TerminalError::new(e.to_string()))?;
                org
            }
            "approved" => {
                Organization::approve(org_id, user.member_id, pool)
                    .await
                    .map_err(|e| TerminalError::new(e.to_string()))?
            }
            "rejected" => {
                let reason = req.reason.unwrap_or_else(|| "Status changed by admin".to_string());
                let org = Organization::reject(org_id, user.member_id, reason, pool)
                    .await
                    .map_err(|e| TerminalError::new(e.to_string()))?;
                OrganizationChecklistItem::reset(org_id, pool)
                    .await
                    .map_err(|e| TerminalError::new(e.to_string()))?;
                org
            }
            "suspended" => {
                let reason = req.reason.unwrap_or_else(|| "Suspended by admin".to_string());
                Organization::suspend(org_id, user.member_id, reason, pool)
                    .await
                    .map_err(|e| TerminalError::new(e.to_string()))?
            }
            _ => {
                return Err(TerminalError::new(format!("Invalid status: {}", req.status)).into());
            }
        };

        info!(org_id = %org.id, new_status = %req.status, "Organization status changed");

        Ok(OrganizationResult {
            id: org.id.to_string(),
            name: org.name,
            description: org.description,
            status: org.status,
            created_at: org.created_at.to_rfc3339(),
            updated_at: org.updated_at.to_rfc3339(),
        })
    }

    async fn get_checklist(
        &self,
        ctx: Context<'_>,
        req: GetOrganizationRequest,
    ) -> Result<ChecklistResult, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;
        let org_id = OrganizationId::from(req.id);

        let checked_items = OrganizationChecklistItem::find_by_organization(org_id, &self.deps.db_pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        let items: Vec<ChecklistItemResult> = CHECKLIST_LABELS
            .iter()
            .map(|(key, label)| {
                let found = checked_items.iter().find(|ci| ci.checklist_key == *key);
                ChecklistItemResult {
                    key: key.to_string(),
                    label: label.to_string(),
                    checked: found.is_some(),
                    checked_by: found.map(|ci| ci.checked_by.to_string()),
                    checked_at: found.map(|ci| ci.checked_at.to_rfc3339()),
                }
            })
            .collect();

        let all_checked = items.iter().all(|i| i.checked);

        Ok(ChecklistResult { items, all_checked })
    }

    async fn toggle_checklist_item(
        &self,
        ctx: Context<'_>,
        req: ToggleChecklistRequest,
    ) -> Result<ChecklistResult, HandlerError> {
        let user = require_admin(ctx.headers(), &self.deps.jwt_service)?;
        let org_id = OrganizationId::from(req.organization_id);

        if !CHECKLIST_KEYS.contains(&req.checklist_key.as_str()) {
            return Err(TerminalError::new(format!("Invalid checklist key: {}", req.checklist_key)).into());
        }

        if req.checked {
            OrganizationChecklistItem::check(org_id, &req.checklist_key, user.member_id, &self.deps.db_pool)
                .await
                .map_err(|e| TerminalError::new(e.to_string()))?;
        } else {
            OrganizationChecklistItem::uncheck(org_id, &req.checklist_key, &self.deps.db_pool)
                .await
                .map_err(|e| TerminalError::new(e.to_string()))?;
        }

        // Return updated checklist
        let checked_items = OrganizationChecklistItem::find_by_organization(org_id, &self.deps.db_pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        let items: Vec<ChecklistItemResult> = CHECKLIST_LABELS
            .iter()
            .map(|(key, label)| {
                let found = checked_items.iter().find(|ci| ci.checklist_key == *key);
                ChecklistItemResult {
                    key: key.to_string(),
                    label: label.to_string(),
                    checked: found.is_some(),
                    checked_by: found.map(|ci| ci.checked_by.to_string()),
                    checked_at: found.map(|ci| ci.checked_at.to_rfc3339()),
                }
            })
            .collect();

        let all_checked = items.iter().all(|i| i.checked);

        Ok(ChecklistResult { items, all_checked })
    }
}
