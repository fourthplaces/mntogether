use std::collections::HashMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// ── Page Brief Extraction (Map Step) ────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PageBriefExtraction {
    /// 2-3 sentence overview of what this page tells us about the organization
    pub summary: String,
    /// Full addresses found on the page
    pub locations: Vec<String>,
    /// Urgent needs, donation requests, volunteer asks
    pub calls_to_action: Vec<String>,
    /// Hours, eligibility, deadlines, closures, capacity
    pub critical_info: Option<String>,
    /// Programs, services, or opportunities offered
    pub services: Vec<String>,
    /// ALL contact methods found on the page
    pub contacts: Vec<BriefContact>,
    /// Operating hours, event times, recurring patterns
    pub schedules: Vec<BriefSchedule>,
    /// Languages services are offered in
    pub languages_mentioned: Vec<String>,
    /// Target populations served
    pub populations_mentioned: Vec<String>,
    /// Current capacity status if mentioned
    pub capacity_info: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BriefContact {
    /// "phone", "email", "website", "booking_url", "intake_form", "address"
    pub contact_type: String,
    pub value: String,
    /// "Main office", "After-hours", "Intake", etc.
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BriefSchedule {
    /// "operating_hours", "event", "recurring", "seasonal"
    pub schedule_type: String,
    /// Human-readable description, e.g. "Monday-Friday 9am-5pm"
    pub description: String,
    /// "monday,wednesday,friday" or "weekdays"
    pub days: Option<String>,
    /// "09:00-17:00" or "6:00 PM"
    pub times: Option<String>,
    /// "2026-03-15" for one-off events
    pub date: Option<String>,
    /// "weekly", "biweekly", "monthly"
    pub frequency: Option<String>,
    /// "September through May", "Summer only"
    pub seasonal_notes: Option<String>,
    /// "Closed holidays", "1st and 3rd week only"
    pub exceptions: Option<String>,
}

// ── Curator Action Types (Reduce Step) ───────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CuratorResponse {
    pub actions: Vec<CuratorAction>,
    /// Brief assessment of the org's current state
    pub org_summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CuratorAction {
    /// "create_post", "update_post", "add_note", "merge_posts", "archive_post", "flag_contradiction"
    pub action_type: String,
    /// Why this action is recommended
    pub reasoning: String,
    /// "high", "medium", "low"
    pub confidence: String,
    /// Which source pages support this action
    pub source_urls: Vec<String>,

    // Narrative content (create_post / update_post)
    pub title: Option<String>,
    pub summary: Option<String>,
    pub description: Option<String>,
    pub description_markdown: Option<String>,

    // Classification (create_post / update_post)
    pub post_type: Option<String>,
    pub category: Option<String>,
    pub urgency: Option<String>,
    pub capacity_status: Option<String>,

    // Structured data (create_post / update_post)
    pub location: Option<LocationData>,
    pub contacts: Option<Vec<ContactData>>,
    pub schedule: Option<Vec<ScheduleData>>,
    pub service_areas: Option<Vec<ServiceAreaData>>,

    /// Keys are tag_kind slugs: "audience_role", "population", "community_served", etc.
    pub tags: Option<HashMap<String, Vec<String>>>,

    /// Schedule-level note (applies to the whole schedule, not per row).
    /// E.g. "Closed holidays", "Hours change week to week — check Instagram"
    pub schedule_notes: Option<String>,

    /// References POST-{uuid} from org document (update_post / archive_post / add_note)
    pub target_post_id: Option<String>,
    /// POST-{uuid} list for merge_posts
    pub merge_post_ids: Option<Vec<String>>,

    // Note fields (add_note / flag_contradiction)
    pub note_content: Option<String>,
    /// "urgent", "notice", "info"
    pub note_severity: Option<String>,
    pub contradiction_details: Option<String>,
}

// ── Structured Data ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LocationData {
    pub address: Option<String>,
    pub address_line_2: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub postal_code: Option<String>,
    /// "physical", "virtual", "postal"
    pub location_type: Option<String>,
    pub accessibility_notes: Option<String>,
    pub transportation_notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ContactData {
    /// "phone", "email", "website", "address", "booking_url", "social"
    pub contact_type: String,
    pub value: String,
    /// "Main", "Booking", "Support", "Intake Form"
    pub label: Option<String>,
}

/// Supports three schedule modes:
/// 1. One-off event: date + start_time + end_time (or is_all_day)
/// 2. Recurring: frequency + day_of_week + start_time + end_time
/// 3. Operating hours: day_of_week + opens_at + closes_at
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ScheduleData {
    // One-off events
    pub date: Option<String>,
    pub date_end: Option<String>,

    // Recurring events
    pub frequency: Option<String>,
    pub day_of_week: Option<String>,
    /// iCalendar RRULE, e.g. "FREQ=WEEKLY;BYDAY=MO,WE,FR"
    pub rrule: Option<String>,

    // Operating hours (also uses day_of_week)
    /// "HH:MM" 24h format
    pub opens_at: Option<String>,
    /// "HH:MM" 24h format
    pub closes_at: Option<String>,

    // Common fields
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub is_all_day: Option<bool>,
    pub duration_minutes: Option<i32>,
    /// Default "America/Chicago"
    pub timezone: Option<String>,
    /// Seasonal start "YYYY-MM-DD"
    pub valid_from: Option<String>,
    /// Seasonal end "YYYY-MM-DD"
    pub valid_to: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ServiceAreaData {
    /// "county", "city", "state", "zip", "custom"
    pub area_type: String,
    pub area_name: String,
    /// FIPS code, ZIP code, state abbreviation
    pub area_code: Option<String>,
}

// ── Org Document Metadata ───────────────────────────────────────────────────

pub struct OrgDocument {
    pub content: String,
    pub token_estimate: usize,
    pub briefs_included: usize,
    pub posts_included: usize,
    pub notes_included: usize,
}

// ── Staging Result ──────────────────────────────────────────────────────────

pub struct StagingResult {
    pub batch_id: uuid::Uuid,
    pub proposals_staged: usize,
}
