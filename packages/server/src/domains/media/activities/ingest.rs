//! Root Signal media ingest activity — the synchronous pipeline the
//! ingest HTTP handler (Worktree 3) calls once per submitted
//! `source_image_url`.
//!
//! Contract: take a source URL + post_id, return a `MediaId` pointing
//! at a fully-hosted WebP in MinIO. Designed to run in-process under
//! the 10 s p95 budget the handoff spec assumes (fetch is capped at
//! 5 s; normalise + hash + S3 put together stay well inside the
//! remaining margin for the 5 MiB cap).

use anyhow::{Context, Result};
use chrono::Utc;
use sha2::{Digest, Sha256};
use tracing::{info, warn};
use uuid::Uuid;

use crate::domains::media::ingest::{fetch, normalise, ssrf, validate};
use crate::domains::media::models::{DesiredRef, Media, MediaReference};
use crate::domains::posts::models::PostMediaRecord;
use crate::kernel::ServerDeps;

/// The outcome of an ingest call. The caller (ingest handler) uses
/// `media_id` unconditionally; `reused_existing` is surfaced for
/// observability and to distinguish "this submission brought a new
/// image" from "this submission was a re-hash of a row we already had."
#[derive(Debug, Clone, Copy)]
pub struct IngestResult {
    pub media_id: Uuid,
    pub reused_existing: bool,
}

/// Errors the ingest activity can hard-fail with. Maps 1:1 onto the
/// structured error codes the ingest handler returns per handoff §11:
///
///   * `Ssrf(_)`        -> 422 `ssrf_blocked`
///   * `Fetch(_)`       -> 422 `source_image_unreachable`
///   * `Validate(_)`    -> 422 `source_image_invalid_format`
///   * `Normalise(_)`   -> 422 `source_image_decode_failed`
///   * `Storage(_)` / `Db(_)` / `MissingStorage` -> 500
#[derive(Debug, thiserror::Error)]
pub enum IngestError {
    #[error("ssrf guard: {0}")]
    Ssrf(#[from] ssrf::SsrfError),
    #[error("fetch: {0}")]
    Fetch(#[from] fetch::FetchError),
    #[error("validate: {0}")]
    Validate(#[from] validate::ValidateError),
    #[error("normalise: {0}")]
    Normalise(#[from] normalise::NormaliseError),
    #[error("storage service not configured")]
    MissingStorage,
    #[error("storage: {0}")]
    Storage(String),
    #[error("db: {0}")]
    Db(String),
}

/// Fetch, normalise, dedup, and store a single `source_image_url`.
/// Attaches the resulting `media_id` to `post_id` via a `post_media`
/// row (plus the polymorphic `media_references` entry that the Media
/// Library reads to compute usage counts).
///
/// Optional inputs (`caption`, `credit`, `alt_text`) are carried from
/// the Root Signal submission envelope; when `None`, the fields are
/// left NULL on the row and editors can fill them in later.
pub async fn ingest_source_image(
    source_url: &str,
    post_id: Uuid,
    caption: Option<&str>,
    credit: Option<&str>,
    alt_text: Option<&str>,
    deps: &ServerDeps,
) -> Result<IngestResult, IngestError> {
    let url = ssrf::validate_url(source_url)?;
    info!(source_url = %source_url, "media ingest: fetching");
    let fetched = fetch::fetch(url).await?;

    if fetched
        .upstream_content_type
        .as_deref()
        .is_some_and(|ct| !ct.starts_with("image/"))
    {
        // Worth logging: upstream's Content-Type disagrees with the
        // magic bytes. Not a hard fail — magic bytes win — but useful
        // observability signal for tracking misbehaving sources.
        warn!(
            ct = ?fetched.upstream_content_type,
            "upstream Content-Type did not claim image/*",
        );
    }

    ingest_from_body(
        source_url,
        post_id,
        fetched.bytes,
        caption,
        credit,
        alt_text,
        deps,
    )
    .await
}

/// The byte-in variant of [`ingest_source_image`]: skips SSRF + fetch
/// and picks up at magic-bytes validation. Used by the end-to-end
/// integration test (which bypasses the HTTPS fetch because
/// testcontainers-hosted fixture servers can't easily serve HTTPS)
/// and as the shared body of the public entry point above.
pub async fn ingest_from_body(
    source_url: &str,
    post_id: Uuid,
    body: Vec<u8>,
    caption: Option<&str>,
    credit: Option<&str>,
    alt_text: Option<&str>,
    deps: &ServerDeps,
) -> Result<IngestResult, IngestError> {
    let format = validate::detect_format(&body)?;
    let normalised = normalise::normalise_to_webp(&body, format)?;
    let content_hash = sha256_hex(&normalised.webp_bytes);

    if let Some(existing) = Media::find_by_content_hash(&content_hash, &deps.db_pool)
        .await
        .map_err(|e| IngestError::Db(e.to_string()))?
    {
        info!(
            media_id = %existing.id,
            content_hash = %content_hash,
            "media ingest: dedup hit, reusing existing media row",
        );
        link_post_media(post_id, existing.id, caption, credit, alt_text, &deps.db_pool)
            .await?;
        return Ok(IngestResult {
            media_id: existing.id,
            reused_existing: true,
        });
    }

    let storage = deps
        .storage
        .as_ref()
        .ok_or(IngestError::MissingStorage)?;
    let now = Utc::now();
    let storage_key = format!(
        "media/{}/{:02}/{}.webp",
        now.format("%Y"),
        now.format("%m"),
        Uuid::new_v4(),
    );
    let size_bytes = normalised.webp_bytes.len() as i64;
    storage
        .put_object(&storage_key, normalised.webp_bytes, "image/webp")
        .await
        .map_err(|e| IngestError::Storage(e.to_string()))?;
    let public_url = storage.public_url(&storage_key);

    let media = Media::create_ingested(
        &derive_filename(source_url),
        "image/webp",
        size_bytes,
        &storage_key,
        &public_url,
        Some(normalised.width as i32),
        Some(normalised.height as i32),
        source_url,
        &content_hash,
        &deps.db_pool,
    )
    .await
    .map_err(|e| IngestError::Db(e.to_string()))?;

    info!(
        media_id = %media.id,
        content_hash = %content_hash,
        width = normalised.width,
        height = normalised.height,
        "media ingest: stored new media row",
    );

    link_post_media(post_id, media.id, caption, credit, alt_text, &deps.db_pool)
        .await?;

    Ok(IngestResult {
        media_id: media.id,
        reused_existing: false,
    })
}

async fn link_post_media(
    post_id: Uuid,
    media_id: Uuid,
    caption: Option<&str>,
    credit: Option<&str>,
    alt_text: Option<&str>,
    pool: &sqlx::PgPool,
) -> Result<(), IngestError> {
    // post_media stores the per-post metadata (caption + credit);
    // alt_text lives on media itself and is updated here if the Root
    // Signal envelope carried it and the media row didn't already have
    // one. (Dedup case: second submission may carry a better alt_text
    // than the first — we don't overwrite existing non-NULL alt_text
    // since an editor may have improved it.)
    if let Some(alt) = alt_text {
        backfill_alt_text_if_empty(media_id, alt, pool)
            .await
            .map_err(|e| IngestError::Db(e.to_string()))?;
    }

    // Find the post_media row's public URL by re-reading the media row
    // — the upsert helper wants image_url alongside media_id so we stay
    // compatible with the existing read path (resolvers still read the
    // denormalised image_url).
    let media_url: (String,) =
        sqlx::query_as("SELECT url FROM media WHERE id = $1")
            .bind(media_id)
            .fetch_one(pool)
            .await
            .map_err(|e| IngestError::Db(e.to_string()))?;

    PostMediaRecord::upsert_primary(
        post_id,
        Some(&media_url.0),
        caption,
        credit,
        Some(media_id),
        pool,
    )
    .await
    .map_err(|e| IngestError::Db(e.to_string()))?;

    // upsert_primary already reconciled the `post_hero` media_reference,
    // but it only clears/sets based on the primary-media concept. Keep
    // the call explicit for callers that bypass post_media (e.g. a
    // future inline-body-image path) — idempotent.
    let desired = vec![DesiredRef { media_id, field_key: None }];
    MediaReference::reconcile("post_hero", post_id, &desired, pool)
        .await
        .map_err(|e| IngestError::Db(e.to_string()))?;

    Ok(())
}

async fn backfill_alt_text_if_empty(
    media_id: Uuid,
    alt_text: &str,
    pool: &sqlx::PgPool,
) -> Result<()> {
    sqlx::query(
        "UPDATE media SET alt_text = $2, updated_at = NOW()
         WHERE id = $1 AND (alt_text IS NULL OR alt_text = '')",
    )
    .bind(media_id)
    .bind(alt_text)
    .execute(pool)
    .await
    .context("backfill_alt_text_if_empty")?;
    Ok(())
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

/// Best-effort filename derivation for the media row — not
/// user-facing; the admin UI shows alt_text + source_url instead.
/// Falls back to "ingested.webp" for URLs like `…/` with no trailing
/// filename.
fn derive_filename(source_url: &str) -> String {
    let stem = source_url
        .rsplit('/')
        .next()
        .unwrap_or("ingested")
        .split(['?', '#'])
        .next()
        .unwrap_or("ingested");
    let stem = stem.trim_end_matches('/');
    if stem.is_empty() {
        "ingested.webp".to_string()
    } else {
        // Force .webp since we re-encoded. Strip any trailing
        // extension the source URL had.
        let base = stem
            .rsplit_once('.')
            .map(|(base, _ext)| base)
            .unwrap_or(stem);
        format!("{base}.webp")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filename_derivation() {
        assert_eq!(
            derive_filename("https://example.org/path/photo.jpg"),
            "photo.webp"
        );
        assert_eq!(
            derive_filename("https://example.org/path/photo.jpg?cache=1"),
            "photo.webp"
        );
        assert_eq!(
            derive_filename("https://example.org/path/photo"),
            "photo.webp"
        );
        assert_eq!(
            derive_filename("https://example.org/"),
            "ingested.webp"
        );
    }

    #[test]
    fn sha256_is_deterministic_and_distinct() {
        assert_eq!(sha256_hex(b"abc"), sha256_hex(b"abc"));
        assert_ne!(sha256_hex(b"abc"), sha256_hex(b"abd"));
    }
}
