//! Relay-style cursor-based pagination types
//!
//! Implements the GraphQL Cursor Connections Specification:
//! https://relay.dev/graphql/connections.htm
//!
//! # Usage
//!
//! ```rust,ignore
//! // In GraphQL query resolver
//! let args = PaginationArgs { first: Some(10), after: None, .. };
//! let validated = args.validate()?;
//!
//! // In model
//! let (items, has_more) = Model::find_paginated(&validated, pool).await?;
//!
//! // Build connection
//! let connection = build_connection(items, has_more, &validated, |item| item.id);
//! ```

use anyhow::{Context, Result};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use juniper::GraphQLObject;
use uuid::Uuid;

// ============================================================================
// Cursor
// ============================================================================

/// Opaque cursor for pagination (base64-encoded UUID).
///
/// V7 UUIDs are time-ordered, so using just the ID provides stable ordering.
#[derive(Debug, Clone)]
pub struct Cursor(Uuid);

impl Cursor {
    /// Create a cursor from a UUID.
    pub fn new(id: Uuid) -> Self {
        Cursor(id)
    }

    /// Encode the cursor as a base64 string.
    pub fn encode(&self) -> String {
        URL_SAFE_NO_PAD.encode(self.0.as_bytes())
    }

    /// Encode a UUID directly to a cursor string.
    pub fn encode_uuid(id: Uuid) -> String {
        Cursor::new(id).encode()
    }

    /// Decode a cursor string back to a Cursor.
    pub fn decode(s: &str) -> Result<Self> {
        let bytes = URL_SAFE_NO_PAD
            .decode(s)
            .context("Invalid cursor: not valid base64")?;
        let uuid = Uuid::from_slice(&bytes).context("Invalid cursor: not a valid UUID")?;
        Ok(Cursor(uuid))
    }

    /// Get the underlying UUID.
    pub fn into_uuid(self) -> Uuid {
        self.0
    }
}

// ============================================================================
// PageInfo (Relay spec)
// ============================================================================

/// Page information for cursor-based pagination.
///
/// Implements the Relay GraphQL Cursor Connections Specification.
#[derive(Debug, Clone, GraphQLObject)]
#[graphql(description = "Information about pagination in a connection")]
pub struct PageInfo {
    /// When paginating forwards, are there more items?
    pub has_next_page: bool,
    /// When paginating backwards, are there more items?
    pub has_previous_page: bool,
    /// Cursor of the first edge in the page.
    pub start_cursor: Option<String>,
    /// Cursor of the last edge in the page.
    pub end_cursor: Option<String>,
}

impl PageInfo {
    /// Create empty page info (no items).
    pub fn empty() -> Self {
        PageInfo {
            has_next_page: false,
            has_previous_page: false,
            start_cursor: None,
            end_cursor: None,
        }
    }
}

impl Default for PageInfo {
    fn default() -> Self {
        Self::empty()
    }
}

// ============================================================================
// Pagination Arguments
// ============================================================================

/// Direction of pagination.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaginationDirection {
    /// Forward pagination (first/after).
    Forward,
    /// Backward pagination (last/before).
    Backward,
}

/// Input arguments for cursor-based pagination.
///
/// Follows Relay spec: use either first/after (forward) or last/before (backward).
#[derive(Debug, Clone, Default)]
pub struct PaginationArgs {
    /// Returns the first n elements from the list.
    pub first: Option<i32>,
    /// Returns elements that come after the specified cursor.
    pub after: Option<String>,
    /// Returns the last n elements from the list.
    pub last: Option<i32>,
    /// Returns elements that come before the specified cursor.
    pub before: Option<String>,
}

impl PaginationArgs {
    /// Create forward pagination args.
    pub fn forward(first: i32, after: Option<String>) -> Self {
        PaginationArgs {
            first: Some(first),
            after,
            last: None,
            before: None,
        }
    }

    /// Create backward pagination args.
    pub fn backward(last: i32, before: Option<String>) -> Self {
        PaginationArgs {
            first: None,
            after: None,
            last: Some(last),
            before,
        }
    }

    /// Validate pagination arguments per Relay spec.
    ///
    /// Returns validated args with defaults applied and cursor decoded.
    pub fn validate(&self) -> Result<ValidatedPaginationArgs, &'static str> {
        // Can't use both forward and backward pagination
        if (self.first.is_some() || self.after.is_some())
            && (self.last.is_some() || self.before.is_some())
        {
            return Err("Cannot use first/after with last/before");
        }

        // Determine direction
        let direction = if self.last.is_some() || self.before.is_some() {
            PaginationDirection::Backward
        } else {
            PaginationDirection::Forward
        };

        // Get limit with defaults (25) and bounds (1-100)
        let limit = self.first.or(self.last).unwrap_or(25);
        let limit = limit.clamp(1, 100);

        // Get cursor for direction
        let cursor_str = match direction {
            PaginationDirection::Forward => self.after.as_ref(),
            PaginationDirection::Backward => self.before.as_ref(),
        };

        // Decode cursor if present
        let cursor = cursor_str
            .map(|c| Cursor::decode(c))
            .transpose()
            .map_err(|_| "Invalid cursor")?
            .map(|c| c.into_uuid());

        Ok(ValidatedPaginationArgs {
            limit,
            cursor,
            direction,
        })
    }
}

/// Validated and normalized pagination arguments.
#[derive(Debug, Clone)]
pub struct ValidatedPaginationArgs {
    /// Number of items to fetch (1-100, default 25).
    pub limit: i32,
    /// Cursor UUID (if provided).
    pub cursor: Option<Uuid>,
    /// Direction of pagination.
    pub direction: PaginationDirection,
}

impl ValidatedPaginationArgs {
    /// Get the SQL LIMIT value (limit + 1 to detect has_more).
    pub fn fetch_limit(&self) -> i64 {
        (self.limit + 1) as i64
    }

    /// Check if we're paginating forward.
    pub fn is_forward(&self) -> bool {
        self.direction == PaginationDirection::Forward
    }

    /// Check if we're paginating backward.
    pub fn is_backward(&self) -> bool {
        self.direction == PaginationDirection::Backward
    }
}

// ============================================================================
// Connection Builder Helpers
// ============================================================================

/// Build PageInfo from pagination results.
///
/// # Arguments
/// * `has_more` - Whether there are more items beyond the current page
/// * `args` - The validated pagination arguments
/// * `start_cursor` - Cursor of the first item in results
/// * `end_cursor` - Cursor of the last item in results
pub fn build_page_info(
    has_more: bool,
    args: &ValidatedPaginationArgs,
    start_cursor: Option<String>,
    end_cursor: Option<String>,
) -> PageInfo {
    match args.direction {
        PaginationDirection::Forward => PageInfo {
            has_next_page: has_more,
            has_previous_page: args.cursor.is_some(),
            start_cursor,
            end_cursor,
        },
        PaginationDirection::Backward => PageInfo {
            has_next_page: args.cursor.is_some(),
            has_previous_page: has_more,
            start_cursor,
            end_cursor,
        },
    }
}

/// Trim results to the requested limit and determine if there are more.
///
/// Database queries should fetch `limit + 1` items. This function trims
/// to the actual limit and returns whether there were more items.
pub fn trim_results<T>(results: Vec<T>, limit: i32) -> (Vec<T>, bool) {
    let has_more = results.len() > limit as usize;
    let results = if has_more {
        results.into_iter().take(limit as usize).collect()
    } else {
        results
    };
    (results, has_more)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cursor_encode_decode() {
        let id = Uuid::new_v4();
        let cursor = Cursor::new(id);
        let encoded = cursor.encode();
        let decoded = Cursor::decode(&encoded).unwrap();
        assert_eq!(id, decoded.into_uuid());
    }

    #[test]
    fn test_cursor_encode_uuid() {
        let id = Uuid::new_v4();
        let encoded = Cursor::encode_uuid(id);
        let decoded = Cursor::decode(&encoded).unwrap();
        assert_eq!(id, decoded.into_uuid());
    }

    #[test]
    fn test_pagination_args_validate_forward() {
        let args = PaginationArgs {
            first: Some(10),
            after: None,
            last: None,
            before: None,
        };
        let validated = args.validate().unwrap();
        assert_eq!(validated.limit, 10);
        assert!(validated.cursor.is_none());
        assert_eq!(validated.direction, PaginationDirection::Forward);
    }

    #[test]
    fn test_pagination_args_validate_backward() {
        let args = PaginationArgs {
            first: None,
            after: None,
            last: Some(5),
            before: None,
        };
        let validated = args.validate().unwrap();
        assert_eq!(validated.limit, 5);
        assert_eq!(validated.direction, PaginationDirection::Backward);
    }

    #[test]
    fn test_pagination_args_validate_defaults() {
        let args = PaginationArgs::default();
        let validated = args.validate().unwrap();
        assert_eq!(validated.limit, 25);
        assert_eq!(validated.direction, PaginationDirection::Forward);
    }

    #[test]
    fn test_pagination_args_validate_clamps() {
        let args = PaginationArgs {
            first: Some(200),
            ..Default::default()
        };
        let validated = args.validate().unwrap();
        assert_eq!(validated.limit, 100);

        let args = PaginationArgs {
            first: Some(0),
            ..Default::default()
        };
        let validated = args.validate().unwrap();
        assert_eq!(validated.limit, 1);
    }

    #[test]
    fn test_pagination_args_validate_rejects_mixed() {
        let args = PaginationArgs {
            first: Some(10),
            after: None,
            last: Some(5),
            before: None,
        };
        assert!(args.validate().is_err());
    }

    #[test]
    fn test_pagination_args_with_cursor() {
        let id = Uuid::new_v4();
        let cursor = Cursor::encode_uuid(id);
        let args = PaginationArgs {
            first: Some(10),
            after: Some(cursor),
            last: None,
            before: None,
        };
        let validated = args.validate().unwrap();
        assert_eq!(validated.cursor, Some(id));
    }

    #[test]
    fn test_trim_results() {
        let items: Vec<i32> = (1..=12).collect();
        let (trimmed, has_more) = trim_results(items, 10);
        assert_eq!(trimmed.len(), 10);
        assert!(has_more);

        let items: Vec<i32> = (1..=5).collect();
        let (trimmed, has_more) = trim_results(items, 10);
        assert_eq!(trimmed.len(), 5);
        assert!(!has_more);
    }
}
