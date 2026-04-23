//! Idempotency-key storage for the ingest endpoint (spec §12.3).
//!
//! A `(api_key_id, key)` pair is unique within a 24-hour window. The handler:
//!
//!   1. Canonicalises the request body (sorted JSON keys, no insignificant
//!      whitespace, `null` kept, `absent` kept absent) and SHA-256's it.
//!   2. Looks up `(api_key_id, key)` — if a row exists:
//!        - Matching `payload_hash` → return the stored `response_body` with
//!          `idempotency_key_seen_before = true`. No re-processing.
//!        - Differing `payload_hash` → 409 `idempotency_conflict`. Client bug.
//!   3. Missing → process normally, then persist the response for future
//!      retries via `ApiIdempotencyKey::store`.

use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ApiIdempotencyKey {
    pub key: Uuid,
    pub api_key_id: Uuid,
    pub payload_hash: String,
    pub response_status: i32,
    pub response_body: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

impl ApiIdempotencyKey {
    /// Look up an existing idempotency record for this (api_key, key) pair.
    pub async fn find(
        api_key_id: Uuid,
        key: Uuid,
        pool: &PgPool,
    ) -> Result<Option<Self>> {
        sqlx::query_as::<_, Self>(
            r#"
            SELECT * FROM api_idempotency_keys
            WHERE api_key_id = $1 AND key = $2
            "#,
        )
        .bind(api_key_id)
        .bind(key)
        .fetch_optional(pool)
        .await
        .map_err(Into::into)
    }

    /// Persist the successful response so retries under the same key return
    /// the same body. Called *after* a 201 is built.
    pub async fn store(
        key: Uuid,
        api_key_id: Uuid,
        payload_hash: &str,
        response_status: i32,
        response_body: &serde_json::Value,
        pool: &PgPool,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO api_idempotency_keys (
                key, api_key_id, payload_hash, response_status, response_body
            )
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (key) DO NOTHING
            "#,
        )
        .bind(key)
        .bind(api_key_id)
        .bind(payload_hash)
        .bind(response_status)
        .bind(response_body)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Periodic cleanup — drop rows older than 24 hours (spec §12.3).
    /// Returns number of rows removed.
    pub async fn prune_older_than_24h(pool: &PgPool) -> Result<u64> {
        let result = sqlx::query(
            "DELETE FROM api_idempotency_keys WHERE created_at < NOW() - INTERVAL '24 hours'",
        )
        .execute(pool)
        .await?;
        Ok(result.rows_affected())
    }
}

/// Canonicalise a JSON body into a byte string for SHA-256 hashing. The
/// canonical form: keys sorted recursively, no indentation, no trailing
/// whitespace, `null` kept, absent keys kept absent, numeric literals
/// preserved exactly as submitted (serde_json's `Value` round-trips numbers
/// by string unless they fit in i64/f64 — close enough for equivalence
/// checking).
pub fn canonicalize_body(raw: &[u8]) -> Result<Vec<u8>> {
    let value: serde_json::Value = serde_json::from_slice(raw)?;
    let canonical = sort_object_keys(value);
    Ok(serde_json::to_vec(&canonical)?)
}

fn sort_object_keys(value: serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => {
            let mut pairs: Vec<_> = map.into_iter().collect();
            pairs.sort_by(|a, b| a.0.cmp(&b.0));
            let sorted: serde_json::Map<String, serde_json::Value> = pairs
                .into_iter()
                .map(|(k, v)| (k, sort_object_keys(v)))
                .collect();
            serde_json::Value::Object(sorted)
        }
        serde_json::Value::Array(items) => {
            // Array order is significant — don't sort, just recurse.
            serde_json::Value::Array(items.into_iter().map(sort_object_keys).collect())
        }
        other => other,
    }
}

/// SHA-256 hex digest of the canonicalised body.
pub fn hash_canonical_body(raw: &[u8]) -> Result<String> {
    use sha2::{Digest, Sha256};
    let canonical = canonicalize_body(raw)?;
    let mut h = Sha256::new();
    h.update(&canonical);
    Ok(format!("{:x}", h.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_hash_is_order_insensitive() {
        let a = br#"{"b":1,"a":2}"#;
        let b = br#"{"a":2,"b":1}"#;
        assert_eq!(
            hash_canonical_body(a).unwrap(),
            hash_canonical_body(b).unwrap()
        );
    }

    #[test]
    fn canonical_hash_is_whitespace_insensitive() {
        let a = br#"{"a":1,"b":2}"#;
        let b = b"{ \"a\" : 1 , \"b\" : 2 }";
        assert_eq!(
            hash_canonical_body(a).unwrap(),
            hash_canonical_body(b).unwrap()
        );
    }

    #[test]
    fn canonical_hash_distinguishes_null_from_absent() {
        let with_null = br#"{"a":1,"b":null}"#;
        let without_b = br#"{"a":1}"#;
        assert_ne!(
            hash_canonical_body(with_null).unwrap(),
            hash_canonical_body(without_b).unwrap()
        );
    }

    #[test]
    fn canonical_hash_preserves_array_order() {
        let a = br#"{"items":[1,2,3]}"#;
        let b = br#"{"items":[3,2,1]}"#;
        assert_ne!(
            hash_canonical_body(a).unwrap(),
            hash_canonical_body(b).unwrap()
        );
    }
}
