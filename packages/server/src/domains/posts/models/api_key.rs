//! Service-client API keys (Bearer tokens).
//!
//! Schema lives in migration 000237. Plaintext tokens are never stored —
//! `token_hash` holds SHA-256(plaintext) hex, lowercase. The `prefix` field is
//! the visible environment tag (`rsk_live_`, `rsk_test_`, `rsk_dev_`) kept
//! separately from the secret body so operators can correlate log lines
//! without logging the token.
//!
//! Token format per spec §14.1: `rsk_{env}_<32-char-url-safe-base64>`.

use anyhow::Result;
use base64::Engine;
use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::ApiKeyId;

/// The `api_keys` row. Plaintext token is never available here — see
/// `IssuedApiKey` for the one-shot issuance return value.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ApiKey {
    pub id: ApiKeyId,
    pub client_name: String,
    pub prefix: String,
    pub token_hash: String,
    pub scopes: Vec<String>,
    pub rotated_from_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub last_used_at: Option<DateTime<Utc>>,
}

/// Returned from `ApiKey::issue`. The `plaintext` field is the only place the
/// full token is ever available — it is not stored and cannot be recovered.
#[derive(Debug, Clone)]
pub struct IssuedApiKey {
    pub record: ApiKey,
    pub plaintext: String,
}

impl ApiKey {
    /// SHA-256(plaintext_token), lowercase hex. This is what we store and
    /// what the `ServiceClient` extractor compares against on each request.
    pub fn hash_token(token: &str) -> String {
        let mut h = Sha256::new();
        h.update(token.as_bytes());
        format!("{:x}", h.finalize())
    }

    /// Issue a fresh key. Generates a cryptographically-random 32-byte body,
    /// base64-url-encodes it (no padding), and prepends the environment prefix.
    /// Returns the plaintext exactly once.
    ///
    /// `env` is one of `"live"`, `"test"`, `"dev"` (spec §14.1).
    pub async fn issue(
        client_name: &str,
        env: &str,
        scopes: &[String],
        pool: &PgPool,
    ) -> Result<IssuedApiKey> {
        use rand::RngCore;

        let mut body = [0u8; 24]; // 24 random bytes → 32 base64-url chars (no pad).
        rand::rngs::OsRng.fill_bytes(&mut body);
        let body_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(body);

        let prefix = format!("rsk_{}_", env);
        let plaintext = format!("{}{}", prefix, body_b64);
        let token_hash = Self::hash_token(&plaintext);

        let record = sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO api_keys (client_name, prefix, token_hash, scopes)
            VALUES ($1, $2, $3, $4)
            RETURNING *
            "#,
        )
        .bind(client_name)
        .bind(&prefix)
        .bind(&token_hash)
        .bind(scopes)
        .fetch_one(pool)
        .await?;

        Ok(IssuedApiKey { record, plaintext })
    }

    /// Look up an active (non-revoked) key by token hash. Used by the
    /// `ServiceClient` auth extractor.
    pub async fn find_active_by_hash(token_hash: &str, pool: &PgPool) -> Result<Option<Self>> {
        sqlx::query_as::<_, Self>(
            r#"
            SELECT * FROM api_keys
            WHERE token_hash = $1 AND revoked_at IS NULL
            LIMIT 1
            "#,
        )
        .bind(token_hash)
        .fetch_optional(pool)
        .await
        .map_err(Into::into)
    }

    /// Update `last_used_at` to NOW(). Called by the extractor on successful
    /// auth. Failures here are not fatal — we don't block the request if the
    /// tracking update fails.
    pub async fn touch_last_used(id: ApiKeyId, pool: &PgPool) -> Result<()> {
        sqlx::query("UPDATE api_keys SET last_used_at = NOW() WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Revoke a key. Idempotent — revoking an already-revoked key is a no-op.
    pub async fn revoke(id: ApiKeyId, pool: &PgPool) -> Result<()> {
        sqlx::query("UPDATE api_keys SET revoked_at = NOW() WHERE id = $1 AND revoked_at IS NULL")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Rotate a key: issue a new one whose `rotated_from_id` points at the
    /// supplied original. Both keys stay active until the original is
    /// explicitly revoked — this preserves the overlap window the client
    /// needs to cut traffic over without race conditions.
    pub async fn rotate(
        from_id: ApiKeyId,
        env: &str,
        pool: &PgPool,
    ) -> Result<IssuedApiKey> {
        let existing = Self::find_by_id(from_id, pool).await?.ok_or_else(|| {
            anyhow::anyhow!("api_key {} not found", from_id.as_uuid())
        })?;
        if existing.revoked_at.is_some() {
            anyhow::bail!("cannot rotate revoked key {}", from_id.as_uuid());
        }

        use rand::RngCore;
        let mut body = [0u8; 24];
        rand::rngs::OsRng.fill_bytes(&mut body);
        let body_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(body);
        let prefix = format!("rsk_{}_", env);
        let plaintext = format!("{}{}", prefix, body_b64);
        let token_hash = Self::hash_token(&plaintext);

        let record = sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO api_keys (client_name, prefix, token_hash, scopes, rotated_from_id)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING *
            "#,
        )
        .bind(&existing.client_name)
        .bind(&prefix)
        .bind(&token_hash)
        .bind(&existing.scopes)
        .bind(from_id.as_uuid())
        .fetch_one(pool)
        .await?;

        Ok(IssuedApiKey { record, plaintext })
    }

    pub async fn find_by_id(id: ApiKeyId, pool: &PgPool) -> Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM api_keys WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await
            .map_err(Into::into)
    }

    /// List all keys — active first, then revoked. Used by `dev-cli apikey list`.
    pub async fn list_all(pool: &PgPool) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            r#"
            SELECT * FROM api_keys
            ORDER BY revoked_at NULLS FIRST, created_at DESC
            "#,
        )
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    /// Find the most-recently-created active key for a client_name. Used by
    /// `dev-cli apikey rotate <client_name>` and `revoke <client_name>`.
    pub async fn find_active_by_client(client_name: &str, pool: &PgPool) -> Result<Option<Self>> {
        sqlx::query_as::<_, Self>(
            r#"
            SELECT * FROM api_keys
            WHERE client_name = $1 AND revoked_at IS NULL
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .bind(client_name)
        .fetch_optional(pool)
        .await
        .map_err(Into::into)
    }
}
