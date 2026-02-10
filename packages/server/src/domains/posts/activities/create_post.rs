//! Post creation action
//!
//! Centralized logic for creating posts with all associated data
//! (contact info, audience role tags, page source links).

use anyhow::Result;
use chrono::{NaiveDate, NaiveTime, Utc};
use sqlx::PgPool;
use std::collections::HashMap;
use tracing::warn;
use uuid::Uuid;

use crate::common::{ContactInfo, ExtractedPost, ExtractedSchedule, PostId};
use crate::domains::locations::models::Location;
use crate::domains::posts::models::PostLocation;
use crate::domains::contacts::Contact;
use crate::domains::posts::models::{CreatePost, Post};
use crate::domains::schedules::models::Schedule;
use crate::domains::tag::models::{Tag, Taggable};

/// Valid urgency values per database constraint
const VALID_URGENCY_VALUES: &[&str] = &["low", "medium", "high", "urgent"];

/// Normalize urgency value to a valid database value.
/// Returns None if the input is invalid or None.
fn normalize_urgency(urgency: Option<&str>) -> Option<String> {
    urgency.and_then(|u| {
        let normalized = u.to_lowercase();
        if VALID_URGENCY_VALUES.contains(&normalized.as_str()) {
            Some(normalized)
        } else {
            warn!(urgency = %u, "Invalid urgency value from AI, ignoring");
            None
        }
    })
}

/// Create a post from extracted data with all associated records.
///
/// This is the single place that handles:
/// - Creating the post record
/// - Creating contact info records
/// - Tagging with audience roles
/// - Linking to source page snapshot
///
/// All sync functions should use this instead of calling Post::create directly.
pub async fn create_extracted_post(
    post: &ExtractedPost,
    source_type: Option<&str>,
    source_id: Option<uuid::Uuid>,
    source_url: Option<String>,
    submitted_by_id: Option<uuid::Uuid>,
    pool: &PgPool,
) -> Result<Post> {
    use crate::domains::posts::models::PostSource;

    let urgency = normalize_urgency(post.urgency.as_deref());

    // Create the post
    let created = Post::create(
        CreatePost::builder()
            .title(post.title.clone())
            .description(post.description.clone())
            .summary(Some(post.summary.clone()))
            .capacity_status(Some("accepting".to_string()))
            .urgency(urgency)
            .location(post.location.clone())
            .submission_type(Some("scraped".to_string()))
            .source_url(source_url.clone())
            .submitted_by_id(submitted_by_id)
            .build(),
        pool,
    )
    .await?;

    // Link to source via post_sources
    if let (Some(st), Some(sid)) = (source_type, source_id) {
        PostSource::create(created.id, st, sid, source_url.as_deref(), pool).await?;
    }

    // Create contact info if available
    if let Some(ref contact) = post.contact {
        save_contact_info(created.id, contact, pool).await;
    }

    // Tag post with dynamic extracted tags (includes audience_role)
    tag_post_from_extracted(created.id, &post.tags, pool).await;

    // Create structured location if zip/city/state available
    if post.zip_code.is_some() || post.city.is_some() {
        create_post_location(&created, post, pool).await;
    }

    // Save schedule entries
    for sched in &post.schedule {
        save_schedule(created.id, sched, pool).await;
    }

    // Link post to source page snapshot
    if let Some(page_snapshot_id) = post.source_page_snapshot_id {
        link_to_page_source(created.id, page_snapshot_id, pool).await;
    }

    Ok(created)
}

/// Save contact info for a post.
pub async fn save_contact_info(post_id: PostId, contact: &ContactInfo, pool: &PgPool) {
    let contact_json = serde_json::json!({
        "phone": contact.phone,
        "email": contact.email,
        "website": contact.website,
        "intake_form_url": contact.intake_form_url,
        "contact_name": contact.contact_name,
    });

    if let Err(e) = Contact::create_from_json_for_post(post_id, &contact_json, pool).await {
        warn!(
            post_id = %post_id,
            error = %e,
            "Failed to save contact info"
        );
    }
}

/// Create a Location and link it to the post.
async fn create_post_location(post: &Post, extracted: &ExtractedPost, pool: &PgPool) {
    let location = Location::find_or_create_from_extraction(
        extracted.city.as_deref(),
        extracted.state.as_deref(),
        extracted.zip_code.as_deref(),
        None,
        pool,
    )
    .await;

    match location {
        Ok(loc) => {
            if let Err(e) = PostLocation::create(post.id, loc.id, true, None, pool).await {
                warn!(
                    post_id = %post.id,
                    location_id = %loc.id,
                    error = %e,
                    "Failed to link post to location"
                );
            }
        }
        Err(e) => {
            warn!(
                post_id = %post.id,
                error = %e,
                "Failed to create location from extraction"
            );
        }
    }
}

/// Apply all extracted tags to a post.
///
/// Looks up each value in the database; warns if not found (admin controls vocabulary).
pub async fn tag_post_from_extracted(
    post_id: PostId,
    tags: &HashMap<String, Vec<String>>,
    pool: &PgPool,
) {
    for (kind, values) in tags {
        for value in values {
            let normalized = value.to_lowercase();
            match Tag::find_by_kind_value(kind, &normalized, pool).await {
                Ok(Some(tag)) => {
                    if let Err(e) = Taggable::create_post_tag(post_id, tag.id, pool).await {
                        warn!(
                            post_id = %post_id,
                            kind = %kind,
                            value = %normalized,
                            error = %e,
                            "Failed to tag post with extracted tag"
                        );
                    }
                }
                Ok(None) => {
                    warn!(kind = %kind, value = %normalized, "Unknown tag value from AI, skipping");
                }
                Err(e) => {
                    warn!(kind = %kind, value = %normalized, error = %e, "Tag lookup failed");
                }
            }
        }
    }
}

/// Parse day_of_week string to i32 (0=Sunday, 1=Monday, ..., 6=Saturday)
fn parse_day_of_week(s: &str) -> Option<i32> {
    match s.to_lowercase().as_str() {
        "sunday" => Some(0),
        "monday" => Some(1),
        "tuesday" => Some(2),
        "wednesday" => Some(3),
        "thursday" => Some(4),
        "friday" => Some(5),
        "saturday" => Some(6),
        _ => {
            warn!(day = %s, "Invalid day_of_week from LLM");
            None
        }
    }
}

/// Convert day_of_week integer to RRULE day abbreviation
fn day_abbr(day: i32) -> &'static str {
    match day {
        0 => "SU",
        1 => "MO",
        2 => "TU",
        3 => "WE",
        4 => "TH",
        5 => "FR",
        6 => "SA",
        _ => "MO",
    }
}

/// Parse time string to NaiveTime
fn parse_time(s: &str) -> Option<NaiveTime> {
    NaiveTime::parse_from_str(s, "%H:%M")
        .or_else(|_| NaiveTime::parse_from_str(s, "%k:%M"))
        .ok()
        .or_else(|| {
            warn!(time = %s, "Invalid time from LLM");
            None
        })
}

/// Parse date string to NaiveDate
fn parse_date(s: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").ok().or_else(|| {
        warn!(date = %s, "Invalid date from LLM");
        None
    })
}

/// Replace all schedules for a post with fresh extraction data.
///
/// Deletes existing schedules and saves new ones. Used when updating/regenerating
/// an existing post with fresh extraction that includes schedule data.
pub async fn sync_schedules_for_post(post_id: PostId, schedules: &[ExtractedSchedule], pool: &PgPool) {
    if schedules.is_empty() {
        return;
    }
    // Clear existing schedules for this post before saving new ones
    if let Err(e) = Schedule::delete_all_for_entity("post", post_id.into_uuid(), pool).await {
        warn!(post_id = %post_id, error = %e, "Failed to clear existing schedules");
    }
    for sched in schedules {
        save_schedule(post_id, sched, pool).await;
    }
}

/// Save a single extracted schedule entry for a post.
async fn save_schedule(post_id: PostId, sched: &ExtractedSchedule, pool: &PgPool) {
    let tz = "America/Chicago";
    let notes = sched.notes.as_deref();
    let post_uuid = post_id.into_uuid();

    let result = match sched.frequency.as_str() {
        "weekly" => {
            let Some(day) = sched.day_of_week.as_deref().and_then(parse_day_of_week) else {
                warn!(frequency = "weekly", "Missing or invalid day_of_week for weekly schedule, skipping");
                return;
            };
            let opens = sched.start_time.as_deref().and_then(parse_time);
            let closes = sched.end_time.as_deref().and_then(parse_time);

            Schedule::create_operating_hours("post", post_uuid, day, opens, closes, tz, notes, pool).await
        }
        "biweekly" => {
            let Some(day) = sched.day_of_week.as_deref().and_then(parse_day_of_week) else {
                warn!(frequency = "biweekly", "Missing or invalid day_of_week, skipping");
                return;
            };
            let opens = sched.start_time.as_deref().and_then(parse_time);
            let closes = sched.end_time.as_deref().and_then(parse_time);
            let rrule = format!("FREQ=WEEKLY;INTERVAL=2;BYDAY={}", day_abbr(day));
            let duration = match (opens, closes) {
                (Some(o), Some(c)) => {
                    let diff = c.signed_duration_since(o);
                    Some(diff.num_minutes() as i32)
                }
                _ => None,
            };

            Schedule::create_recurring(
                "post", post_uuid, Utc::now(), &rrule, duration,
                opens, closes, Some(day), tz, notes, pool,
            ).await
        }
        "monthly" => {
            let Some(day) = sched.day_of_week.as_deref().and_then(parse_day_of_week) else {
                warn!(frequency = "monthly", "Missing or invalid day_of_week, skipping");
                return;
            };
            let opens = sched.start_time.as_deref().and_then(parse_time);
            let closes = sched.end_time.as_deref().and_then(parse_time);
            let rrule = format!("FREQ=MONTHLY;BYDAY=1{}", day_abbr(day));
            let duration = match (opens, closes) {
                (Some(o), Some(c)) => {
                    let diff = c.signed_duration_since(o);
                    Some(diff.num_minutes() as i32)
                }
                _ => None,
            };

            Schedule::create_recurring(
                "post", post_uuid, Utc::now(), &rrule, duration,
                opens, closes, Some(day), tz, notes, pool,
            ).await
        }
        "one_time" => {
            let Some(date) = sched.date.as_deref().and_then(parse_date) else {
                warn!(frequency = "one_time", "Missing or invalid date for one-off event, skipping");
                return;
            };
            let start_time = sched.start_time.as_deref().and_then(parse_time)
                .unwrap_or_else(|| NaiveTime::from_hms_opt(0, 0, 0).unwrap());
            let end_time = sched.end_time.as_deref().and_then(parse_time)
                .unwrap_or_else(|| NaiveTime::from_hms_opt(23, 59, 0).unwrap());
            let is_all_day = sched.start_time.is_none() && sched.end_time.is_none();

            let dtstart = date.and_time(start_time).and_utc();
            let dtend = date.and_time(end_time).and_utc();

            Schedule::create_one_off("post", post_uuid, dtstart, dtend, is_all_day, tz, notes, pool).await
        }
        other => {
            // Default to weekly if frequency is unknown but we have a day
            if let Some(day) = sched.day_of_week.as_deref().and_then(parse_day_of_week) {
                let opens = sched.start_time.as_deref().and_then(parse_time);
                let closes = sched.end_time.as_deref().and_then(parse_time);
                warn!(frequency = %other, "Unknown frequency from LLM, defaulting to weekly");
                Schedule::create_operating_hours("post", post_uuid, day, opens, closes, tz, notes, pool).await
            } else {
                warn!(frequency = %other, "Unknown frequency and no day_of_week, skipping");
                return;
            }
        }
    };

    match result {
        Ok(created) => {
            // Validate the generated rrule if present
            if let Some(ref rrule_str) = created.rrule {
                let test = format!("DTSTART:20260101T000000Z\nRRULE:{}", rrule_str);
                if test.parse::<rrule::RRuleSet>().is_err() {
                    warn!(rrule = %rrule_str, post_id = %post_id, "Generated rrule failed to parse, deleting schedule");
                    let _ = Schedule::delete(created.id, pool).await;
                }
            }
        }
        Err(e) => {
            warn!(
                post_id = %post_id,
                frequency = %sched.frequency,
                error = %e,
                "Failed to save schedule"
            );
        }
    }
}

/// Link a post to its source page snapshot.
async fn link_to_page_source(post_id: PostId, page_snapshot_id: Uuid, pool: &PgPool) {
    if let Err(e) = sqlx::query(
        "INSERT INTO post_page_sources (post_id, page_snapshot_id) VALUES ($1, $2) ON CONFLICT DO NOTHING"
    )
    .bind(post_id.into_uuid())
    .bind(page_snapshot_id)
    .execute(pool)
    .await
    {
        warn!(
            post_id = %post_id,
            page_snapshot_id = %page_snapshot_id,
            error = %e,
            "Failed to link post to page source"
        );
    }
}
