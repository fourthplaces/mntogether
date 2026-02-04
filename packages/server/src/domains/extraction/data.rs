//! GraphQL data types for the extraction domain.
//!
//! These types wrap the extraction library types for GraphQL exposure.

use chrono::{DateTime, Utc};
use juniper::GraphQLEnum;

use crate::server::graphql::context::GraphQLContext;

// =============================================================================
// Extraction Status
// =============================================================================

/// Status of an extraction result
#[derive(Debug, Clone, Copy, GraphQLEnum)]
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
#[derive(Debug, Clone, Copy, GraphQLEnum)]
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
#[derive(Debug, Clone, Copy, GraphQLEnum)]
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
#[derive(Debug, Clone, juniper::GraphQLObject)]
#[graphql(context = GraphQLContext)]
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
#[derive(Debug, Clone, juniper::GraphQLObject)]
#[graphql(context = GraphQLContext)]
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
#[derive(Debug, Clone, juniper::GraphQLObject)]
#[graphql(context = GraphQLContext)]
pub struct ConflictData {
    /// Topic of the conflict
    pub topic: String,
    /// The conflicting claims
    pub claims: Vec<ConflictingClaimData>,
}

/// A single conflicting claim
#[derive(Debug, Clone, juniper::GraphQLObject)]
#[graphql(context = GraphQLContext)]
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
#[derive(Debug, Clone, juniper::GraphQLObject)]
#[graphql(context = GraphQLContext)]
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
#[derive(Debug, Clone, juniper::GraphQLObject)]
#[graphql(context = GraphQLContext)]
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
#[derive(Debug, Clone, juniper::GraphQLObject)]
#[graphql(context = GraphQLContext)]
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
// Input Types
// =============================================================================

/// Input for submitting a URL
#[derive(Debug, Clone, juniper::GraphQLInputObject)]
pub struct SubmitUrlInput {
    /// The URL to submit
    pub url: String,
    /// Optional: specific query to extract (default: "events, services, or opportunities")
    pub query: Option<String>,
}

/// Input for triggering an extraction
#[derive(Debug, Clone, juniper::GraphQLInputObject)]
pub struct TriggerExtractionInput {
    /// The extraction query
    pub query: String,
    /// Optional site filter
    pub site: Option<String>,
}
