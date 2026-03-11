use axum::extract::State;
use axum::routing::post;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::info;
use uuid::Uuid;

use crate::api::auth::AdminUser;
use crate::api::error::{ApiError, ApiResult};
use crate::api::state::AppState;
use crate::common::OrganizationId;
use crate::domains::notes::models::Note;
use crate::domains::organization::models::organization_checklist::{
    CHECKLIST_KEYS, CHECKLIST_LABELS,
};
use crate::domains::organization::models::{Organization, OrganizationChecklistItem};
use crate::domains::posts::models::Post;
use crate::common::TagId;
use crate::domains::tag::models::TagKindConfig;
use crate::domains::tag::models::Tag;
use crate::domains::tag::Taggable;

// =============================================================================
// Local types (public-facing DTOs)
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicTagResult {
    pub kind: String,
    pub value: String,
    pub display_name: Option<String>,
    pub color: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicPostResult {
    pub id: Uuid,
    pub title: String,
    pub summary: String,
    pub description: String,
    pub location: Option<String>,
    pub source_url: Option<String>,
    pub post_type: String,
    pub category: Option<String>,
    pub created_at: String,
    pub published_at: Option<String>,
    pub tags: Vec<PublicTagResult>,
    pub urgent_notes: Vec<String>,
    pub distance_miles: Option<f64>,
    pub organization_id: Option<Uuid>,
    pub organization_name: Option<String>,
}

// =============================================================================
// Request types
// =============================================================================

#[derive(Debug, Deserialize)]
pub struct CreateOrganizationRequest {
    pub name: String,
    pub description: Option<String>,
    #[serde(default = "default_source_type")]
    pub source_type: String,
}

fn default_source_type() -> String {
    "organization".to_string()
}

#[derive(Debug, Deserialize)]
pub struct UpdateOrganizationRequest {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GetOrganizationRequest {
    pub id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct DeleteOrganizationRequest {
    pub id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct RegenerateOrganizationRequest {
    pub id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct ApproveOrganizationRequest {
    pub id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct RejectOrganizationRequest {
    pub id: Uuid,
    pub reason: String,
}

#[derive(Debug, Deserialize)]
pub struct SuspendOrganizationRequest {
    pub id: Uuid,
    pub reason: String,
}

#[derive(Debug, Deserialize)]
pub struct SetStatusRequest {
    pub id: Uuid,
    pub status: String,
    pub reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ToggleChecklistRequest {
    pub organization_id: Uuid,
    pub checklist_key: String,
    pub checked: bool,
}

// =============================================================================
// Response types
// =============================================================================

#[derive(Debug, Serialize)]
pub struct OrganizationResult {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub source_type: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
pub struct OrganizationListResult {
    pub organizations: Vec<OrganizationResult>,
}

#[derive(Debug, Serialize)]
pub struct OrganizationDetailResult {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub posts: Vec<PublicPostResult>,
}

#[derive(Debug, Serialize)]
pub struct RemoveAllResult {
    pub organization_id: String,
    pub deleted_count: i64,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct ChecklistItemResult {
    pub key: String,
    pub label: String,
    pub checked: bool,
    pub checked_by: Option<String>,
    pub checked_at: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ChecklistResult {
    pub items: Vec<ChecklistItemResult>,
    pub all_checked: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EmptyRequest {}

// =============================================================================
// Helpers
// =============================================================================

fn org_to_result(org: Organization) -> OrganizationResult {
    OrganizationResult {
        id: org.id.to_string(),
        name: org.name,
        description: org.description,
        source_type: org.source_type,
        status: org.status,
        created_at: org.created_at.to_rfc3339(),
        updated_at: org.updated_at.to_rfc3339(),
    }
}

fn build_checklist(checked_items: &[OrganizationChecklistItem]) -> ChecklistResult {
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
    ChecklistResult { items, all_checked }
}

// =============================================================================
// Handlers — public (no auth)
// =============================================================================

async fn public_list(
    State(state): State<AppState>,
    Json(_req): Json<EmptyRequest>,
) -> ApiResult<Json<OrganizationListResult>> {
    let orgs = Organization::find_approved(&state.deps.db_pool).await?;

    Ok(Json(OrganizationListResult {
        organizations: orgs.into_iter().map(org_to_result).collect(),
    }))
}

async fn public_get(
    State(state): State<AppState>,
    Json(req): Json<GetOrganizationRequest>,
) -> ApiResult<Json<OrganizationDetailResult>> {
    let pool = &state.deps.db_pool;
    let org = Organization::find_by_id(OrganizationId::from(req.id), pool).await?;

    if org.status != "approved" {
        return Err(ApiError::NotFound("Organization not found".into()));
    }

    let posts = Post::find_by_organization_id(req.id, pool).await?;

    // Batch-load public tags
    let post_ids: Vec<Uuid> = posts.iter().map(|p| p.id.into_uuid()).collect();
    let tag_rows = Tag::find_public_for_post_ids(&post_ids, pool).await?;

    let mut tags_by_post: HashMap<Uuid, Vec<PublicTagResult>> = HashMap::new();
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

    Ok(Json(OrganizationDetailResult {
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
                    summary: p.summary.unwrap_or_default(),
                    description: p.description,
                    location: p.location,
                    source_url: p.source_url,
                    post_type: p.post_type,
                    category: Some(p.category),
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
    }))
}

// =============================================================================
// Handlers — admin (require AdminUser)
// =============================================================================

async fn list(
    State(state): State<AppState>,
    _user: AdminUser,
    Json(_req): Json<EmptyRequest>,
) -> ApiResult<Json<OrganizationListResult>> {
    let orgs = Organization::list(&state.deps.db_pool).await?;

    Ok(Json(OrganizationListResult {
        organizations: orgs.into_iter().map(org_to_result).collect(),
    }))
}

async fn get(
    State(state): State<AppState>,
    _user: AdminUser,
    Json(req): Json<GetOrganizationRequest>,
) -> ApiResult<Json<OrganizationResult>> {
    let org = Organization::find_by_id(OrganizationId::from(req.id), &state.deps.db_pool).await?;
    Ok(Json(org_to_result(org)))
}

async fn create(
    State(state): State<AppState>,
    user: AdminUser,
    Json(req): Json<CreateOrganizationRequest>,
) -> ApiResult<Json<OrganizationResult>> {
    let pool = &state.deps.db_pool;

    // Validate source_type
    if req.source_type != "organization" && req.source_type != "individual" {
        return Err(ApiError::BadRequest(format!(
            "Invalid source_type: '{}'. Must be 'organization' or 'individual'.",
            req.source_type
        )));
    }

    let org = Organization::create_with_source_type(
        &req.name,
        req.description.as_deref(),
        "admin",
        &req.source_type,
        pool,
    )
    .await?;

    // Admin-created orgs are auto-approved
    let org = Organization::approve(org.id, user.0.member_id, pool).await?;

    Ok(Json(org_to_result(org)))
}

async fn update(
    State(state): State<AppState>,
    _user: AdminUser,
    Json(req): Json<UpdateOrganizationRequest>,
) -> ApiResult<Json<OrganizationResult>> {
    let org = Organization::update(
        OrganizationId::from(req.id),
        &req.name,
        req.description.as_deref(),
        &state.deps.db_pool,
    )
    .await?;

    Ok(Json(org_to_result(org)))
}

async fn delete(
    State(state): State<AppState>,
    _user: AdminUser,
    Json(req): Json<DeleteOrganizationRequest>,
) -> ApiResult<Json<EmptyRequest>> {
    Organization::delete(OrganizationId::from(req.id), &state.deps.db_pool).await?;
    Ok(Json(EmptyRequest {}))
}

async fn approve(
    State(state): State<AppState>,
    user: AdminUser,
    Json(req): Json<ApproveOrganizationRequest>,
) -> ApiResult<Json<OrganizationResult>> {
    let org = Organization::approve(
        OrganizationId::from(req.id),
        user.0.member_id,
        &state.deps.db_pool,
    )
    .await?;

    info!(org_id = %org.id, "Organization approved");
    Ok(Json(org_to_result(org)))
}

async fn reject(
    State(state): State<AppState>,
    user: AdminUser,
    Json(req): Json<RejectOrganizationRequest>,
) -> ApiResult<Json<OrganizationResult>> {
    let pool = &state.deps.db_pool;
    let org_id = OrganizationId::from(req.id);

    let org = Organization::reject(org_id, user.0.member_id, req.reason, pool).await?;

    // Reset checklist on rejection
    OrganizationChecklistItem::reset(org.id, pool).await?;

    info!(org_id = %org.id, "Organization rejected");
    Ok(Json(org_to_result(org)))
}

async fn suspend(
    State(state): State<AppState>,
    user: AdminUser,
    Json(req): Json<SuspendOrganizationRequest>,
) -> ApiResult<Json<OrganizationResult>> {
    let org = Organization::suspend(
        OrganizationId::from(req.id),
        user.0.member_id,
        req.reason,
        &state.deps.db_pool,
    )
    .await?;

    info!(org_id = %org.id, "Organization suspended");
    Ok(Json(org_to_result(org)))
}

async fn remove_all_posts(
    State(state): State<AppState>,
    _user: AdminUser,
    Json(req): Json<RegenerateOrganizationRequest>,
) -> ApiResult<Json<RemoveAllResult>> {
    let deleted = Post::delete_all_for_organization(req.id, &state.deps.db_pool).await?;

    info!(org_id = %req.id, deleted = deleted, "Removed all posts for organization");

    Ok(Json(RemoveAllResult {
        organization_id: req.id.to_string(),
        deleted_count: deleted,
        status: "completed".to_string(),
    }))
}

async fn remove_all_notes(
    State(state): State<AppState>,
    _user: AdminUser,
    Json(req): Json<RegenerateOrganizationRequest>,
) -> ApiResult<Json<RemoveAllResult>> {
    let deleted = Note::delete_all_for_organization(req.id, &state.deps.db_pool).await?;

    info!(org_id = %req.id, deleted = deleted, "Removed all notes for organization");

    Ok(Json(RemoveAllResult {
        organization_id: req.id.to_string(),
        deleted_count: deleted,
        status: "completed".to_string(),
    }))
}

async fn set_status(
    State(state): State<AppState>,
    user: AdminUser,
    Json(req): Json<SetStatusRequest>,
) -> ApiResult<Json<OrganizationResult>> {
    let org_id = OrganizationId::from(req.id);
    let pool = &state.deps.db_pool;

    let org = match req.status.as_str() {
        "pending_review" => {
            let org = Organization::move_to_pending(org_id, pool).await?;
            // Clear checklist when moving back to pending
            OrganizationChecklistItem::reset(org_id, pool).await?;
            org
        }
        "approved" => Organization::approve(org_id, user.0.member_id, pool).await?,
        "rejected" => {
            let reason = req
                .reason
                .unwrap_or_else(|| "Status changed by admin".to_string());
            let org = Organization::reject(org_id, user.0.member_id, reason, pool).await?;
            OrganizationChecklistItem::reset(org_id, pool).await?;
            org
        }
        "suspended" => {
            let reason = req
                .reason
                .unwrap_or_else(|| "Suspended by admin".to_string());
            Organization::suspend(org_id, user.0.member_id, reason, pool).await?
        }
        _ => {
            return Err(ApiError::BadRequest(format!(
                "Invalid status: {}",
                req.status
            )));
        }
    };

    info!(org_id = %org.id, new_status = %req.status, "Organization status changed");
    Ok(Json(org_to_result(org)))
}

async fn get_checklist(
    State(state): State<AppState>,
    _user: AdminUser,
    Json(req): Json<GetOrganizationRequest>,
) -> ApiResult<Json<ChecklistResult>> {
    let org_id = OrganizationId::from(req.id);
    let checked_items =
        OrganizationChecklistItem::find_by_organization(org_id, &state.deps.db_pool).await?;

    Ok(Json(build_checklist(&checked_items)))
}

async fn toggle_checklist_item(
    State(state): State<AppState>,
    user: AdminUser,
    Json(req): Json<ToggleChecklistRequest>,
) -> ApiResult<Json<ChecklistResult>> {
    let org_id = OrganizationId::from(req.organization_id);
    let pool = &state.deps.db_pool;

    if !CHECKLIST_KEYS.contains(&req.checklist_key.as_str()) {
        return Err(ApiError::BadRequest(format!(
            "Invalid checklist key: {}",
            req.checklist_key
        )));
    }

    if req.checked {
        OrganizationChecklistItem::check(org_id, &req.checklist_key, user.0.member_id, pool)
            .await?;
    } else {
        OrganizationChecklistItem::uncheck(org_id, &req.checklist_key, pool).await?;
    }

    // Return updated checklist
    let checked_items = OrganizationChecklistItem::find_by_organization(org_id, pool).await?;

    Ok(Json(build_checklist(&checked_items)))
}

// =============================================================================
// Tag operations
// =============================================================================

#[derive(Debug, Deserialize)]
pub struct OrgAddTagRequest {
    pub organization_id: Uuid,
    pub tag_kind: String,
    pub tag_value: String,
    pub display_name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct OrgRemoveTagRequest {
    pub organization_id: Uuid,
    pub tag_id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct OrgTagResult {
    pub id: String,
    pub kind: String,
    pub value: String,
    pub display_name: Option<String>,
    pub color: Option<String>,
    pub description: Option<String>,
    pub emoji: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct OrgTagsResult {
    pub tags: Vec<OrgTagResult>,
}

fn tag_to_result(tag: &Tag) -> OrgTagResult {
    OrgTagResult {
        id: tag.id.to_string(),
        kind: tag.kind.clone(),
        value: tag.value.clone(),
        display_name: tag.display_name.clone(),
        color: tag.color.clone(),
        description: tag.description.clone(),
        emoji: tag.emoji.clone(),
    }
}

async fn list_org_tags(
    State(state): State<AppState>,
    _user: AdminUser,
    Json(req): Json<GetOrganizationRequest>,
) -> ApiResult<Json<OrgTagsResult>> {
    let org_id = OrganizationId::from(req.id);
    let tags = Tag::find_for_organization(org_id, &state.deps.db_pool).await?;
    Ok(Json(OrgTagsResult {
        tags: tags.iter().map(tag_to_result).collect(),
    }))
}

async fn add_org_tag(
    State(state): State<AppState>,
    _user: AdminUser,
    Json(req): Json<OrgAddTagRequest>,
) -> ApiResult<Json<OrgTagsResult>> {
    let pool = &state.deps.db_pool;
    let org_id = OrganizationId::from(req.organization_id);

    // Check if locked kind — only allow existing tag values
    if let Some(kind_config) = TagKindConfig::find_by_slug(&req.tag_kind, pool).await? {
        if kind_config.locked {
            let existing = Tag::find_by_kind_value(&req.tag_kind, &req.tag_value, pool).await?;
            if existing.is_none() {
                return Err(ApiError::BadRequest(format!(
                    "Cannot create new tags under locked kind '{}'. Values are fixed.",
                    req.tag_kind
                )));
            }
        }
    }

    let tag = Tag::find_or_create(&req.tag_kind, &req.tag_value, req.display_name, pool).await?;
    Taggable::create_org_tag(org_id, tag.id, pool).await?;

    info!(org_id = %org_id, tag_kind = %req.tag_kind, tag_value = %req.tag_value, "Added org tag");

    let tags = Tag::find_for_organization(org_id, pool).await?;
    Ok(Json(OrgTagsResult {
        tags: tags.iter().map(tag_to_result).collect(),
    }))
}

async fn remove_org_tag(
    State(state): State<AppState>,
    _user: AdminUser,
    Json(req): Json<OrgRemoveTagRequest>,
) -> ApiResult<Json<OrgTagsResult>> {
    let pool = &state.deps.db_pool;
    let org_id = OrganizationId::from(req.organization_id);
    let tag_id = TagId::from(req.tag_id);

    Taggable::delete_org_tag(org_id, tag_id, pool).await?;

    info!(org_id = %org_id, tag_id = %tag_id, "Removed org tag");

    let tags = Tag::find_for_organization(org_id, pool).await?;
    Ok(Json(OrgTagsResult {
        tags: tags.iter().map(tag_to_result).collect(),
    }))
}

// =============================================================================
// Router
// =============================================================================

pub fn router() -> Router<AppState> {
    Router::new()
        // Public (no auth)
        .route("/Organizations/public_list", post(public_list))
        .route("/Organizations/public_get", post(public_get))
        // Admin CRUD
        .route("/Organizations/list", post(list))
        .route("/Organizations/get", post(get))
        .route("/Organizations/create", post(create))
        .route("/Organizations/update", post(update))
        .route("/Organizations/delete", post(delete))
        // Approval workflow
        .route("/Organizations/approve", post(approve))
        .route("/Organizations/reject", post(reject))
        .route("/Organizations/suspend", post(suspend))
        // Bulk operations
        .route("/Organizations/remove_all_posts", post(remove_all_posts))
        .route("/Organizations/remove_all_notes", post(remove_all_notes))
        // Status management
        .route("/Organizations/set_status", post(set_status))
        // Checklist
        .route("/Organizations/get_checklist", post(get_checklist))
        .route(
            "/Organizations/toggle_checklist_item",
            post(toggle_checklist_item),
        )
        // Tags
        .route("/Organizations/list_tags", post(list_org_tags))
        .route("/Organizations/add_tag", post(add_org_tag))
        .route("/Organizations/remove_tag", post(remove_org_tag))
}
