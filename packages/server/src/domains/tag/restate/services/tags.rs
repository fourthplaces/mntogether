//! Tags service (stateless)
//!
//! CRUD operations for tag kinds and tag values.

use restate_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::common::auth::restate_auth::require_admin;
use crate::common::EmptyRequest;
use crate::domains::tag::models::tag::Tag;
use crate::domains::tag::models::tag_kind_config::TagKindConfig;
use crate::impl_restate_serde;
use crate::kernel::ServerDeps;

// =============================================================================
// Request types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTagKindRequest {
    pub slug: String,
    pub display_name: String,
    pub description: Option<String>,
    pub allowed_resource_types: Vec<String>,
}

impl_restate_serde!(CreateTagKindRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateTagKindRequest {
    pub id: Uuid,
    pub display_name: String,
    pub description: Option<String>,
    pub allowed_resource_types: Vec<String>,
}

impl_restate_serde!(UpdateTagKindRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteTagKindRequest {
    pub id: Uuid,
}

impl_restate_serde!(DeleteTagKindRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListTagsRequest {
    pub kind: Option<String>,
}

impl_restate_serde!(ListTagsRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTagRequest {
    pub kind: String,
    pub value: String,
    pub display_name: Option<String>,
}

impl_restate_serde!(CreateTagRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateTagRequest {
    pub id: Uuid,
    pub display_name: String,
}

impl_restate_serde!(UpdateTagRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteTagRequest {
    pub id: Uuid,
}

impl_restate_serde!(DeleteTagRequest);

// =============================================================================
// Response types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagKindResult {
    pub id: Uuid,
    pub slug: String,
    pub display_name: String,
    pub description: Option<String>,
    pub allowed_resource_types: Vec<String>,
    pub tag_count: i64,
}

impl_restate_serde!(TagKindResult);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagKindListResult {
    pub kinds: Vec<TagKindResult>,
}

impl_restate_serde!(TagKindListResult);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagResult {
    pub id: Uuid,
    pub kind: String,
    pub value: String,
    pub display_name: Option<String>,
}

impl_restate_serde!(TagResult);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagListResult {
    pub tags: Vec<TagResult>,
}

impl_restate_serde!(TagListResult);

// =============================================================================
// Service definition
// =============================================================================

#[restate_sdk::service]
#[name = "Tags"]
pub trait TagsService {
    async fn list_kinds(req: EmptyRequest) -> Result<TagKindListResult, HandlerError>;
    async fn create_kind(req: CreateTagKindRequest) -> Result<TagKindResult, HandlerError>;
    async fn update_kind(req: UpdateTagKindRequest) -> Result<TagKindResult, HandlerError>;
    async fn delete_kind(req: DeleteTagKindRequest) -> Result<EmptyRequest, HandlerError>;
    async fn list_tags(req: ListTagsRequest) -> Result<TagListResult, HandlerError>;
    async fn create_tag(req: CreateTagRequest) -> Result<TagResult, HandlerError>;
    async fn update_tag(req: UpdateTagRequest) -> Result<TagResult, HandlerError>;
    async fn delete_tag(req: DeleteTagRequest) -> Result<EmptyRequest, HandlerError>;
}

pub struct TagsServiceImpl {
    deps: Arc<ServerDeps>,
}

impl TagsServiceImpl {
    pub fn with_deps(deps: Arc<ServerDeps>) -> Self {
        Self { deps }
    }
}

impl TagsService for TagsServiceImpl {
    async fn list_kinds(
        &self,
        ctx: Context<'_>,
        _req: EmptyRequest,
    ) -> Result<TagKindListResult, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        let pool = &self.deps.db_pool;
        let kinds = TagKindConfig::find_all(pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

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
                tag_count,
            });
        }

        Ok(TagKindListResult { kinds: results })
    }

    async fn create_kind(
        &self,
        ctx: Context<'_>,
        req: CreateTagKindRequest,
    ) -> Result<TagKindResult, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        let pool = &self.deps.db_pool;
        let kind = TagKindConfig::create(
            &req.slug,
            &req.display_name,
            req.description.as_deref(),
            &req.allowed_resource_types,
            pool,
        )
        .await
        .map_err(|e| TerminalError::new(e.to_string()))?;

        Ok(TagKindResult {
            id: kind.id,
            slug: kind.slug,
            display_name: kind.display_name,
            description: kind.description,
            allowed_resource_types: kind.allowed_resource_types,
            tag_count: 0,
        })
    }

    async fn update_kind(
        &self,
        ctx: Context<'_>,
        req: UpdateTagKindRequest,
    ) -> Result<TagKindResult, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        let pool = &self.deps.db_pool;
        let kind = TagKindConfig::update(
            req.id,
            &req.display_name,
            req.description.as_deref(),
            &req.allowed_resource_types,
            pool,
        )
        .await
        .map_err(|e| TerminalError::new(e.to_string()))?;

        let tag_count = TagKindConfig::tag_count_for_slug(&kind.slug, pool)
            .await
            .unwrap_or(0);

        Ok(TagKindResult {
            id: kind.id,
            slug: kind.slug,
            display_name: kind.display_name,
            description: kind.description,
            allowed_resource_types: kind.allowed_resource_types,
            tag_count,
        })
    }

    async fn delete_kind(
        &self,
        ctx: Context<'_>,
        req: DeleteTagKindRequest,
    ) -> Result<EmptyRequest, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        let pool = &self.deps.db_pool;
        let kind = TagKindConfig::find_by_id(req.id, pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        let tag_count = TagKindConfig::tag_count_for_slug(&kind.slug, pool)
            .await
            .unwrap_or(0);

        if tag_count > 0 {
            return Err(TerminalError::new(format!(
                "Cannot delete kind '{}' — it still has {} tags. Delete the tags first.",
                kind.slug, tag_count
            ))
            .into());
        }

        TagKindConfig::delete(req.id, pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        Ok(EmptyRequest {})
    }

    async fn list_tags(
        &self,
        ctx: Context<'_>,
        req: ListTagsRequest,
    ) -> Result<TagListResult, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        let pool = &self.deps.db_pool;
        let tags = if let Some(kind) = &req.kind {
            Tag::find_by_kind(kind, pool)
                .await
                .map_err(|e| TerminalError::new(e.to_string()))?
        } else {
            Tag::find_all(pool)
                .await
                .map_err(|e| TerminalError::new(e.to_string()))?
        };

        Ok(TagListResult {
            tags: tags
                .into_iter()
                .map(|t| TagResult {
                    id: t.id.into_uuid(),
                    kind: t.kind,
                    value: t.value,
                    display_name: t.display_name,
                })
                .collect(),
        })
    }

    async fn create_tag(
        &self,
        ctx: Context<'_>,
        req: CreateTagRequest,
    ) -> Result<TagResult, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        let pool = &self.deps.db_pool;
        let tag = Tag::find_or_create(&req.kind, &req.value, req.display_name, pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        Ok(TagResult {
            id: tag.id.into_uuid(),
            kind: tag.kind,
            value: tag.value,
            display_name: tag.display_name,
        })
    }

    async fn update_tag(
        &self,
        ctx: Context<'_>,
        req: UpdateTagRequest,
    ) -> Result<TagResult, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        let pool = &self.deps.db_pool;
        let tag_id = crate::common::TagId::from_uuid(req.id);
        let tag = Tag::update_display_name(tag_id, &req.display_name, pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        Ok(TagResult {
            id: tag.id.into_uuid(),
            kind: tag.kind,
            value: tag.value,
            display_name: tag.display_name,
        })
    }

    async fn delete_tag(
        &self,
        ctx: Context<'_>,
        req: DeleteTagRequest,
    ) -> Result<EmptyRequest, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        let pool = &self.deps.db_pool;
        let tag_id = crate::common::TagId::from_uuid(req.id);

        // Delete cascading taggables first
        sqlx::query("DELETE FROM taggables WHERE tag_id = $1")
            .bind(req.id)
            .execute(pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        Tag::delete(tag_id, pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        Ok(EmptyRequest {})
    }
}
