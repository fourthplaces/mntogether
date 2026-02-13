//! Schedule activities for posts - create/update/delete schedules

use anyhow::Result;
use chrono::{NaiveTime, Utc};
use uuid::Uuid;

use crate::common::ScheduleId;
use crate::domains::schedules::models::{
    CreateOneOffSchedule, CreateOperatingHoursSchedule, CreateRecurringSchedule, Schedule,
    UpdateScheduleParams,
};
use crate::kernel::ServerDeps;

/// Input for creating or updating a schedule
pub struct ScheduleParams {
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

/// Add a schedule to a post.
///
/// Determines the schedule type (recurring, operating hours, one-off) from the input
/// and creates the appropriate record.
pub async fn add_post_schedule(
    post_id: Uuid,
    input: ScheduleParams,
    deps: &ServerDeps,
) -> Result<Schedule> {
    let timezone = input.timezone.as_deref().unwrap_or("America/Chicago");

    if let Some(ref rrule) = input.rrule {
        // Recurring schedule
        let dtstart = input
            .dtstart
            .as_ref()
            .map(|s| s.parse::<chrono::DateTime<Utc>>())
            .transpose()?;

        let opens_at = input
            .opens_at
            .as_ref()
            .map(|s| NaiveTime::parse_from_str(s, "%H:%M"))
            .transpose()?;

        let closes_at = input
            .closes_at
            .as_ref()
            .map(|s| NaiveTime::parse_from_str(s, "%H:%M"))
            .transpose()?;

        Schedule::create_recurring(
            &CreateRecurringSchedule::builder()
                .schedulable_type("post")
                .schedulable_id(post_id)
                .dtstart(dtstart.unwrap_or_else(Utc::now))
                .rrule(rrule.as_str())
                .timezone(timezone)
                .duration_minutes(input.duration_minutes)
                .opens_at(opens_at)
                .closes_at(closes_at)
                .day_of_week(input.day_of_week)
                .notes(input.notes.as_deref())
                .build(),
            &deps.db_pool,
        )
        .await
    } else if input.day_of_week.is_some() && input.dtstart.is_none() {
        // Operating hours
        let opens_at = input
            .opens_at
            .as_ref()
            .map(|s| NaiveTime::parse_from_str(s, "%H:%M"))
            .transpose()?;

        let closes_at = input
            .closes_at
            .as_ref()
            .map(|s| NaiveTime::parse_from_str(s, "%H:%M"))
            .transpose()?;

        Schedule::create_operating_hours(
            &CreateOperatingHoursSchedule::builder()
                .schedulable_type("post")
                .schedulable_id(post_id)
                .day_of_week(input.day_of_week.unwrap())
                .timezone(timezone)
                .opens_at(opens_at)
                .closes_at(closes_at)
                .notes(input.notes.as_deref())
                .build(),
            &deps.db_pool,
        )
        .await
    } else {
        // One-off event
        let dtstart = input
            .dtstart
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("dtstart is required for one-off events"))?
            .parse::<chrono::DateTime<Utc>>()?;

        let dtend = input
            .dtend
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("dtend is required for one-off events"))?
            .parse::<chrono::DateTime<Utc>>()?;

        let is_all_day = input.is_all_day.unwrap_or(false);

        Schedule::create_one_off(
            &CreateOneOffSchedule::builder()
                .schedulable_type("post")
                .schedulable_id(post_id)
                .dtstart(dtstart)
                .dtend(dtend)
                .is_all_day(is_all_day)
                .timezone(timezone)
                .notes(input.notes.as_deref())
                .build(),
            &deps.db_pool,
        )
        .await
    }
}

/// Update an existing schedule.
pub async fn update_schedule(
    schedule_id: ScheduleId,
    input: ScheduleParams,
    deps: &ServerDeps,
) -> Result<Schedule> {
    let dtstart = input
        .dtstart
        .as_ref()
        .map(|s| s.parse::<chrono::DateTime<Utc>>())
        .transpose()?;

    let dtend = input
        .dtend
        .as_ref()
        .map(|s| s.parse::<chrono::DateTime<Utc>>())
        .transpose()?;

    let opens_at = input
        .opens_at
        .as_ref()
        .map(|s| NaiveTime::parse_from_str(s, "%H:%M"))
        .transpose()?;

    let closes_at = input
        .closes_at
        .as_ref()
        .map(|s| NaiveTime::parse_from_str(s, "%H:%M"))
        .transpose()?;

    Schedule::update(
        schedule_id,
        &UpdateScheduleParams {
            dtstart,
            dtend,
            rrule: input.rrule.as_deref(),
            exdates: input.exdates.as_deref(),
            opens_at,
            closes_at,
            day_of_week: input.day_of_week,
            is_all_day: input.is_all_day,
            duration_minutes: input.duration_minutes,
            timezone: input.timezone.as_deref(),
            notes: input.notes.as_deref(),
        },
        &deps.db_pool,
    )
    .await
}

/// Delete a schedule.
pub async fn delete_schedule(schedule_id: ScheduleId, deps: &ServerDeps) -> Result<()> {
    Schedule::delete(schedule_id, &deps.db_pool).await
}
