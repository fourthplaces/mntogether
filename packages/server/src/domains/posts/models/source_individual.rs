//! Individual authors cited as post sources.
//!
//! Schema lives in migration 000237 as `source_individuals`. Dedup ladder
//! (spec §7.2) is:
//!
//!   1. `already_known_individual_id` provided by the client → direct lookup.
//!   2. `(platform, handle)` exact match.
//!   3. `platform_url` match.
//!   4. Insert.
//!
//! On match, NULL columns on the stored row are filled in from the
//! submission. Conflicting non-NULL values are surfaced by the caller as a
//! `source_stale` soft-fail; the update isn't applied silently.

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::common::SourceIndividualId;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct SourceIndividual {
    pub id: SourceIndividualId,
    pub display_name: String,
    pub handle: Option<String>,
    pub platform: Option<String>,
    pub platform_url: Option<String>,
    pub verified_identity: bool,
    pub consent_to_publish: bool,
    pub consent_source: Option<String>,
    pub consent_captured_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Input for creating or enriching a `source_individuals` row.
#[derive(Debug, Clone)]
pub struct SourceIndividualInput {
    pub display_name: String,
    pub handle: Option<String>,
    pub platform: Option<String>,
    pub platform_url: Option<String>,
    pub verified_identity: bool,
    pub consent_to_publish: bool,
    pub consent_source: Option<String>,
    pub consent_captured_at: Option<DateTime<Utc>>,
}

impl SourceIndividual {
    pub async fn find_by_id(
        id: SourceIndividualId,
        pool: &PgPool,
    ) -> Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM source_individuals WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await
            .map_err(Into::into)
    }

    /// Dedup step 2: `(platform, handle)` exact match. Leading `@` on handle
    /// is stripped before the lookup so `@jamielocal` and `jamielocal` resolve
    /// to the same row.
    pub async fn find_by_platform_handle(
        platform: &str,
        handle: &str,
        pool: &PgPool,
    ) -> Result<Option<Self>> {
        let handle = handle.trim_start_matches('@');
        sqlx::query_as::<_, Self>(
            r#"
            SELECT * FROM source_individuals
            WHERE platform = $1 AND handle = $2
            LIMIT 1
            "#,
        )
        .bind(platform)
        .bind(handle)
        .fetch_optional(pool)
        .await
        .map_err(Into::into)
    }

    /// Dedup step 3: platform_url lookup.
    pub async fn find_by_platform_url(
        platform_url: &str,
        pool: &PgPool,
    ) -> Result<Option<Self>> {
        sqlx::query_as::<_, Self>(
            r#"
            SELECT * FROM source_individuals
            WHERE platform_url = $1
            LIMIT 1
            "#,
        )
        .bind(platform_url)
        .fetch_optional(pool)
        .await
        .map_err(Into::into)
    }

    /// Insert a new individual. Handle's leading `@` is normalised away.
    pub async fn insert(input: SourceIndividualInput, pool: &PgPool) -> Result<Self> {
        let handle = input
            .handle
            .as_deref()
            .map(|h| h.trim_start_matches('@').to_string());

        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO source_individuals (
                display_name, handle, platform, platform_url,
                verified_identity, consent_to_publish,
                consent_source, consent_captured_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
            "#,
        )
        .bind(input.display_name)
        .bind(handle)
        .bind(input.platform)
        .bind(input.platform_url)
        .bind(input.verified_identity)
        .bind(input.consent_to_publish)
        .bind(input.consent_source)
        .bind(input.consent_captured_at)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    /// Enrich a matched row: fill NULL columns from the submission. Does not
    /// overwrite non-NULL columns (those go through the `source_stale` path
    /// at a higher level). `verified_identity` and `consent_to_publish` are
    /// monotonic — once true, stay true.
    pub async fn enrich(
        id: SourceIndividualId,
        input: &SourceIndividualInput,
        pool: &PgPool,
    ) -> Result<Self> {
        let handle = input
            .handle
            .as_deref()
            .map(|h| h.trim_start_matches('@').to_string());

        sqlx::query_as::<_, Self>(
            r#"
            UPDATE source_individuals
            SET
                handle              = COALESCE(handle, $2),
                platform            = COALESCE(platform, $3),
                platform_url        = COALESCE(platform_url, $4),
                verified_identity   = verified_identity OR $5,
                consent_to_publish  = consent_to_publish OR $6,
                consent_source      = COALESCE(consent_source, $7),
                consent_captured_at = COALESCE(consent_captured_at, $8),
                updated_at          = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(handle)
        .bind(&input.platform)
        .bind(&input.platform_url)
        .bind(input.verified_identity)
        .bind(input.consent_to_publish)
        .bind(&input.consent_source)
        .bind(input.consent_captured_at)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }
}
