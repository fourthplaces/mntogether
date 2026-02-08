//! Data types for the extraction domain.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// =============================================================================
// Extraction Status
// =============================================================================

/// Status of an extraction result
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ExtractionStatusData {
    /// Information was found and extracted
    Found,
    /// Some information found, but gaps remain
    Partial,
    /// No matching information found
    Missing,
    /// Conflicting information from different sources
    Contradictory,
}

impl From<extraction::ExtractionStatus> for ExtractionStatusData {
    fn from(status: extraction::ExtractionStatus) -> Self {
        match status {
            extraction::ExtractionStatus::Found => Self::Found,
            extraction::ExtractionStatus::Partial => Self::Partial,
            extraction::ExtractionStatus::Missing => Self::Missing,
            extraction::ExtractionStatus::Contradictory => Self::Contradictory,
        }
    }
}

// =============================================================================
// Grounding Grade
// =============================================================================

/// How well-grounded the extraction is in sources
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum GroundingGradeData {
    /// Multiple sources agree
    Verified,
    /// Only one source
    SingleSource,
    /// Sources conflict
    Conflicted,
    /// Information was inferred
    Inferred,
}

impl From<extraction::GroundingGrade> for GroundingGradeData {
    fn from(grade: extraction::GroundingGrade) -> Self {
        match grade {
            extraction::GroundingGrade::Verified => Self::Verified,
            extraction::GroundingGrade::SingleSource => Self::SingleSource,
            extraction::GroundingGrade::Conflicted => Self::Conflicted,
            extraction::GroundingGrade::Inferred => Self::Inferred,
        }
    }
}

// =============================================================================
// Source Role
// =============================================================================

/// Role a source played in the extraction
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum SourceRoleData {
    /// Primary source of information
    Primary,
    /// Supporting/confirming source
    Supporting,
    /// Corroborating source
    Corroborating,
}

impl From<extraction::SourceRole> for SourceRoleData {
    fn from(role: extraction::SourceRole) -> Self {
        match role {
            extraction::SourceRole::Primary => Self::Primary,
            extraction::SourceRole::Supporting => Self::Supporting,
            extraction::SourceRole::Corroborating => Self::Corroborating,
        }
    }
}

// =============================================================================
// Source
// =============================================================================

/// A source used in an extraction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceData {
    /// URL of the source
    pub url: String,
    /// Title of the page (if available)
    pub title: Option<String>,
    /// When the source was fetched
    pub fetched_at: DateTime<Utc>,
    /// Role this source played
    pub role: SourceRoleData,
}

impl From<extraction::Source> for SourceData {
    fn from(source: extraction::Source) -> Self {
        Self {
            url: source.url,
            title: source.title,
            fetched_at: source.fetched_at,
            role: source.role.into(),
        }
    }
}

// =============================================================================
// Gap Query
// =============================================================================

/// Information that couldn't be found
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GapData {
    /// What field/information is missing
    pub field: String,
    /// Suggested search query to find it
    pub suggested_query: String,
    /// Whether this gap is searchable online
    pub is_searchable: bool,
}

impl From<extraction::GapQuery> for GapData {
    fn from(gap: extraction::GapQuery) -> Self {
        // Check searchability before moving
        let is_searchable = gap.is_searchable();
        Self {
            field: gap.field,
            suggested_query: gap.query,
            is_searchable,
        }
    }
}

// =============================================================================
// Conflict
// =============================================================================

/// Conflicting information from different sources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictData {
    /// Topic of the conflict
    pub topic: String,
    /// The conflicting claims
    pub claims: Vec<ConflictingClaimData>,
}

/// A single conflicting claim
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictingClaimData {
    /// The statement being made
    pub statement: String,
    /// URL of the source making this claim
    pub source_url: String,
}

impl From<extraction::Conflict> for ConflictData {
    fn from(conflict: extraction::Conflict) -> Self {
        Self {
            topic: conflict.topic,
            claims: conflict
                .claims
                .into_iter()
                .map(ConflictingClaimData::from)
                .collect(),
        }
    }
}

impl From<extraction::ConflictingClaim> for ConflictingClaimData {
    fn from(claim: extraction::ConflictingClaim) -> Self {
        Self {
            statement: claim.statement,
            source_url: claim.source_url,
        }
    }
}

// =============================================================================
// Extraction Result
// =============================================================================

/// Result of an extraction query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionData {
    /// Extracted content (markdown formatted with citations)
    pub content: String,
    /// Status of the extraction
    pub status: ExtractionStatusData,
    /// How well-grounded the extraction is
    pub grounding: GroundingGradeData,
    /// Sources used in the extraction
    pub sources: Vec<SourceData>,
    /// Information that couldn't be found
    pub gaps: Vec<GapData>,
    /// Conflicting information (if any)
    pub conflicts: Vec<ConflictData>,
}

impl From<extraction::Extraction> for ExtractionData {
    fn from(extraction: extraction::Extraction) -> Self {
        Self {
            content: extraction.content,
            status: extraction.status.into(),
            grounding: extraction.grounding.into(),
            sources: extraction
                .sources
                .into_iter()
                .map(SourceData::from)
                .collect(),
            gaps: extraction.gaps.into_iter().map(GapData::from).collect(),
            conflicts: extraction
                .conflicts
                .into_iter()
                .map(ConflictData::from)
                .collect(),
        }
    }
}

// =============================================================================
// Submit URL Result
// =============================================================================

/// Result of submitting a URL for extraction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitUrlResult {
    /// Whether the submission was successful
    pub success: bool,
    /// The URL that was submitted
    pub url: String,
    /// Extraction result (if available immediately)
    pub extraction: Option<ExtractionData>,
    /// Error message (if failed)
    pub error: Option<String>,
}

// =============================================================================
// Trigger Extraction Result
// =============================================================================

/// Result of triggering an extraction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerExtractionResult {
    /// Whether the extraction was successful
    pub success: bool,
    /// The query that was run
    pub query: String,
    /// Site filter (if any)
    pub site: Option<String>,
    /// Extraction results
    pub extractions: Vec<ExtractionData>,
    /// Error message (if failed)
    pub error: Option<String>,
}

// =============================================================================
// Extraction Page (from extraction_pages table)
// =============================================================================

/// A crawled page from the extraction library's storage.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
pub struct ExtractionPageRow {
    pub url: String,
    pub site_url: String,
    pub content: String,
    pub content_hash: String,
    pub fetched_at: DateTime<Utc>,
    pub title: Option<String>,
    pub metadata: serde_json::Value,
}

/// API representation of an extraction page
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExtractionPageData {
    pub url: String,
    pub site_url: String,
    pub content: String,
    pub title: Option<String>,
    pub fetched_at: String,
}

impl From<ExtractionPageRow> for ExtractionPageData {
    fn from(row: ExtractionPageRow) -> Self {
        Self {
            url: row.url,
            site_url: row.site_url,
            content: row.content,
            title: row.title,
            fetched_at: row.fetched_at.to_rfc3339(),
        }
    }
}

impl ExtractionPageData {
    /// Find a page by URL
    pub async fn find_by_url(url: &str, pool: &sqlx::PgPool) -> anyhow::Result<Option<Self>> {
        let row = sqlx::query_as::<_, ExtractionPageRow>(
            "SELECT url, site_url, content, content_hash, fetched_at, title, metadata FROM extraction_pages WHERE url = $1"
        )
        .bind(url)
        .fetch_optional(pool)
        .await?;
        Ok(row.map(Self::from))
    }

    /// Find pages by domain/site_url
    pub async fn find_by_domain(
        domain: &str,
        limit: i32,
        pool: &sqlx::PgPool,
    ) -> anyhow::Result<Vec<Self>> {
        let normalized = domain
            .trim_start_matches("https://")
            .trim_start_matches("http://");

        let https_prefix = format!("https://{}", normalized);
        let http_prefix = format!("http://{}", normalized);

        let rows = sqlx::query_as::<_, ExtractionPageRow>(
            r#"
            SELECT url, site_url, content, content_hash, fetched_at, title, metadata
            FROM extraction_pages
            WHERE site_url = $1 OR site_url = $2
            ORDER BY fetched_at DESC
            LIMIT $3
            "#,
        )
        .bind(&https_prefix)
        .bind(&http_prefix)
        .bind(limit as i64)
        .fetch_all(pool)
        .await?;

        Ok(rows.into_iter().map(Self::from).collect())
    }

    /// Count pages for a domain
    pub async fn count_by_domain(domain: &str, pool: &sqlx::PgPool) -> anyhow::Result<i32> {
        let normalized = domain
            .trim_start_matches("https://")
            .trim_start_matches("http://");

        let https_prefix = format!("https://{}", normalized);
        let http_prefix = format!("http://{}", normalized);

        let count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM extraction_pages WHERE site_url = $1 OR site_url = $2",
        )
        .bind(&https_prefix)
        .bind(&http_prefix)
        .fetch_one(pool)
        .await?;

        Ok(count.0 as i32)
    }
}

// =============================================================================
// Input Types
// =============================================================================

/// Input for submitting a URL
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitUrlInput {
    /// The URL to submit
    pub url: String,
    /// Optional: specific query to extract (default: "events, services, or opportunities")
    pub query: Option<String>,
}

/// Input for triggering an extraction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerExtractionInput {
    /// The extraction query
    pub query: String,
    /// Optional site filter
    pub site: Option<String>,
}
