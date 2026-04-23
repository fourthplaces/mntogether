//! Tag resolution for Root Signal ingest (spec §10).
//!
//! Ingest-time tag behaviour diverges by kind:
//!
//!   * `topic`        — open vocabulary. Unknown slug auto-creates and flips
//!                      the post to `in_review` for editor confirmation.
//!   * `service_area` — closed vocabulary (87 counties + `statewide`).
//!                      Unknown slug is a hard-fail (`unknown_service_area`).
//!                      Matches existing rows only.
//!   * `safety`       — reserved vocabulary. Unknown slug is a hard-fail
//!                      (`unknown_tag`). Matches existing rows only.
//!
//! On success, tags are attached to the post via `taggables`. This activity
//! doesn't write — it builds a `TagResolution` the orchestrator applies after
//! the post row itself lands.

use anyhow::Result;
use sqlx::PgPool;

use crate::api::error::{ErrorCode, FieldError};
use crate::common::TagId;
use crate::domains::tag::{Tag, Taggable};
use crate::common::PostId;

#[derive(Debug, Clone)]
pub struct ResolvedTag {
    pub id: TagId,
    pub kind: String,
    pub value: String,
    /// True if this is a freshly-auto-created topic slug (flips the post to
    /// `in_review`).
    pub auto_created: bool,
}

#[derive(Debug, Default)]
pub struct TagResolution {
    pub tags: Vec<ResolvedTag>,
    pub errors: Vec<FieldError>,
    /// Did at least one topic tag auto-create? Informs the orchestrator's
    /// soft-fail decision.
    pub unknown_topic_auto_created: bool,
    /// Resolved service_area slugs. The orchestrator mirrors these into the
    /// `service_areas` + `post_locations` tables alongside taggables.
    pub service_area_slugs: Vec<String>,
}

/// Resolve every tag on a submission. On hard-fail, `errors` is non-empty and
/// the orchestrator returns 422; on success, `tags` is applied to the post.
pub async fn resolve_tags(
    topics: &[String],
    service_areas: &[String],
    safety: &[String],
    pool: &PgPool,
) -> Result<TagResolution> {
    let mut out = TagResolution::default();

    // ----- service_area (closed) -----
    if service_areas.is_empty() {
        out.errors.push(FieldError::new(
            "tags.service_area",
            ErrorCode::MissingRequired,
            "at least one service_area tag required",
        ));
    }
    for slug in service_areas {
        match Tag::find_by_kind_value("service_area", slug, pool).await? {
            Some(tag) => {
                out.service_area_slugs.push(slug.clone());
                out.tags.push(ResolvedTag {
                    id: tag.id,
                    kind: tag.kind,
                    value: tag.value,
                    auto_created: false,
                });
            }
            None => out.errors.push(FieldError::new(
                "tags.service_area",
                ErrorCode::UnknownServiceArea,
                format!("unknown service_area '{slug}'"),
            )),
        }
    }

    // ----- topic (open, auto-create) -----
    if topics.is_empty() {
        out.errors.push(FieldError::new(
            "tags.topic",
            ErrorCode::MissingRequired,
            "at least one topic tag required",
        ));
    }
    for slug in topics {
        let existed = Tag::find_by_kind_value("topic", slug, pool).await?.is_some();
        let tag = Tag::find_or_create("topic", slug, None, pool).await?;
        if !existed {
            out.unknown_topic_auto_created = true;
        }
        out.tags.push(ResolvedTag {
            id: tag.id,
            kind: tag.kind,
            value: tag.value,
            auto_created: !existed,
        });
    }

    // ----- safety (closed reserved) -----
    for slug in safety {
        match Tag::find_by_kind_value("safety", slug, pool).await? {
            Some(tag) => out.tags.push(ResolvedTag {
                id: tag.id,
                kind: tag.kind,
                value: tag.value,
                auto_created: false,
            }),
            None => out.errors.push(FieldError::new(
                "tags.safety",
                ErrorCode::UnknownTag,
                format!("unknown safety tag '{slug}'"),
            )),
        }
    }

    Ok(out)
}

/// Apply a successful resolution: attach every tag to the post.
/// Called by the orchestrator after the `posts` row is inserted.
pub async fn apply_tags(
    post_id: PostId,
    resolution: &TagResolution,
    pool: &PgPool,
) -> Result<()> {
    for tag in &resolution.tags {
        Taggable::create_post_tag(post_id, tag.id, pool).await?;
    }
    Ok(())
}
