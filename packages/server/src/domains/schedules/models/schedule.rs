use anyhow::Result;
use chrono::{DateTime, Duration, NaiveDate, NaiveTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::ScheduleId;

/// A schedule entry for posts, locations, or post_locations (polymorphic).
///
/// Supports three modes:
/// - **One-off event**: `dtstart`/`dtend` set, `rrule` NULL
/// - **Recurring event**: `dtstart` + `rrule` set, occurrences expanded by rrule crate
/// - **Operating hours**: `day_of_week` + `opens_at`/`closes_at` + weekly rrule
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Schedule {
    pub id: ScheduleId,
    pub schedulable_type: String,
    pub schedulable_id: Uuid,
    pub day_of_week: Option<i32>,
    pub opens_at: Option<NaiveTime>,
    pub closes_at: Option<NaiveTime>,
    pub timezone: String,
    pub valid_from: Option<NaiveDate>,
    pub valid_to: Option<NaiveDate>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub dtstart: Option<DateTime<Utc>>,
    pub dtend: Option<DateTime<Utc>>,
    pub rrule: Option<String>,
    pub exdates: Option<String>,
    pub is_all_day: bool,
    pub duration_minutes: Option<i32>,
    pub updated_at: DateTime<Utc>,
}

impl Schedule {
    /// Batch-load schedules for multiple posts (for DataLoader)
    pub async fn find_for_post_ids(post_ids: &[Uuid], pool: &PgPool) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            r#"
            SELECT * FROM schedules
            WHERE schedulable_type = 'post' AND schedulable_id = ANY($1)
            ORDER BY day_of_week ASC NULLS LAST, opens_at ASC NULLS LAST, dtstart ASC NULLS LAST
            "#,
        )
        .bind(post_ids)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    pub async fn find_by_id(id: ScheduleId, pool: &PgPool) -> Result<Self> {
        sqlx::query_as::<_, Self>("SELECT * FROM schedules WHERE id = $1")
            .bind(id)
            .fetch_one(pool)
            .await
            .map_err(Into::into)
    }

    /// Find all schedules for an entity (post, location, or post_location)
    pub async fn find_for_entity(
        schedulable_type: &str,
        schedulable_id: Uuid,
        pool: &PgPool,
    ) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            r#"
            SELECT * FROM schedules
            WHERE schedulable_type = $1 AND schedulable_id = $2
            ORDER BY day_of_week ASC NULLS LAST, opens_at ASC NULLS LAST, dtstart ASC NULLS LAST
            "#,
        )
        .bind(schedulable_type)
        .bind(schedulable_id)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    /// Find all schedules for a post
    pub async fn find_for_post(post_id: Uuid, pool: &PgPool) -> Result<Vec<Self>> {
        Self::find_for_entity("post", post_id, pool).await
    }

    /// Find all schedules for a location
    pub async fn find_for_location(location_id: Uuid, pool: &PgPool) -> Result<Vec<Self>> {
        Self::find_for_entity("location", location_id, pool).await
    }

    /// Expand this schedule's next N occurrences using rrule (computed, not stored).
    pub fn next_occurrences(&self, limit: usize) -> Vec<DateTime<Utc>> {
        let now = Utc::now();
        let window_end = now + Duration::days(90);

        // One-off event: dtstart set, no rrule
        if self.dtstart.is_some() && self.rrule.is_none() {
            return self.dtstart.into_iter().filter(|d| *d >= now).collect();
        }

        let Some(ref rrule_str) = self.rrule else {
            return vec![];
        };

        let full = if let Some(dtstart) = self.dtstart {
            format!(
                "DTSTART:{}\nRRULE:{}",
                dtstart.format("%Y%m%dT%H%M%SZ"),
                rrule_str
            )
        } else {
            format!(
                "DTSTART:{}\nRRULE:{}",
                now.format("%Y%m%dT%H%M%SZ"),
                rrule_str
            )
        };

        let Ok(rrule_set) = full.parse::<rrule::RRuleSet>() else {
            return vec![];
        };

        let start = now.with_timezone(&rrule::Tz::UTC);
        let end = window_end.with_timezone(&rrule::Tz::UTC);

        let result = rrule_set.after(start).before(end).all(limit as u16);

        // Apply exdates filter
        let exdates = parse_exdates(&self.exdates);

        result
            .dates
            .into_iter()
            .filter(|dt| {
                let utc_dt: DateTime<Utc> = dt.with_timezone(&Utc);
                !exdates.iter().any(|ex| ex.date_naive() == utc_dt.date_naive())
            })
            .map(|d| d.with_timezone(&Utc))
            .collect()
    }

    /// Create a one-off event schedule (e.g. workshop on Mar 15 2-4pm)
    pub async fn create_one_off(
        schedulable_type: &str,
        schedulable_id: Uuid,
        dtstart: DateTime<Utc>,
        dtend: DateTime<Utc>,
        is_all_day: bool,
        timezone: &str,
        notes: Option<&str>,
        pool: &PgPool,
    ) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO schedules (
                schedulable_type, schedulable_id, dtstart, dtend, is_all_day,
                timezone, notes
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING *
            "#,
        )
        .bind(schedulable_type)
        .bind(schedulable_id)
        .bind(dtstart)
        .bind(dtend)
        .bind(is_all_day)
        .bind(timezone)
        .bind(notes)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    /// Create a recurring event schedule (e.g. ESL class every Tue 6-8pm)
    pub async fn create_recurring(
        schedulable_type: &str,
        schedulable_id: Uuid,
        dtstart: DateTime<Utc>,
        rrule: &str,
        duration_minutes: Option<i32>,
        opens_at: Option<NaiveTime>,
        closes_at: Option<NaiveTime>,
        day_of_week: Option<i32>,
        timezone: &str,
        notes: Option<&str>,
        pool: &PgPool,
    ) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO schedules (
                schedulable_type, schedulable_id, dtstart, rrule,
                duration_minutes, opens_at, closes_at, day_of_week,
                timezone, notes
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING *
            "#,
        )
        .bind(schedulable_type)
        .bind(schedulable_id)
        .bind(dtstart)
        .bind(rrule)
        .bind(duration_minutes)
        .bind(opens_at)
        .bind(closes_at)
        .bind(day_of_week)
        .bind(timezone)
        .bind(notes)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    /// Create an operating hours schedule (e.g. Mon 9am-5pm)
    pub async fn create_operating_hours(
        schedulable_type: &str,
        schedulable_id: Uuid,
        day_of_week: i32,
        opens_at: Option<NaiveTime>,
        closes_at: Option<NaiveTime>,
        timezone: &str,
        notes: Option<&str>,
        pool: &PgPool,
    ) -> Result<Self> {
        let day_abbr = match day_of_week {
            0 => "SU",
            1 => "MO",
            2 => "TU",
            3 => "WE",
            4 => "TH",
            5 => "FR",
            6 => "SA",
            _ => return Err(anyhow::anyhow!("Invalid day_of_week: {}", day_of_week)),
        };
        let rrule = format!("FREQ=WEEKLY;BYDAY={}", day_abbr);

        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO schedules (
                schedulable_type, schedulable_id, day_of_week,
                opens_at, closes_at, rrule, timezone, notes
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
            "#,
        )
        .bind(schedulable_type)
        .bind(schedulable_id)
        .bind(day_of_week)
        .bind(opens_at)
        .bind(closes_at)
        .bind(&rrule)
        .bind(timezone)
        .bind(notes)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    pub async fn update(
        id: ScheduleId,
        dtstart: Option<DateTime<Utc>>,
        dtend: Option<DateTime<Utc>>,
        rrule: Option<&str>,
        exdates: Option<&str>,
        opens_at: Option<NaiveTime>,
        closes_at: Option<NaiveTime>,
        day_of_week: Option<i32>,
        is_all_day: Option<bool>,
        duration_minutes: Option<i32>,
        timezone: Option<&str>,
        notes: Option<&str>,
        pool: &PgPool,
    ) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            UPDATE schedules SET
                dtstart = COALESCE($2, dtstart),
                dtend = COALESCE($3, dtend),
                rrule = COALESCE($4, rrule),
                exdates = COALESCE($5, exdates),
                opens_at = COALESCE($6, opens_at),
                closes_at = COALESCE($7, closes_at),
                day_of_week = COALESCE($8, day_of_week),
                is_all_day = COALESCE($9, is_all_day),
                duration_minutes = COALESCE($10, duration_minutes),
                timezone = COALESCE($11, timezone),
                notes = COALESCE($12, notes),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(dtstart)
        .bind(dtend)
        .bind(rrule)
        .bind(exdates)
        .bind(opens_at)
        .bind(closes_at)
        .bind(day_of_week)
        .bind(is_all_day)
        .bind(duration_minutes)
        .bind(timezone)
        .bind(notes)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    pub async fn delete(id: ScheduleId, pool: &PgPool) -> Result<()> {
        sqlx::query("DELETE FROM schedules WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Delete all schedules for an entity
    pub async fn delete_all_for_entity(
        schedulable_type: &str,
        schedulable_id: Uuid,
        pool: &PgPool,
    ) -> Result<()> {
        sqlx::query("DELETE FROM schedules WHERE schedulable_type = $1 AND schedulable_id = $2")
            .bind(schedulable_type)
            .bind(schedulable_id)
            .execute(pool)
            .await?;
        Ok(())
    }
}

/// Parse exdates from comma-separated ISO date strings.
fn parse_exdates(exdates: &Option<String>) -> Vec<DateTime<Utc>> {
    let Some(ref s) = exdates else {
        return vec![];
    };
    s.split(',')
        .filter_map(|part| {
            let trimmed = part.trim();
            if trimmed.is_empty() {
                return None;
            }
            trimmed
                .parse::<DateTime<Utc>>()
                .ok()
                .or_else(|| {
                    chrono::NaiveDate::parse_from_str(trimmed, "%Y-%m-%d")
                        .ok()
                        .and_then(|d| d.and_hms_opt(0, 0, 0).map(|dt| dt.and_utc()))
                })
        })
        .collect()
}
