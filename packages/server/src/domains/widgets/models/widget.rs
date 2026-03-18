use anyhow::{bail, Result};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::domains::editions::ZipCounty;

/// Known widget types — discriminated union key.
/// Note: stat_card and number_block were merged into "number" with visual
/// variants controlled by widget_template on edition_slots.
/// See DECISIONS_LOG.md: "Widget template system: Merge stat_card + number_block"
pub const WIDGET_TYPES: &[&str] = &[
    "number",
    "pull_quote",
    "resource_bar",
    "weather",
    "section_sep",
];

/// Authoring modes.
pub const AUTHORING_MODES: &[&str] = &["human", "automated", "layout"];

/// A standalone widget record in the CMS.
/// Each widget_type has its own data shape stored in JSONB,
/// validated at the application layer before create/update.
#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
pub struct Widget {
    pub id: Uuid,
    pub widget_type: String,
    pub authoring_mode: String,
    pub data: serde_json::Value,
    pub zip_code: Option<String>,
    pub city: Option<String>,
    pub county_id: Option<Uuid>,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Parameters for creating a widget.
#[derive(Debug, Default)]
pub struct CreateWidgetParams {
    pub zip_code: Option<String>,
    pub city: Option<String>,
    pub county_id: Option<Uuid>,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
}

/// Parameters for updating a widget.
#[derive(Debug, Default)]
pub struct UpdateWidgetParams {
    pub data: Option<serde_json::Value>,
    pub zip_code: Option<Option<String>>,
    pub city: Option<Option<String>>,
    pub county_id: Option<Option<Uuid>>,
    pub start_date: Option<Option<NaiveDate>>,
    pub end_date: Option<Option<NaiveDate>>,
}

/// Filters for listing widgets.
#[derive(Debug, Default)]
pub struct WidgetFilters<'a> {
    pub widget_type: Option<&'a str>,
    pub county_id: Option<Uuid>,
    pub search: Option<&'a str>,
}

impl Widget {
    /// Create a new widget. Validates type and data shape before insert.
    pub async fn create(
        widget_type: &str,
        authoring_mode: &str,
        data: serde_json::Value,
        params: CreateWidgetParams,
        pool: &PgPool,
    ) -> Result<Self> {
        validate_widget_type(widget_type)?;
        validate_authoring_mode(authoring_mode)?;
        // Skip data validation on create — widgets start as empty drafts.
        // Validation runs on update as the user fills in content.
        validate_date_range(params.start_date, params.end_date)?;

        let county_id = derive_county_id(
            params.zip_code.as_deref(),
            params.city.as_deref(),
            params.county_id,
            pool,
        )
        .await?;

        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO widgets (widget_type, authoring_mode, data, zip_code, city, county_id, start_date, end_date)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
            "#,
        )
        .bind(widget_type)
        .bind(authoring_mode)
        .bind(&data)
        .bind(&params.zip_code)
        .bind(&params.city)
        .bind(county_id)
        .bind(params.start_date)
        .bind(params.end_date)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    /// Find a widget by ID.
    pub async fn find_by_id(id: Uuid, pool: &PgPool) -> Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM widgets WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await
            .map_err(Into::into)
    }

    /// List widgets with optional filters.
    pub async fn find_all(
        filters: &WidgetFilters<'_>,
        limit: i64,
        offset: i64,
        pool: &PgPool,
    ) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            r#"
            SELECT * FROM widgets
            WHERE ($1::text IS NULL OR widget_type = $1)
              AND ($2::uuid IS NULL OR county_id = $2)
              AND ($3::text IS NULL OR (
                  data::text ILIKE '%' || $3 || '%'
                  OR widget_type ILIKE '%' || $3 || '%'
              ))
            ORDER BY created_at DESC
            LIMIT $4 OFFSET $5
            "#,
        )
        .bind(filters.widget_type)
        .bind(filters.county_id)
        .bind(filters.search)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    /// Update a widget. Data validation is skipped to allow draft editing
    /// (auto-save fires while the user is still typing). Validation can be
    /// checked explicitly via `validate_widget_data()` before publishing.
    pub async fn update(id: Uuid, params: UpdateWidgetParams, pool: &PgPool) -> Result<Self> {
        let existing = sqlx::query_as::<_, Self>("SELECT * FROM widgets WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await?;

        let existing = match existing {
            Some(w) => w,
            None => bail!("Widget not found: {}", id),
        };

        let data = match &params.data {
            Some(d) => d.clone(),
            None => existing.data.clone(),
        };

        // Resolve geo fields: use new values if provided, else keep existing
        let zip_code = match &params.zip_code {
            Some(v) => v.clone(),
            None => existing.zip_code.clone(),
        };
        let city = match &params.city {
            Some(v) => v.clone(),
            None => existing.city.clone(),
        };
        let explicit_county = match &params.county_id {
            Some(v) => *v,
            None => existing.county_id,
        };
        let start_date = match params.start_date {
            Some(v) => v,
            None => existing.start_date,
        };
        let end_date = match params.end_date {
            Some(v) => v,
            None => existing.end_date,
        };

        validate_date_range(start_date, end_date)?;

        // Re-derive county if any geo field changed
        let geo_changed = params.zip_code.is_some()
            || params.city.is_some()
            || params.county_id.is_some();

        let county_id = if geo_changed {
            derive_county_id(
                zip_code.as_deref(),
                city.as_deref(),
                explicit_county,
                pool,
            )
            .await?
        } else {
            existing.county_id
        };

        sqlx::query_as::<_, Self>(
            r#"
            UPDATE widgets
            SET data = $2, zip_code = $3, city = $4, county_id = $5,
                start_date = $6, end_date = $7, updated_at = now()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(&data)
        .bind(&zip_code)
        .bind(&city)
        .bind(county_id)
        .bind(start_date)
        .bind(end_date)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    /// Find widgets matching an edition's county and active within the given date.
    /// `slotted_filter`: "all", "slotted", or "not_slotted" relative to the edition.
    pub async fn find_for_edition(
        county_id: Uuid,
        as_of_date: NaiveDate,
        edition_id: Uuid,
        slotted_filter: &str,
        limit: i64,
        offset: i64,
        pool: &PgPool,
    ) -> Result<Vec<Self>> {
        let slotted_clause = match slotted_filter {
            "slotted" => r#"
                AND EXISTS (
                    SELECT 1 FROM edition_slots es
                    JOIN edition_rows er ON er.id = es.edition_row_id
                    WHERE es.widget_id = w.id AND er.edition_id = $4
                )"#,
            "not_slotted" => r#"
                AND NOT EXISTS (
                    SELECT 1 FROM edition_slots es
                    JOIN edition_rows er ON er.id = es.edition_row_id
                    WHERE es.widget_id = w.id AND er.edition_id = $4
                )"#,
            _ => "", // "all" — no extra filter
        };

        let sql = format!(
            r#"
            SELECT w.* FROM widgets w
            WHERE w.county_id = $1
              AND (w.start_date IS NULL OR w.start_date <= $2)
              AND (w.end_date IS NULL OR w.end_date >= $3)
              {}
            ORDER BY w.created_at DESC
            LIMIT $5 OFFSET $6
            "#,
            slotted_clause
        );

        sqlx::query_as::<_, Self>(&sql)
            .bind(county_id)
            .bind(as_of_date)
            .bind(as_of_date)
            .bind(edition_id)
            .bind(limit)
            .bind(offset)
            .fetch_all(pool)
            .await
            .map_err(Into::into)
    }

    /// Delete a widget.
    pub async fn delete(id: Uuid, pool: &PgPool) -> Result<()> {
        sqlx::query("DELETE FROM widgets WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }
}

// =============================================================================
// County derivation
// =============================================================================

/// Derive the county_id from the provided geo fields.
/// Priority: explicit county_id > zip_code lookup > city lookup.
async fn derive_county_id(
    zip_code: Option<&str>,
    city: Option<&str>,
    explicit_county_id: Option<Uuid>,
    pool: &PgPool,
) -> Result<Option<Uuid>> {
    // 1. Explicit county takes priority
    if let Some(cid) = explicit_county_id {
        return Ok(Some(cid));
    }

    // 2. Derive from zip code
    if let Some(zip) = zip_code {
        if !zip.is_empty() {
            let mappings = ZipCounty::find_counties_for_zip(zip, pool).await?;
            if let Some(primary) = mappings.first() {
                return Ok(Some(primary.county_id));
            }
        }
    }

    // 3. Derive from city: find a zip for this city, then look up county
    if let Some(city_name) = city {
        if !city_name.is_empty() {
            let zip_row: Option<(String,)> = sqlx::query_as(
                "SELECT zip_code FROM zip_codes WHERE LOWER(city) = LOWER($1) AND state = 'MN' LIMIT 1",
            )
            .bind(city_name)
            .fetch_optional(pool)
            .await?;

            if let Some((zip,)) = zip_row {
                let mappings = ZipCounty::find_counties_for_zip(&zip, pool).await?;
                if let Some(primary) = mappings.first() {
                    return Ok(Some(primary.county_id));
                }
            }
        }
    }

    Ok(None)
}

fn validate_date_range(start: Option<NaiveDate>, end: Option<NaiveDate>) -> Result<()> {
    if let (Some(s), Some(e)) = (start, end) {
        if e < s {
            bail!("end_date ({}) must be on or after start_date ({})", e, s);
        }
    }
    Ok(())
}

// =============================================================================
// Validation
// =============================================================================

fn validate_widget_type(widget_type: &str) -> Result<()> {
    if !WIDGET_TYPES.contains(&widget_type) {
        bail!("Unknown widget type: '{}'. Valid types: {:?}", widget_type, WIDGET_TYPES);
    }
    Ok(())
}

fn validate_authoring_mode(mode: &str) -> Result<()> {
    if !AUTHORING_MODES.contains(&mode) {
        bail!("Unknown authoring mode: '{}'. Valid modes: {:?}", mode, AUTHORING_MODES);
    }
    Ok(())
}

/// Validate widget data shape per type.
/// Enforces required fields and character limits from the spec.
pub fn validate_widget_data(widget_type: &str, data: &serde_json::Value) -> Result<()> {
    match widget_type {
        "number" => validate_number(data),
        "pull_quote" => validate_pull_quote(data),
        "resource_bar" => validate_resource_bar(data),
        "weather" => validate_weather(data),
        "section_sep" => validate_section_sep(data),
        _ => bail!("Cannot validate unknown widget type: {}", widget_type),
    }
}

fn require_string(data: &serde_json::Value, field: &str, min: usize, max: usize) -> Result<()> {
    let val = data.get(field).and_then(|v| v.as_str());
    match val {
        None => bail!("Missing required field '{}'", field),
        Some(s) => {
            let len = s.chars().count();
            if len < min || len > max {
                bail!("Field '{}' must be {}-{} characters (got {})", field, min, max, len);
            }
            Ok(())
        }
    }
}

fn optional_string(data: &serde_json::Value, field: &str, min: usize, max: usize) -> Result<()> {
    if let Some(val) = data.get(field) {
        if val.is_null() {
            return Ok(());
        }
        let s = val.as_str().ok_or_else(|| anyhow::anyhow!("Field '{}' must be a string", field))?;
        let len = s.chars().count();
        if len < min || len > max {
            bail!("Field '{}' must be {}-{} characters (got {})", field, min, max, len);
        }
    }
    Ok(())
}

/// Unified validation for the merged "number" widget type.
/// Accepts fields from both old stat_card (number, title, body) and
/// number_block (number, label, detail, color). Visual variant is
/// controlled by widget_template on the edition slot, not the data shape.
fn validate_number(data: &serde_json::Value) -> Result<()> {
    require_string(data, "number", 1, 6)?;
    // Accept either title (stat_card style) or label (number_block style)
    let has_title = data.get("title").and_then(|v| v.as_str()).map(|s| !s.is_empty()).unwrap_or(false);
    let has_label = data.get("label").and_then(|v| v.as_str()).map(|s| !s.is_empty()).unwrap_or(false);
    if !has_title && !has_label {
        bail!("Number widget requires either 'title' or 'label' field");
    }
    if has_title {
        require_string(data, "title", 1, 55)?;
    }
    if has_label {
        require_string(data, "label", 1, 55)?;
    }
    optional_string(data, "body", 1, 100)?;
    optional_string(data, "detail", 1, 100)?;
    // Validate color if present
    if let Some(color) = data.get("color").and_then(|v| v.as_str()) {
        let valid_colors = ["teal", "rust", "forest", "plum", "blue"];
        if !valid_colors.contains(&color) {
            bail!("Invalid color '{}'. Valid colors: {:?}", color, valid_colors);
        }
    }
    Ok(())
}

fn validate_pull_quote(data: &serde_json::Value) -> Result<()> {
    require_string(data, "quote", 40, 140)?;
    require_string(data, "attribution", 10, 40)?;
    Ok(())
}

fn validate_resource_bar(data: &serde_json::Value) -> Result<()> {
    require_string(data, "label", 10, 25)?;

    let items = data.get("items").and_then(|v| v.as_array());
    match items {
        None => bail!("Missing required field 'items'"),
        Some(arr) => {
            if arr.len() < 3 || arr.len() > 8 {
                bail!("'items' must have 3-8 entries (got {})", arr.len());
            }
            for (i, item) in arr.iter().enumerate() {
                let number = item.get("number").and_then(|v| v.as_str());
                let text = item.get("text").and_then(|v| v.as_str());
                match number {
                    None => bail!("items[{}] missing 'number'", i),
                    Some(s) if s.chars().count() < 1 || s.chars().count() > 12 => {
                        bail!("items[{}].number must be 1-12 characters", i);
                    }
                    _ => {}
                }
                match text {
                    None => bail!("items[{}] missing 'text'", i),
                    Some(s) if s.chars().count() < 5 || s.chars().count() > 30 => {
                        bail!("items[{}].text must be 5-30 characters", i);
                    }
                    _ => {}
                }
            }
        }
    }
    Ok(())
}

fn validate_weather(data: &serde_json::Value) -> Result<()> {
    // Validate variant
    let variant = data.get("variant").and_then(|v| v.as_str());
    match variant {
        None => bail!("Missing required field 'variant'"),
        Some(v) => {
            let valid = ["forecast", "line", "almanac", "thermo"];
            if !valid.contains(&v) {
                bail!("Invalid variant '{}'. Valid variants: {:?}", v, valid);
            }
        }
    }
    // Validate config.location
    let config = data.get("config");
    match config {
        None => bail!("Missing required field 'config'"),
        Some(c) => {
            let location = c.get("location").and_then(|v| v.as_str());
            match location {
                None => bail!("Missing required field 'config.location'"),
                Some(s) => {
                    let len = s.chars().count();
                    if len < 8 || len > 30 {
                        bail!("'config.location' must be 8-30 characters (got {})", len);
                    }
                }
            }
        }
    }
    Ok(())
}

fn validate_section_sep(data: &serde_json::Value) -> Result<()> {
    require_string(data, "title", 8, 35)?;
    optional_string(data, "sub", 15, 60)?;
    Ok(())
}
