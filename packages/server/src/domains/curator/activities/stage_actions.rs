use anyhow::Result;
use chrono::{NaiveDate, NaiveTime, Utc};
use tracing::{info, warn};
use uuid::Uuid;

use crate::common::PostId;
use crate::domains::curator::activities::note_proposal_handler::NoteProposalHandler;
use crate::domains::curator::models::{CuratorAction, StagingResult};
use crate::domains::contacts::models::{Contact, ContactType};
use crate::domains::locations::models::{Location, Locationable};
use crate::domains::notes::models::Note;
use crate::domains::posts::activities::post_sync_handler::PostProposalHandler;
use crate::domains::posts::models::{CreatePost, Post, PostSource};
use crate::domains::sync::activities::proposal_actions::ProposalHandler;
use crate::domains::schedules::models::{
    CreateOneOffSchedule, CreateOperatingHoursSchedule, CreateRecurringSchedule, Schedule,
};
use crate::domains::source::models::Source;
use crate::domains::sync::models::{SyncBatch, SyncProposal, SyncProposalMergeSource};
use crate::domains::tag::models::{Tag, Taggable};
use crate::kernel::ServerDeps;

/// The resource_type used for curator batches, distinct from the extraction
/// pipeline's "organization" type to prevent cross-contamination.
const CURATOR_RESOURCE_TYPE: &str = "curator";

/// Convert curator actions into sync proposals with draft entities.
///
/// `source_url_map` maps each source's site_url to its Source record,
/// used to create PostSource links so curator-created posts are visible
/// under the organization.
pub async fn stage_curator_actions(
    org_id: Uuid,
    actions: &[CuratorAction],
    org_summary: &str,
    source_url_map: &[(String, Source)],
    deps: &ServerDeps,
) -> Result<StagingResult> {
    let pool = &deps.db_pool;

    // Expire stale pending batches for this org — properly reject proposals
    // and clean up draft entities (posts, notes) to avoid orphaned rows.
    let stale = SyncBatch::find_stale(CURATOR_RESOURCE_TYPE, org_id, pool).await?;
    for batch in &stale {
        let pending = SyncProposal::find_pending_by_batch(batch.id, pool).await?;
        for proposal in &pending {
            let merge_sources =
                SyncProposalMergeSource::find_by_proposal(proposal.id, pool).await?;
            let reject_result = match proposal.entity_type.as_str() {
                "note" => NoteProposalHandler.reject(proposal, &merge_sources, pool).await,
                _ => PostProposalHandler.reject(proposal, &merge_sources, pool).await,
            };
            if let Err(e) = reject_result {
                warn!(proposal_id = %proposal.id, error = %e, "Draft cleanup failed during curator batch expiry");
            }
        }
        SyncProposal::reject_all_pending(batch.id, pool).await?;
        SyncBatch::update_status(batch.id, "expired", pool).await?;
    }
    if !stale.is_empty() {
        info!(
            org_id = %org_id,
            expired = stale.len(),
            "Expired stale curator batches with draft cleanup"
        );
    }

    // Stage all actions first, then create the batch with the actual count
    let mut staged = 0;
    let mut errors = Vec::new();

    // Create a temporary batch — we'll update its proposal_count after staging
    let batch = SyncBatch::create(
        CURATOR_RESOURCE_TYPE,
        Some(org_id),
        Some(&format!("Curator: {}", truncate(org_summary, 200))),
        0, // will be updated after staging
        pool,
    )
    .await?;

    for action in actions {
        let result = match action.action_type.as_str() {
            "create_post" => stage_create_post(action, org_id, &batch, source_url_map, deps).await,
            "update_post" => stage_update_post(action, &batch, pool).await,
            "add_note" => stage_add_note(action, org_id, &batch, pool).await,
            "merge_posts" => stage_merge_posts(action, &batch, pool).await,
            "archive_post" => stage_archive_post(action, &batch, pool).await,
            "flag_contradiction" => stage_flag_contradiction(action, org_id, &batch, pool).await,
            other => {
                warn!("Unknown curator action type: {}", other);
                continue;
            }
        };

        match result {
            Ok(()) => staged += 1,
            Err(e) => {
                warn!(action_type = action.action_type, error = %e, "Failed to stage curator action");
                errors.push(format!("{}: {}", action.action_type, e));
            }
        }
    }

    // Update batch with actual proposal count
    SyncBatch::update_proposal_count(batch.id, staged as i32, pool).await?;

    if staged == 0 && !actions.is_empty() {
        // All actions failed — expire the empty batch so it doesn't confuse users
        SyncBatch::update_status(batch.id, "expired", pool).await?;
        return Err(anyhow::anyhow!(
            "All {} curator actions failed to stage: {}",
            actions.len(),
            errors.join("; ")
        ));
    }

    info!(
        batch_id = %batch.id,
        staged = staged,
        total = actions.len(),
        errors = errors.len(),
        "Staged curator actions"
    );

    Ok(StagingResult {
        batch_id: batch.id.into(),
        proposals_staged: staged,
    })
}

async fn stage_create_post(
    action: &CuratorAction,
    _org_id: Uuid,
    batch: &SyncBatch,
    source_url_map: &[(String, Source)],
    deps: &ServerDeps,
) -> Result<()> {
    let pool = &deps.db_pool;

    // Create draft post
    let draft = Post::create(
        CreatePost::builder()
            .title(action.title.clone().unwrap_or_else(|| "Untitled".into()))
            .description(action.description.clone().unwrap_or_default())
            .summary(action.summary.clone())
            .post_type(action.post_type.clone().unwrap_or_else(|| "opportunity".into()))
            .category(action.category.clone().unwrap_or_else(|| "general".into()))
            .capacity_status(action.capacity_status.clone())
            .urgency(action.urgency.clone())
            .location(action.location.as_ref().and_then(|l| l.address.clone()))
            .status("draft".to_string())
            .submission_type(Some("agent".to_string()))
            .source_url(action.source_urls.first().cloned())
            .build(),
        pool,
    )
    .await?;

    let draft_uuid: Uuid = draft.id.into();

    // Link post to source(s) via post_sources so it appears under the organization.
    // Match each action source_url to the Source whose site_url is a prefix.
    let mut linked_sources = std::collections::HashSet::new();
    for page_url in &action.source_urls {
        if let Some((_, source)) = source_url_map
            .iter()
            .find(|(site_url, _)| page_url.starts_with(site_url.as_str()))
        {
            let source_uuid: Uuid = source.id.into();
            if linked_sources.insert(source_uuid) {
                if let Err(e) = PostSource::create(
                    draft.id,
                    &source.source_type,
                    source_uuid,
                    Some(page_url.as_str()),
                    pool,
                )
                .await
                {
                    warn!(post_id = %draft.id, source_id = %source_uuid, error = %e, "Failed to create post source link");
                }
            }
        }
    }
    // Fallback: if no source_urls matched, link to the first source so the post isn't orphaned
    if linked_sources.is_empty() {
        if let Some((_, source)) = source_url_map.first() {
            let source_uuid: Uuid = source.id.into();
            if let Err(e) = PostSource::create(
                draft.id,
                &source.source_type,
                source_uuid,
                action.source_urls.first().map(|s| s.as_str()),
                pool,
            )
            .await
            {
                warn!(post_id = %draft.id, error = %e, "Failed to create fallback post source link");
            }
        }
    }

    // Create location if provided
    if let Some(loc) = &action.location {
        if let Ok(location) = Location::find_or_create_from_extraction(
            loc.city.as_deref(),
            loc.state.as_deref(),
            loc.postal_code.as_deref(),
            loc.address.as_deref(),
            pool,
        )
        .await
        {
            let _ = Locationable::create(location.id, "post", draft_uuid, true, None, pool).await;
        }
    }

    // Create contacts
    if let Some(contacts) = &action.contacts {
        for (i, c) in contacts.iter().enumerate() {
            let ct = c.contact_type.parse::<ContactType>().unwrap_or(ContactType::Website);
            let _ = Contact::create(
                "post",
                draft_uuid,
                ct,
                c.value.clone(),
                c.label.clone(),
                Some(i as i32),
                pool,
            )
            .await;
        }
    }

    // Create schedules
    if let Some(schedules) = &action.schedule {
        for s in schedules {
            let _ = create_schedule_from_data("post", draft_uuid, s, pool).await;
        }
    }

    // Create schedule note (applies to whole schedule, linked to post via noteables)
    if let Some(schedule_note) = &action.schedule_notes {
        if !schedule_note.trim().is_empty() {
            if let Ok(note) = Note::create(
                schedule_note.trim(),
                "info",
                action.source_urls.first().map(|s| s.as_str()),
                None,
                Some("schedule"),
                true,
                "curator",
                None,
                pool,
            )
            .await
            {
                let _ = crate::domains::notes::models::Noteable::create(
                    note.id,
                    "post",
                    draft_uuid,
                    pool,
                )
                .await;
            }
        }
    }

    // Apply tags
    if let Some(tags) = &action.tags {
        for (kind, values) in tags {
            for value in values {
                if let Ok(tag) = Tag::find_or_create(kind, value, None, pool).await {
                    let _ = Taggable::create_post_tag(draft.id, tag.id, pool).await;
                }
            }
        }
    }

    // Generate embedding
    let embedding_text = draft.get_embedding_text();
    if let Ok(embedding) = deps.embedding_service.generate(&embedding_text).await {
        let _ = Post::update_embedding(draft.id, &embedding, pool).await;
    }

    SyncProposal::create_with_curator_fields(
        batch.id,
        "insert",
        "post",
        Some(draft_uuid),
        None,
        &action.reasoning,
        &action.confidence,
        &action.source_urls,
        pool,
    )
    .await?;

    Ok(())
}

async fn stage_update_post(
    action: &CuratorAction,
    batch: &SyncBatch,
    pool: &sqlx::PgPool,
) -> Result<()> {
    let target_id = parse_post_id(&action.target_post_id)?;
    let target_uuid: Uuid = target_id.into();

    // Create a revision post based on the original
    let original = Post::find_by_id(target_id, pool)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Target post not found: {}", target_uuid))?;
    let revision = Post::create(
        CreatePost::builder()
            .title(action.title.clone().unwrap_or_else(|| original.title.clone()))
            .description(
                action
                    .description
                    .clone()
                    .unwrap_or_else(|| original.description.clone()),
            )
            .summary(action.summary.clone().or_else(|| original.summary.clone()))
            .post_type(
                action
                    .post_type
                    .clone()
                    .unwrap_or_else(|| original.post_type.clone()),
            )
            .category(
                action
                    .category
                    .clone()
                    .unwrap_or_else(|| original.category.clone()),
            )
            .urgency(action.urgency.clone().or_else(|| original.urgency.clone()))
            .status("draft".to_string())
            .revision_of_post_id(Some(target_id))
            .source_url(action.source_urls.first().cloned())
            .build(),
        pool,
    )
    .await?;

    let revision_uuid: Uuid = revision.id.into();

    SyncProposal::create_with_curator_fields(
        batch.id,
        "update",
        "post",
        Some(revision_uuid),
        Some(target_uuid),
        &action.reasoning,
        &action.confidence,
        &action.source_urls,
        pool,
    )
    .await?;

    Ok(())
}

async fn stage_add_note(
    action: &CuratorAction,
    _org_id: Uuid,
    batch: &SyncBatch,
    pool: &sqlx::PgPool,
) -> Result<()> {
    let target_post_id = action
        .target_post_id
        .as_deref()
        .and_then(parse_post_id_optional);

    let note = Note::create_draft(
        action.note_content.as_deref().unwrap_or(""),
        action.note_severity.as_deref().unwrap_or("info"),
        action.source_urls.first().map(|s| s.as_str()),
        target_post_id.map(|id| id.into()),
        Some("curator"),
        None,
        pool,
    )
    .await?;

    let note_uuid: Uuid = note.id.into();

    SyncProposal::create_with_curator_fields(
        batch.id,
        "insert",
        "note",
        Some(note_uuid),
        None,
        &action.reasoning,
        &action.confidence,
        &action.source_urls,
        pool,
    )
    .await?;

    Ok(())
}

async fn stage_merge_posts(
    action: &CuratorAction,
    batch: &SyncBatch,
    pool: &sqlx::PgPool,
) -> Result<()> {
    let merge_ids: Vec<PostId> = action
        .merge_post_ids
        .as_ref()
        .map(|ids| {
            ids.iter()
                .filter_map(|id| parse_post_id_optional(id))
                .collect()
        })
        .unwrap_or_default();

    if merge_ids.len() < 2 {
        return Err(anyhow::anyhow!("Merge requires at least 2 posts"));
    }

    let canonical_uuid: Uuid = merge_ids[0].into();

    let proposal = SyncProposal::create_with_curator_fields(
        batch.id,
        "merge",
        "post",
        None,
        Some(canonical_uuid),
        &action.reasoning,
        &action.confidence,
        &action.source_urls,
        pool,
    )
    .await?;

    for source_id in &merge_ids[1..] {
        SyncProposalMergeSource::create(proposal.id, (*source_id).into(), pool).await?;
    }

    Ok(())
}

async fn stage_archive_post(
    action: &CuratorAction,
    batch: &SyncBatch,
    pool: &sqlx::PgPool,
) -> Result<()> {
    let target_id = parse_post_id(&action.target_post_id)?;
    let target_uuid: Uuid = target_id.into();

    SyncProposal::create_with_curator_fields(
        batch.id,
        "delete",
        "post",
        None,
        Some(target_uuid),
        &action.reasoning,
        &action.confidence,
        &action.source_urls,
        pool,
    )
    .await?;

    Ok(())
}

async fn stage_flag_contradiction(
    action: &CuratorAction,
    _org_id: Uuid,
    batch: &SyncBatch,
    pool: &sqlx::PgPool,
) -> Result<()> {
    let content = format!(
        "Contradiction detected: {}",
        action.contradiction_details.as_deref().unwrap_or("")
    );

    let note = Note::create_draft(
        &content,
        "urgent",
        action.source_urls.first().map(|s| s.as_str()),
        None,
        Some("curator"),
        None,
        pool,
    )
    .await?;

    let note_uuid: Uuid = note.id.into();

    SyncProposal::create_with_curator_fields(
        batch.id,
        "insert",
        "note",
        Some(note_uuid),
        None,
        &action.reasoning,
        &action.confidence,
        &action.source_urls,
        pool,
    )
    .await?;

    Ok(())
}

// ── Helpers ─────────────────────────────────────────────────────────────────

fn parse_post_id(id_ref: &Option<String>) -> Result<PostId> {
    let id_str = id_ref
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Missing target_post_id"))?;
    let uuid_str = id_str.strip_prefix("POST-").unwrap_or(id_str);
    let uuid = Uuid::parse_str(uuid_str)?;
    Ok(PostId::from(uuid))
}

fn parse_post_id_optional(id_str: &str) -> Option<PostId> {
    let uuid_str = id_str.strip_prefix("POST-").unwrap_or(id_str);
    Uuid::parse_str(uuid_str).ok().map(PostId::from)
}

fn truncate(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

/// Map ScheduleData into one of the three schedule creation modes.
async fn create_schedule_from_data(
    schedulable_type: &str,
    schedulable_id: Uuid,
    data: &crate::domains::curator::models::ScheduleData,
    pool: &sqlx::PgPool,
) -> Result<Schedule> {
    let tz = data.timezone.as_deref().unwrap_or("America/Chicago");

    // Guard: reject operating-hours entries missing closes_at — a single time on a
    // single day is almost certainly the LLM copying the org's general office hours
    // rather than a real schedule for this specific service.
    if data.day_of_week.is_some()
        && data.opens_at.is_some()
        && data.closes_at.is_none()
        && data.date.is_none()
        && data.rrule.is_none()
        && data.frequency.is_none()
    {
        warn!(
            day = data.day_of_week.as_deref().unwrap_or("?"),
            opens_at = data.opens_at.as_deref().unwrap_or("?"),
            "Skipping schedule with opens_at but no closes_at (likely generic org hours)"
        );
        return Err(anyhow::anyhow!(
            "Schedule has opens_at without closes_at — likely generic org hours, skipping"
        ));
    }

    // Mode 1: Operating hours (day_of_week + opens_at/closes_at, no date)
    if data.day_of_week.is_some() && data.opens_at.is_some() && data.date.is_none() {
        let dow = parse_day_of_week(data.day_of_week.as_deref().unwrap_or("monday"));
        return Schedule::create_operating_hours(
            &CreateOperatingHoursSchedule::builder()
                .schedulable_type(schedulable_type)
                .schedulable_id(schedulable_id)
                .day_of_week(dow)
                .timezone(tz)
                .opens_at(data.opens_at.as_deref().and_then(parse_time))
                .closes_at(data.closes_at.as_deref().and_then(parse_time))
                .build(),
            pool,
        )
        .await;
    }

    // Mode 2: Recurring (rrule or frequency set)
    if data.rrule.is_some() || data.frequency.is_some() {
        let rrule = data
            .rrule
            .clone()
            .unwrap_or_else(|| build_rrule_from_frequency(data));
        let dtstart = data
            .date
            .as_deref()
            .and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok())
            .map(|d| d.and_hms_opt(0, 0, 0).unwrap())
            .map(|dt| chrono::DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc))
            .unwrap_or_else(Utc::now);

        return Schedule::create_recurring(
            &CreateRecurringSchedule::builder()
                .schedulable_type(schedulable_type)
                .schedulable_id(schedulable_id)
                .dtstart(dtstart)
                .rrule(rrule.as_str())
                .timezone(tz)
                .duration_minutes(data.duration_minutes)
                .opens_at(data.start_time.as_deref().and_then(parse_time))
                .closes_at(data.end_time.as_deref().and_then(parse_time))
                .day_of_week(
                    data.day_of_week
                        .as_deref()
                        .map(|d| parse_day_of_week(d)),
                )
                .build(),
            pool,
        )
        .await;
    }

    // Mode 3: One-off event (date set)
    if let Some(date_str) = &data.date {
        let date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
            .unwrap_or_else(|_| Utc::now().date_naive());

        let start_time = data.start_time.as_deref().and_then(parse_time);
        let end_time = data.end_time.as_deref().and_then(parse_time);
        let is_all_day = data.is_all_day.unwrap_or(start_time.is_none());

        let dtstart = if let Some(t) = start_time {
            date.and_time(t)
        } else {
            date.and_hms_opt(0, 0, 0).unwrap()
        };
        let dtend = if let Some(t) = end_time {
            let end_date = data
                .date_end
                .as_deref()
                .and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok())
                .unwrap_or(date);
            end_date.and_time(t)
        } else {
            dtstart + chrono::Duration::hours(1)
        };

        return Schedule::create_one_off(
            &CreateOneOffSchedule::builder()
                .schedulable_type(schedulable_type)
                .schedulable_id(schedulable_id)
                .dtstart(chrono::DateTime::<Utc>::from_naive_utc_and_offset(dtstart, Utc))
                .dtend(chrono::DateTime::<Utc>::from_naive_utc_and_offset(dtend, Utc))
                .is_all_day(is_all_day)
                .timezone(tz)
                .build(),
            pool,
        )
        .await;
    }

    Err(anyhow::anyhow!("Could not determine schedule mode from data"))
}

fn parse_day_of_week(day: &str) -> i32 {
    match day.to_lowercase().as_str() {
        "sunday" | "sun" => 0,
        "monday" | "mon" => 1,
        "tuesday" | "tue" => 2,
        "wednesday" | "wed" => 3,
        "thursday" | "thu" => 4,
        "friday" | "fri" => 5,
        "saturday" | "sat" => 6,
        _ => 1, // default monday
    }
}

fn parse_time(s: &str) -> Option<NaiveTime> {
    NaiveTime::parse_from_str(s, "%H:%M")
        .or_else(|_| NaiveTime::parse_from_str(s, "%I:%M %p"))
        .ok()
}

fn build_rrule_from_frequency(data: &crate::domains::curator::models::ScheduleData) -> String {
    let freq = data.frequency.as_deref().unwrap_or("weekly");
    let freq_upper = freq.to_uppercase();
    let mut rrule = format!("FREQ={}", freq_upper);

    if let Some(day) = &data.day_of_week {
        let byday = match day.to_lowercase().as_str() {
            "sunday" | "sun" => "SU",
            "monday" | "mon" => "MO",
            "tuesday" | "tue" => "TU",
            "wednesday" | "wed" => "WE",
            "thursday" | "thu" => "TH",
            "friday" | "fri" => "FR",
            "saturday" | "sat" => "SA",
            _ => "MO",
        };
        rrule.push_str(&format!(";BYDAY={}", byday));
    }

    rrule
}
