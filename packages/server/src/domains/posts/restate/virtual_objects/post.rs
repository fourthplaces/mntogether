//! Post virtual object
//!
//! Keyed by post_id. Exclusive handlers serialize writes per post.
//! Shared handlers allow concurrent reads.

use restate_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::common::auth::restate_auth::{optional_auth, require_admin};
use crate::common::EmptyRequest;
use crate::common::{PostId, ScheduleId};
use crate::domains::posts::activities;
use crate::domains::posts::activities::schedule::ScheduleParams;
use crate::domains::posts::activities::tags::TagInput;
use crate::domains::posts::models::post_report::PostReportRecord;
use crate::domains::posts::models::Post;
use crate::impl_restate_serde;
use crate::kernel::ServerDeps;

// =============================================================================
// Request types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovePostRequest {}

impl_restate_serde!(ApprovePostRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditApproveRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub description_markdown: Option<String>,
    pub tldr: Option<String>,
    pub urgency: Option<String>,
    pub location: Option<String>,
}

impl_restate_serde!(EditApproveRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RejectPostRequest {
    pub reason: String,
}

impl_restate_serde!(RejectPostRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportPostRequest {
    pub reason: String,
    pub category: String,
    pub reporter_email: Option<String>,
}

impl_restate_serde!(ReportPostRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolveReportRequest {
    pub report_id: Uuid,
    pub resolution_notes: Option<String>,
    pub action_taken: String,
}

impl_restate_serde!(ResolveReportRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DismissReportRequest {
    pub report_id: Uuid,
    pub resolution_notes: Option<String>,
}

impl_restate_serde!(DismissReportRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateTagsRequest {
    pub tags: Vec<TagInputData>,
}

impl_restate_serde!(UpdateTagsRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagInputData {
    pub kind: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddTagRequest {
    pub tag_kind: String,
    pub tag_value: String,
    pub display_name: Option<String>,
}

impl_restate_serde!(AddTagRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoveTagRequest {
    pub tag_id: String,
}

impl_restate_serde!(RemoveTagRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddScheduleRequest {
    pub dtstart: Option<String>,
    pub dtend: Option<String>,
    pub rrule: Option<String>,
    pub exdates: Option<String>,
    pub opens_at: Option<String>,
    pub closes_at: Option<String>,
    pub day_of_week: Option<i32>,
    pub timezone: Option<String>,
    pub is_all_day: Option<bool>,
    pub duration_minutes: Option<i32>,
    pub notes: Option<String>,
}

impl_restate_serde!(AddScheduleRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateScheduleRequest {
    pub schedule_id: Uuid,
    pub dtstart: Option<String>,
    pub dtend: Option<String>,
    pub rrule: Option<String>,
    pub exdates: Option<String>,
    pub opens_at: Option<String>,
    pub closes_at: Option<String>,
    pub day_of_week: Option<i32>,
    pub timezone: Option<String>,
    pub is_all_day: Option<bool>,
    pub duration_minutes: Option<i32>,
    pub notes: Option<String>,
}

impl_restate_serde!(UpdateScheduleRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteScheduleRequest {
    pub schedule_id: Uuid,
}

impl_restate_serde!(DeleteScheduleRequest);

// =============================================================================
// Response types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostResult {
    pub id: Uuid,
    pub title: String,
    pub description: String,
    pub description_markdown: Option<String>,
    pub tldr: Option<String>,
    pub status: String,
    pub post_type: String,
    pub category: String,
    pub urgency: Option<String>,
    pub location: Option<String>,
    pub source_url: Option<String>,
    pub website_id: Option<Uuid>,
    pub submission_type: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl_restate_serde!(PostResult);

impl From<Post> for PostResult {
    fn from(p: Post) -> Self {
        Self {
            id: p.id.into_uuid(),
            title: p.title,
            description: p.description,
            description_markdown: p.description_markdown,
            tldr: p.tldr,
            status: p.status,
            post_type: p.post_type,
            category: p.category,
            urgency: p.urgency,
            location: p.location,
            source_url: p.source_url,
            website_id: p.website_id.map(|id| id.into_uuid()),
            submission_type: p.submission_type,
            created_at: p.created_at.to_rfc3339(),
            updated_at: p.updated_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleResult {
    pub id: Uuid,
    pub schedulable_type: String,
    pub schedulable_id: Uuid,
}

impl_restate_serde!(ScheduleResult);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportResult {
    pub id: Uuid,
    pub post_id: Uuid,
    pub reason: String,
    pub category: String,
    pub status: String,
}

impl_restate_serde!(ReportResult);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportListResult {
    pub reports: Vec<ReportResult>,
}

impl_restate_serde!(ReportListResult);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptionalPostResult {
    pub post: Option<PostResult>,
}

impl_restate_serde!(OptionalPostResult);

// =============================================================================
// Virtual object definition
// =============================================================================

#[restate_sdk::object]
#[name = "Post"]
pub trait PostObject {
    // --- Writes (exclusive, serialized per post_id) ---
    async fn approve(req: ApprovePostRequest) -> Result<PostResult, HandlerError>;
    async fn edit_and_approve(req: EditApproveRequest) -> Result<PostResult, HandlerError>;
    async fn reject(req: RejectPostRequest) -> Result<PostResult, HandlerError>;
    async fn delete(req: EmptyRequest) -> Result<(), HandlerError>;
    async fn archive(req: EmptyRequest) -> Result<PostResult, HandlerError>;
    async fn expire(req: EmptyRequest) -> Result<PostResult, HandlerError>;
    async fn report(req: ReportPostRequest) -> Result<(), HandlerError>;
    async fn resolve_report(req: ResolveReportRequest) -> Result<(), HandlerError>;
    async fn dismiss_report(req: DismissReportRequest) -> Result<(), HandlerError>;
    async fn update_tags(req: UpdateTagsRequest) -> Result<PostResult, HandlerError>;
    async fn add_tag(req: AddTagRequest) -> Result<(), HandlerError>;
    async fn remove_tag(req: RemoveTagRequest) -> Result<(), HandlerError>;
    async fn add_schedule(req: AddScheduleRequest) -> Result<ScheduleResult, HandlerError>;
    async fn update_schedule(req: UpdateScheduleRequest) -> Result<ScheduleResult, HandlerError>;
    async fn delete_schedule(req: DeleteScheduleRequest) -> Result<(), HandlerError>;
    async fn track_view(req: EmptyRequest) -> Result<(), HandlerError>;
    async fn track_click(req: EmptyRequest) -> Result<(), HandlerError>;
    async fn generate_embedding(req: EmptyRequest) -> Result<(), HandlerError>;
    async fn approve_revision(req: EmptyRequest) -> Result<PostResult, HandlerError>;
    async fn reject_revision(req: EmptyRequest) -> Result<(), HandlerError>;
    async fn regenerate(req: EmptyRequest) -> Result<PostResult, HandlerError>;

    // --- Reads (shared, concurrent) ---
    #[shared]
    async fn get(req: EmptyRequest) -> Result<PostResult, HandlerError>;

    #[shared]
    async fn get_reports(req: EmptyRequest) -> Result<ReportListResult, HandlerError>;

    #[shared]
    async fn get_revision(req: EmptyRequest) -> Result<OptionalPostResult, HandlerError>;
}

// =============================================================================
// Implementation
// =============================================================================

pub struct PostObjectImpl {
    deps: Arc<ServerDeps>,
}

impl PostObjectImpl {
    pub fn with_deps(deps: Arc<ServerDeps>) -> Self {
        Self { deps }
    }

    fn parse_post_id(key: &str) -> Result<Uuid, HandlerError> {
        Uuid::parse_str(key).map_err(|e| TerminalError::new(format!("Invalid post ID: {}", e)).into())
    }
}

impl PostObject for PostObjectImpl {
    async fn approve(
        &self,
        ctx: ObjectContext<'_>,
        _req: ApprovePostRequest,
    ) -> Result<PostResult, HandlerError> {
        let user = require_admin(ctx.headers(), &self.deps.jwt_service)?;
        let post_id = Self::parse_post_id(ctx.key())?;

        ctx.run(|| async {
            activities::approve_post(
                post_id,
                user.member_id.into_uuid(),
                user.is_admin,
                &self.deps,
            )
            .await
            .map(|_| ())
            .map_err(Into::into)
        })
        .await?;

        let post = Post::find_by_id(PostId::from_uuid(post_id), &self.deps.db_pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?
            .ok_or_else(|| TerminalError::new("Post not found after approve"))?;

        Ok(PostResult::from(post))
    }

    async fn edit_and_approve(
        &self,
        ctx: ObjectContext<'_>,
        req: EditApproveRequest,
    ) -> Result<PostResult, HandlerError> {
        let user = require_admin(ctx.headers(), &self.deps.jwt_service)?;
        let post_id = Self::parse_post_id(ctx.key())?;

        // Bridge to the existing EditPostInput (which derives GraphQLInputObject)
        // by creating the activity-level type directly
        use crate::domains::posts::data::types::EditPostInput;
        let edit_input = EditPostInput {
            title: req.title,
            description: req.description,
            description_markdown: req.description_markdown,
            tldr: req.tldr,
            urgency: req.urgency,
            location: req.location,
        };

        ctx.run(|| async {
            activities::edit_and_approve_post(
                post_id,
                edit_input.clone(),
                user.member_id.into_uuid(),
                user.is_admin,
                &self.deps,
            )
            .await
            .map(|_| ())
            .map_err(Into::into)
        })
        .await?;

        let post = Post::find_by_id(PostId::from_uuid(post_id), &self.deps.db_pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?
            .ok_or_else(|| TerminalError::new("Post not found after edit_and_approve"))?;

        Ok(PostResult::from(post))
    }

    async fn reject(
        &self,
        ctx: ObjectContext<'_>,
        req: RejectPostRequest,
    ) -> Result<PostResult, HandlerError> {
        let user = require_admin(ctx.headers(), &self.deps.jwt_service)?;
        let post_id = Self::parse_post_id(ctx.key())?;

        ctx.run(|| async {
            activities::reject_post(
                post_id,
                req.reason.clone(),
                user.member_id.into_uuid(),
                user.is_admin,
                &self.deps,
            )
            .await
            .map_err(Into::into)
        })
        .await?;

        let post = Post::find_by_id(PostId::from_uuid(post_id), &self.deps.db_pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?
            .ok_or_else(|| TerminalError::new("Post not found after reject"))?;

        Ok(PostResult::from(post))
    }

    async fn delete(
        &self,
        ctx: ObjectContext<'_>,
        _req: EmptyRequest,
    ) -> Result<(), HandlerError> {
        let user = require_admin(ctx.headers(), &self.deps.jwt_service)?;
        let post_id = Self::parse_post_id(ctx.key())?;

        ctx.run(|| async {
            activities::delete_post(
                post_id,
                user.member_id.into_uuid(),
                user.is_admin,
                &self.deps,
            )
            .await
            .map_err(Into::into)
        })
        .await?;

        Ok(())
    }

    async fn archive(
        &self,
        ctx: ObjectContext<'_>,
        _req: EmptyRequest,
    ) -> Result<PostResult, HandlerError> {
        let user = require_admin(ctx.headers(), &self.deps.jwt_service)?;
        let post_id = Self::parse_post_id(ctx.key())?;

        ctx.run(|| async {
            activities::archive_post(
                post_id,
                user.member_id.into_uuid(),
                user.is_admin,
                &self.deps,
            )
            .await
            .map(|_| ())
            .map_err(Into::into)
        })
        .await?;

        let post = Post::find_by_id(PostId::from_uuid(post_id), &self.deps.db_pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?
            .ok_or_else(|| TerminalError::new("Post not found after archive"))?;

        Ok(PostResult::from(post))
    }

    async fn expire(
        &self,
        ctx: ObjectContext<'_>,
        _req: EmptyRequest,
    ) -> Result<PostResult, HandlerError> {
        let user = require_admin(ctx.headers(), &self.deps.jwt_service)?;
        let post_id = Self::parse_post_id(ctx.key())?;

        ctx.run(|| async {
            activities::expire_post(
                post_id,
                user.member_id.into_uuid(),
                user.is_admin,
                &self.deps,
            )
            .await
            .map(|_| ())
            .map_err(Into::into)
        })
        .await?;

        let post = Post::find_by_id(PostId::from_uuid(post_id), &self.deps.db_pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?
            .ok_or_else(|| TerminalError::new("Post not found after expire"))?;

        Ok(PostResult::from(post))
    }

    async fn report(
        &self,
        ctx: ObjectContext<'_>,
        req: ReportPostRequest,
    ) -> Result<(), HandlerError> {
        let post_id = Self::parse_post_id(ctx.key())?;
        let user = optional_auth(ctx.headers(), &self.deps.jwt_service);

        ctx.run(|| async {
            activities::report_post(
                post_id,
                user.as_ref().map(|u| u.member_id.into_uuid()),
                req.reporter_email.clone(),
                req.reason.clone(),
                req.category.clone(),
                &self.deps,
            )
            .await
            .map(|_| ())
            .map_err(Into::into)
        })
        .await?;

        Ok(())
    }

    async fn resolve_report(
        &self,
        ctx: ObjectContext<'_>,
        req: ResolveReportRequest,
    ) -> Result<(), HandlerError> {
        let user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        ctx.run(|| async {
            activities::resolve_report(
                req.report_id,
                user.member_id.into_uuid(),
                user.is_admin,
                req.resolution_notes.clone(),
                req.action_taken.clone(),
                &self.deps,
            )
            .await
            .map_err(Into::into)
        })
        .await?;

        Ok(())
    }

    async fn dismiss_report(
        &self,
        ctx: ObjectContext<'_>,
        req: DismissReportRequest,
    ) -> Result<(), HandlerError> {
        let user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        ctx.run(|| async {
            activities::dismiss_report(
                req.report_id,
                user.member_id.into_uuid(),
                user.is_admin,
                req.resolution_notes.clone(),
                &self.deps,
            )
            .await
            .map_err(Into::into)
        })
        .await?;

        Ok(())
    }

    async fn update_tags(
        &self,
        ctx: ObjectContext<'_>,
        req: UpdateTagsRequest,
    ) -> Result<PostResult, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;
        let post_id = Self::parse_post_id(ctx.key())?;

        let tags: Vec<TagInput> = req
            .tags
            .into_iter()
            .map(|t| TagInput {
                kind: t.kind,
                value: t.value,
            })
            .collect();

        ctx.run(|| async {
            activities::tags::update_post_tags(post_id, tags.clone(), &self.deps.db_pool)
                .await
                .map(|_| ())
                .map_err(Into::into)
        })
        .await?;

        let post = Post::find_by_id(PostId::from_uuid(post_id), &self.deps.db_pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?
            .ok_or_else(|| TerminalError::new("Post not found after update_tags"))?;

        Ok(PostResult::from(post))
    }

    async fn add_tag(
        &self,
        ctx: ObjectContext<'_>,
        req: AddTagRequest,
    ) -> Result<(), HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;
        let post_id = Self::parse_post_id(ctx.key())?;

        ctx.run(|| async {
            activities::tags::add_post_tag(
                post_id,
                req.tag_kind.clone(),
                req.tag_value.clone(),
                req.display_name.clone(),
                &self.deps.db_pool,
            )
            .await
            .map(|_| ())
            .map_err(Into::into)
        })
        .await?;

        Ok(())
    }

    async fn remove_tag(
        &self,
        ctx: ObjectContext<'_>,
        req: RemoveTagRequest,
    ) -> Result<(), HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;
        let post_id = Self::parse_post_id(ctx.key())?;

        ctx.run(|| async {
            activities::tags::remove_post_tag(post_id, req.tag_id.clone(), &self.deps.db_pool)
                .await
                .map(|_| ())
                .map_err(Into::into)
        })
        .await?;

        Ok(())
    }

    async fn add_schedule(
        &self,
        ctx: ObjectContext<'_>,
        req: AddScheduleRequest,
    ) -> Result<ScheduleResult, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;
        let post_id = Self::parse_post_id(ctx.key())?;

        let params = ScheduleParams {
            dtstart: req.dtstart,
            dtend: req.dtend,
            rrule: req.rrule,
            exdates: req.exdates,
            opens_at: req.opens_at,
            closes_at: req.closes_at,
            day_of_week: req.day_of_week,
            timezone: req.timezone,
            is_all_day: req.is_all_day,
            duration_minutes: req.duration_minutes,
            notes: req.notes,
        };

        ctx.run(|| async {
            activities::schedule::add_post_schedule(post_id, params, &self.deps)
                .await
                .map(|_| ())
                .map_err(Into::into)
        })
        .await?;

        // Fetch the latest schedule for this post
        use crate::domains::schedules::models::Schedule;
        let schedules = Schedule::find_for_post(post_id, &self.deps.db_pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        let schedule = schedules
            .last()
            .ok_or_else(|| TerminalError::new("Schedule not found after add"))?;

        Ok(ScheduleResult {
            id: schedule.id.into_uuid(),
            schedulable_type: schedule.schedulable_type.clone(),
            schedulable_id: schedule.schedulable_id,
        })
    }

    async fn update_schedule(
        &self,
        ctx: ObjectContext<'_>,
        req: UpdateScheduleRequest,
    ) -> Result<ScheduleResult, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        let schedule_id = ScheduleId::from_uuid(req.schedule_id);
        let params = ScheduleParams {
            dtstart: req.dtstart,
            dtend: req.dtend,
            rrule: req.rrule,
            exdates: req.exdates,
            opens_at: req.opens_at,
            closes_at: req.closes_at,
            day_of_week: req.day_of_week,
            timezone: req.timezone,
            is_all_day: req.is_all_day,
            duration_minutes: req.duration_minutes,
            notes: req.notes,
        };

        ctx.run(|| async {
            activities::schedule::update_schedule(schedule_id, params, &self.deps)
                .await
                .map(|_| ())
                .map_err(Into::into)
        })
        .await?;

        use crate::domains::schedules::models::Schedule;
        let schedule = Schedule::find_by_id(schedule_id, &self.deps.db_pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        Ok(ScheduleResult {
            id: schedule.id.into_uuid(),
            schedulable_type: schedule.schedulable_type,
            schedulable_id: schedule.schedulable_id,
        })
    }

    async fn delete_schedule(
        &self,
        ctx: ObjectContext<'_>,
        req: DeleteScheduleRequest,
    ) -> Result<(), HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        let schedule_id = ScheduleId::from_uuid(req.schedule_id);

        ctx.run(|| async {
            activities::schedule::delete_schedule(schedule_id, &self.deps)
                .await
                .map_err(Into::into)
        })
        .await?;

        Ok(())
    }

    async fn track_view(
        &self,
        ctx: ObjectContext<'_>,
        _req: EmptyRequest,
    ) -> Result<(), HandlerError> {
        let post_id = Self::parse_post_id(ctx.key())?;

        ctx.run(|| async {
            activities::track_post_view(post_id, &self.deps)
                .await
                .map_err(Into::into)
        })
        .await?;

        Ok(())
    }

    async fn track_click(
        &self,
        ctx: ObjectContext<'_>,
        _req: EmptyRequest,
    ) -> Result<(), HandlerError> {
        let post_id = Self::parse_post_id(ctx.key())?;

        ctx.run(|| async {
            activities::track_post_click(post_id, &self.deps)
                .await
                .map_err(Into::into)
        })
        .await?;

        Ok(())
    }

    // --- Shared (concurrent reads) ---

    async fn get(
        &self,
        ctx: SharedObjectContext<'_>,
        _req: EmptyRequest,
    ) -> Result<PostResult, HandlerError> {
        let post_id = Uuid::parse_str(ctx.key())
            .map_err(|e| TerminalError::new(format!("Invalid post ID: {}", e)))?;

        let post = Post::find_by_id(PostId::from_uuid(post_id), &self.deps.db_pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?
            .ok_or_else(|| TerminalError::new("Post not found"))?;

        Ok(PostResult::from(post))
    }

    async fn get_reports(
        &self,
        ctx: SharedObjectContext<'_>,
        _req: EmptyRequest,
    ) -> Result<ReportListResult, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;
        let post_id = Uuid::parse_str(ctx.key())
            .map_err(|e| TerminalError::new(format!("Invalid post ID: {}", e)))?;

        let reports =
            PostReportRecord::query_for_post(PostId::from_uuid(post_id), &self.deps.db_pool)
                .await
                .map_err(|e| TerminalError::new(e.to_string()))?;

        Ok(ReportListResult {
            reports: reports
                .into_iter()
                .map(|r| ReportResult {
                    id: r.id.into_uuid(),
                    post_id: r.post_id.into_uuid(),
                    reason: r.reason,
                    category: r.category,
                    status: r.status,
                })
                .collect(),
        })
    }

    async fn get_revision(
        &self,
        ctx: SharedObjectContext<'_>,
        _req: EmptyRequest,
    ) -> Result<OptionalPostResult, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;
        let post_id = Self::parse_post_id(ctx.key())?;

        let revision = activities::revision_actions::get_revision_for_post(
            PostId::from_uuid(post_id),
            &self.deps.db_pool,
        )
        .await
        .map_err(|e| TerminalError::new(e.to_string()))?;

        Ok(OptionalPostResult {
            post: revision.map(PostResult::from),
        })
    }

    async fn generate_embedding(
        &self,
        ctx: ObjectContext<'_>,
        _req: EmptyRequest,
    ) -> Result<(), HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;
        let post_id = Self::parse_post_id(ctx.key())?;

        ctx.run(|| async {
            activities::post_operations::generate_post_embedding(
                PostId::from_uuid(post_id),
                self.deps.embedding_service.as_ref(),
                &self.deps.db_pool,
            )
            .await
            .map(|_| ())
            .map_err(Into::into)
        })
        .await?;

        Ok(())
    }

    async fn approve_revision(
        &self,
        ctx: ObjectContext<'_>,
        _req: EmptyRequest,
    ) -> Result<PostResult, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;
        let post_id = Self::parse_post_id(ctx.key())?;

        ctx.run(|| async {
            activities::revision_actions::approve_revision(
                PostId::from_uuid(post_id),
                &self.deps.db_pool,
            )
            .await
            .map(|_| ())
            .map_err(Into::into)
        })
        .await?;

        let post = Post::find_by_id(PostId::from_uuid(post_id), &self.deps.db_pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?
            .ok_or_else(|| TerminalError::new("Revision not found after approve"))?;

        Ok(PostResult::from(post))
    }

    async fn reject_revision(
        &self,
        ctx: ObjectContext<'_>,
        _req: EmptyRequest,
    ) -> Result<(), HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;
        let post_id = Self::parse_post_id(ctx.key())?;

        ctx.run(|| async {
            activities::revision_actions::reject_revision(
                PostId::from_uuid(post_id),
                &self.deps.db_pool,
            )
            .await
            .map_err(Into::into)
        })
        .await?;

        Ok(())
    }

    async fn regenerate(
        &self,
        ctx: ObjectContext<'_>,
        _req: EmptyRequest,
    ) -> Result<PostResult, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;
        let post_id = Self::parse_post_id(ctx.key())?;

        ctx.run(|| async {
            crate::domains::crawling::activities::regenerate_single_post(post_id, &self.deps)
                .await
                .map(|_| ())
                .map_err(Into::into)
        })
        .await?;

        let post = Post::find_by_id(PostId::from_uuid(post_id), &self.deps.db_pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?
            .ok_or_else(|| TerminalError::new("Post not found after regenerate"))?;

        Ok(PostResult::from(post))
    }
}
