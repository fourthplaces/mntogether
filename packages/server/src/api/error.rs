use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;

/// Stable machine-readable error code. The ingest client matches on these; do
/// not rename without a contract version bump. See `ROOT_SIGNAL_API_REQUEST.md`
/// §11.3 and `ADDENDUM_01_CITATIONS_AND_SOURCE_METADATA.md` §6.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    MissingRequired,
    BelowMinLength,
    AboveMaxLength,
    UnknownValue,
    InvalidFormat,
    EditorOnlyField,
    PostTypeGroupMissing,
    SourceUrlRequired,
    OrganizationRequired,
    ConsentWithoutPlatformUrl,
    EditorialSourceForbidden,
    DuplicateBodyTier,
    UnknownTag,
    UnknownServiceArea,
    InvalidCoordinates,
    IdempotencyConflict,
    RateLimited,
    // Addendum 01
    TooManyCitations,
    CitationPrimaryMismatch,
    CitationHashFormat,
    CitationMissingRequired,
    CitationEditorialForbidden,
    InvalidRetrievedAt,
}

impl ErrorCode {
    pub fn as_str(self) -> &'static str {
        match self {
            ErrorCode::MissingRequired => "missing_required",
            ErrorCode::BelowMinLength => "below_min_length",
            ErrorCode::AboveMaxLength => "above_max_length",
            ErrorCode::UnknownValue => "unknown_value",
            ErrorCode::InvalidFormat => "invalid_format",
            ErrorCode::EditorOnlyField => "editor_only_field",
            ErrorCode::PostTypeGroupMissing => "post_type_group_missing",
            ErrorCode::SourceUrlRequired => "source_url_required",
            ErrorCode::OrganizationRequired => "organization_required",
            ErrorCode::ConsentWithoutPlatformUrl => "consent_without_platform_url",
            ErrorCode::EditorialSourceForbidden => "editorial_source_forbidden",
            ErrorCode::DuplicateBodyTier => "duplicate_body_tier",
            ErrorCode::UnknownTag => "unknown_tag",
            ErrorCode::UnknownServiceArea => "unknown_service_area",
            ErrorCode::InvalidCoordinates => "invalid_coordinates",
            ErrorCode::IdempotencyConflict => "idempotency_conflict",
            ErrorCode::RateLimited => "rate_limited",
            ErrorCode::TooManyCitations => "too_many_citations",
            ErrorCode::CitationPrimaryMismatch => "citation_primary_mismatch",
            ErrorCode::CitationHashFormat => "citation_hash_format",
            ErrorCode::CitationMissingRequired => "citation_missing_required",
            ErrorCode::CitationEditorialForbidden => "citation_editorial_forbidden",
            ErrorCode::InvalidRetrievedAt => "invalid_retrieved_at",
        }
    }
}

impl Serialize for ErrorCode {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.as_str())
    }
}

/// One entry in a 422 response. Always has all three fields populated; `field`
/// is a dotted JSON path into the submission.
#[derive(Debug, Clone, Serialize)]
pub struct FieldError {
    pub field: String,
    pub code: ErrorCode,
    pub detail: String,
}

impl FieldError {
    pub fn new(field: impl Into<String>, code: ErrorCode, detail: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            code,
            detail: detail.into(),
        }
    }
}

/// Accumulator for building up a validation response. Callers push as many
/// errors as they find, then convert via `into_result()` at the end so a 422
/// carries every problem in one round-trip.
#[derive(Debug, Default)]
pub struct FieldErrors(Vec<FieldError>);

impl FieldErrors {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, err: FieldError) {
        self.0.push(err);
    }

    pub fn extend(&mut self, errs: impl IntoIterator<Item = FieldError>) {
        self.0.extend(errs);
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn into_vec(self) -> Vec<FieldError> {
        self.0
    }

    /// Return `Ok(())` if empty, otherwise an `ApiError::Validation` wrapping
    /// every accumulated error. The intended pattern:
    ///
    ///     let mut errs = FieldErrors::new();
    ///     // … push errors as you find them …
    ///     errs.into_result()?;
    pub fn into_result(self) -> Result<(), ApiError> {
        if self.0.is_empty() {
            Ok(())
        } else {
            Err(ApiError::Validation(self.0))
        }
    }
}

/// Unified API error type.
///
/// Maps to HTTP status codes and returns either `{"message": "..."}` JSON
/// (most variants) or the structured `{"message": "...", "errors": [...]}`
/// shape for 422 validation failures per spec §11.1.
pub enum ApiError {
    Unauthorized(String),
    Forbidden(String),
    NotFound(String),
    BadRequest(String),
    /// 409 — same idempotency key, different payload (see spec §12.3).
    Conflict(String),
    /// 422 — one or more field-level validation failures.
    Validation(Vec<FieldError>),
    Internal(anyhow::Error),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        match self {
            ApiError::Unauthorized(msg) => (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({ "message": msg })),
            )
                .into_response(),
            ApiError::Forbidden(msg) => (
                StatusCode::FORBIDDEN,
                Json(serde_json::json!({ "message": msg })),
            )
                .into_response(),
            ApiError::NotFound(msg) => (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "message": msg })),
            )
                .into_response(),
            ApiError::BadRequest(msg) => (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "message": msg })),
            )
                .into_response(),
            ApiError::Conflict(msg) => (
                StatusCode::CONFLICT,
                Json(serde_json::json!({ "message": msg })),
            )
                .into_response(),
            ApiError::Validation(errors) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(serde_json::json!({
                    "message": "Validation failed",
                    "errors": errors,
                })),
            )
                .into_response(),
            ApiError::Internal(err) => {
                tracing::error!(error = %err, "Internal server error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "message": "Internal server error" })),
                )
                    .into_response()
            }
        }
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(err: anyhow::Error) -> Self {
        ApiError::Internal(err)
    }
}

pub type ApiResult<T> = Result<T, ApiError>;
