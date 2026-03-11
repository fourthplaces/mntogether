use axum::extract::State;
use axum::routing::post;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::auth::AdminUser;
use crate::api::error::{ApiError, ApiResult};
use crate::api::state::AppState;
use crate::common::TagId;
use crate::domains::tag::models::tag::Tag;
use crate::domains::tag::models::tag_kind_config::TagKindConfig;

// --- Request types ---

#[derive(Debug, Deserialize)]
pub struct EmptyRequest {}

#[derive(Debug, Deserialize)]
pub struct CreateTagKindRequest {
    pub slug: String,
    pub display_name: String,
    pub description: Option<String>,
    pub allowed_resource_types: Vec<String>,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub is_public: bool,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTagKindRequest {
    pub id: Uuid,
    pub display_name: String,
    pub description: Option<String>,
    pub allowed_resource_types: Vec<String>,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub is_public: bool,
}

#[derive(Debug, Deserialize)]
pub struct DeleteTagKindRequest {
    pub id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct ListTagsRequest {
    pub kind: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateTagRequest {
    pub kind: String,
    pub value: String,
    pub display_name: Option<String>,
    pub color: Option<String>,
    pub description: Option<String>,
    pub emoji: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTagRequest {
    pub id: Uuid,
    pub display_name: String,
    pub color: Option<String>,
    pub description: Option<String>,
    pub emoji: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct DeleteTagRequest {
    pub id: Uuid,
}

// --- Response types ---

#[derive(Debug, Serialize)]
pub struct TagKindResult {
    pub id: Uuid,
    pub slug: String,
    pub display_name: String,
    pub description: Option<String>,
    pub allowed_resource_types: Vec<String>,
    pub required: bool,
    pub is_public: bool,
    pub locked: bool,
    pub tag_count: i64,
}

#[derive(Debug, Serialize)]
pub struct TagKindListResult {
    pub kinds: Vec<TagKindResult>,
}

#[derive(Debug, Serialize)]
pub struct TagResult {
    pub id: Uuid,
    pub kind: String,
    pub value: String,
    pub display_name: Option<String>,
    pub color: Option<String>,
    pub description: Option<String>,
    pub emoji: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TagListResult {
    pub tags: Vec<TagResult>,
}

#[derive(Debug, Serialize)]
pub struct Empty {}

// --- Handlers ---

async fn list_kinds(
    State(state): State<AppState>,
    _user: AdminUser,
    Json(_req): Json<EmptyRequest>,
) -> ApiResult<Json<TagKindListResult>> {
    let pool = &state.deps.db_pool;
    let kinds = TagKindConfig::find_all(pool).await?;

    let mut results = Vec::with_capacity(kinds.len());
    for kind in kinds {
        let tag_count = TagKindConfig::tag_count_for_slug(&kind.slug, pool)
            .await
            .unwrap_or(0);
        results.push(TagKindResult {
            id: kind.id,
            slug: kind.slug,
            display_name: kind.display_name,
            description: kind.description,
            allowed_resource_types: kind.allowed_resource_types,
            required: kind.required,
            is_public: kind.is_public,
            locked: kind.locked,
            tag_count,
        });
    }

    Ok(Json(TagKindListResult { kinds: results }))
}

async fn create_kind(
    State(state): State<AppState>,
    _user: AdminUser,
    Json(req): Json<CreateTagKindRequest>,
) -> ApiResult<Json<TagKindResult>> {
    let pool = &state.deps.db_pool;
    let kind = TagKindConfig::create(
        &req.slug,
        &req.display_name,
        req.description.as_deref(),
        &req.allowed_resource_types,
        req.required,
        req.is_public,
        pool,
    )
    .await?;

    Ok(Json(TagKindResult {
        id: kind.id,
        slug: kind.slug,
        display_name: kind.display_name,
        description: kind.description,
        allowed_resource_types: kind.allowed_resource_types,
        required: kind.required,
        is_public: kind.is_public,
        locked: kind.locked,
        tag_count: 0,
    }))
}

async fn update_kind(
    State(state): State<AppState>,
    _user: AdminUser,
    Json(req): Json<UpdateTagKindRequest>,
) -> ApiResult<Json<TagKindResult>> {
    let pool = &state.deps.db_pool;

    // Guard: cannot update locked (hard) tag kinds
    let existing = TagKindConfig::find_by_id(req.id, pool).await?;
    if existing.locked {
        return Err(ApiError::BadRequest(format!(
            "Cannot update locked tag kind '{}'",
            existing.slug
        )));
    }

    let kind = TagKindConfig::update(
        req.id,
        &req.display_name,
        req.description.as_deref(),
        &req.allowed_resource_types,
        req.required,
        req.is_public,
        pool,
    )
    .await?;

    let tag_count = TagKindConfig::tag_count_for_slug(&kind.slug, pool)
        .await
        .unwrap_or(0);

    Ok(Json(TagKindResult {
        id: kind.id,
        slug: kind.slug,
        display_name: kind.display_name,
        description: kind.description,
        allowed_resource_types: kind.allowed_resource_types,
        required: kind.required,
        is_public: kind.is_public,
        locked: kind.locked,
        tag_count,
    }))
}

async fn delete_kind(
    State(state): State<AppState>,
    _user: AdminUser,
    Json(req): Json<DeleteTagKindRequest>,
) -> ApiResult<Json<Empty>> {
    let pool = &state.deps.db_pool;
    let kind = TagKindConfig::find_by_id(req.id, pool).await?;

    // Guard: cannot delete locked (hard) tag kinds
    if kind.locked {
        return Err(ApiError::BadRequest(format!(
            "Cannot delete locked tag kind '{}'",
            kind.slug
        )));
    }

    let tag_count = TagKindConfig::tag_count_for_slug(&kind.slug, pool)
        .await
        .unwrap_or(0);

    if tag_count > 0 {
        return Err(ApiError::BadRequest(format!(
            "Cannot delete kind '{}' — it still has {} tags. Delete the tags first.",
            kind.slug, tag_count
        )));
    }

    TagKindConfig::delete(req.id, pool).await?;

    Ok(Json(Empty {}))
}

async fn list_tags(
    State(state): State<AppState>,
    _user: AdminUser,
    Json(req): Json<ListTagsRequest>,
) -> ApiResult<Json<TagListResult>> {
    let pool = &state.deps.db_pool;
    let tags = if let Some(kind) = &req.kind {
        Tag::find_by_kind(kind, pool).await?
    } else {
        Tag::find_all(pool).await?
    };

    Ok(Json(TagListResult {
        tags: tags
            .into_iter()
            .map(|t| TagResult {
                id: t.id.into_uuid(),
                kind: t.kind,
                value: t.value,
                display_name: t.display_name,
                color: t.color,
                description: t.description,
                emoji: t.emoji,
            })
            .collect(),
    }))
}

async fn create_tag(
    State(state): State<AppState>,
    _user: AdminUser,
    Json(req): Json<CreateTagRequest>,
) -> ApiResult<Json<TagResult>> {
    let pool = &state.deps.db_pool;

    // Guard: cannot create new tag values under locked (hard) kinds
    if let Some(kind_config) = TagKindConfig::find_by_slug(&req.kind, pool).await? {
        if kind_config.locked {
            // Check if this exact value already exists — allow find, block create
            let existing = Tag::find_by_kind_value(&req.kind, &req.value, pool).await?;
            if existing.is_none() {
                return Err(ApiError::BadRequest(format!(
                    "Cannot create new tags under locked kind '{}'. Values are fixed.",
                    req.kind
                )));
            }
        }
    }

    let mut tag = Tag::find_or_create(&req.kind, &req.value, req.display_name, pool).await?;

    if req.color.is_some() {
        tag = Tag::update_color(tag.id, req.color.as_deref(), pool).await?;
    }

    if req.description.is_some() {
        tag = Tag::update_description(tag.id, req.description.as_deref(), pool).await?;
    }

    if req.emoji.is_some() {
        tag = Tag::update_emoji(tag.id, req.emoji.as_deref(), pool).await?;
    }

    Ok(Json(TagResult {
        id: tag.id.into_uuid(),
        kind: tag.kind,
        value: tag.value,
        display_name: tag.display_name,
        color: tag.color,
        description: tag.description,
        emoji: tag.emoji,
    }))
}

async fn update_tag(
    State(state): State<AppState>,
    _user: AdminUser,
    Json(req): Json<UpdateTagRequest>,
) -> ApiResult<Json<TagResult>> {
    let pool = &state.deps.db_pool;
    let tag_id = TagId::from_uuid(req.id);

    Tag::update_display_name(tag_id, &req.display_name, pool).await?;
    Tag::update_color(tag_id, req.color.as_deref(), pool).await?;
    Tag::update_description(tag_id, req.description.as_deref(), pool).await?;
    let tag = Tag::update_emoji(tag_id, req.emoji.as_deref(), pool).await?;

    Ok(Json(TagResult {
        id: tag.id.into_uuid(),
        kind: tag.kind,
        value: tag.value,
        display_name: tag.display_name,
        color: tag.color,
        description: tag.description,
        emoji: tag.emoji,
    }))
}

async fn delete_tag(
    State(state): State<AppState>,
    _user: AdminUser,
    Json(req): Json<DeleteTagRequest>,
) -> ApiResult<Json<Empty>> {
    let pool = &state.deps.db_pool;
    let tag_id = TagId::from_uuid(req.id);

    // Guard: cannot delete tags from locked (hard) kinds
    let tag = Tag::find_by_id(tag_id, pool).await?;
    if let Some(kind_config) = TagKindConfig::find_by_slug(&tag.kind, pool).await? {
        if kind_config.locked {
            return Err(ApiError::BadRequest(format!(
                "Cannot delete tags from locked kind '{}'",
                tag.kind
            )));
        }
    }

    // Delete cascading taggables first
    sqlx::query("DELETE FROM taggables WHERE tag_id = $1")
        .bind(req.id)
        .execute(pool)
        .await
        .map_err(|e| anyhow::anyhow!(e))?;

    Tag::delete(tag_id, pool).await?;

    Ok(Json(Empty {}))
}

// --- Router ---

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/Tags/list_kinds", post(list_kinds))
        .route("/Tags/create_kind", post(create_kind))
        .route("/Tags/update_kind", post(update_kind))
        .route("/Tags/delete_kind", post(delete_kind))
        .route("/Tags/list_tags", post(list_tags))
        .route("/Tags/create_tag", post(create_tag))
        .route("/Tags/update_tag", post(update_tag))
        .route("/Tags/delete_tag", post(delete_tag))
}
