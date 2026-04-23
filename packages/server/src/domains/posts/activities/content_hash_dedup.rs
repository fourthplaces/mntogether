//! Content-hash dedup for §1.5.
//!
//! Hash function:
//!
//!   SHA-256(
//!       normalised_title
//!       || "\n" || normalised_source_url
//!       || "\n" || day_bucket(published_at)
//!       || "\n" || sorted_service_area_slugs.join(",")
//!   )
//!
//! On match, the orchestrator refreshes the matched post's `published_at` to
//! NOW() (extends 7-day eligibility) and returns the existing `post_id`
//! without inserting.
//!
//! Normalisation:
//!   * Title: lowercase, trim, collapse whitespace runs to single space.
//!   * URL: same treatment as dedup-time domain normalisation (lowercase,
//!     strip scheme, strip `www.`, strip trailing slash and querystring).
//!     We don't canonicalise the path — two different article URLs on the
//!     same domain should not collide.
//!   * Day bucket: YYYY-MM-DD of `published_at` in UTC. Missing `published_at`
//!     degrades gracefully to the string "no_published_at".
//!   * Service areas: slugs are sorted lexicographically before joining so
//!     `["hennepin-county", "ramsey-county"]` and `["ramsey-county", "hennepin-county"]`
//!     produce the same hash.

use anyhow::Result;
use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

pub fn compute_content_hash(
    title: &str,
    source_url: Option<&str>,
    published_at: Option<DateTime<Utc>>,
    service_area_slugs: &[String],
) -> String {
    let title = normalise_text(title);
    let url = source_url
        .map(normalise_url_for_hash)
        .unwrap_or_else(|| "no_source_url".to_string());
    let day = published_at
        .map(|dt| dt.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| "no_published_at".to_string());

    let mut slugs = service_area_slugs.to_vec();
    slugs.sort();
    let slugs_joined = slugs.join(",");

    let mut h = Sha256::new();
    h.update(title.as_bytes());
    h.update(b"\n");
    h.update(url.as_bytes());
    h.update(b"\n");
    h.update(day.as_bytes());
    h.update(b"\n");
    h.update(slugs_joined.as_bytes());

    format!("{:x}", h.finalize())
}

fn normalise_text(s: &str) -> String {
    let lowered = s.to_lowercase();
    lowered.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn normalise_url_for_hash(url: &str) -> String {
    let trimmed = url.trim();
    // Strip scheme.
    let without_scheme = match trimmed.find("://") {
        Some(idx) => &trimmed[idx + 3..],
        None => trimmed,
    };
    // Drop fragment + querystring.
    let no_frag = without_scheme
        .split(|c: char| c == '#' || c == '?')
        .next()
        .unwrap_or(without_scheme);
    let lower = no_frag.to_lowercase();
    // Strip www.
    let stripped = lower.strip_prefix("www.").unwrap_or(&lower).to_string();
    // Collapse trailing slashes.
    stripped.trim_end_matches('/').to_string()
}

/// Find an existing non-deleted post with the same content hash.
pub async fn find_existing_by_hash(
    content_hash: &str,
    pool: &PgPool,
) -> Result<Option<Uuid>> {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        SELECT id FROM posts
        WHERE content_hash = $1
          AND deleted_at IS NULL
          AND duplicate_of_id IS NULL
        ORDER BY created_at DESC
        LIMIT 1
        "#,
    )
    .bind(content_hash)
    .fetch_optional(pool)
    .await
    .map_err(Into::into)
}

/// On dedup hit: refresh `published_at` to NOW() (extends 7-day eligibility)
/// and update the hash (harmless — it already matches). Returns the row for
/// the caller to echo back in the 201.
pub async fn refresh_existing(post_id: Uuid, pool: &PgPool) -> Result<()> {
    sqlx::query(
        r#"
        UPDATE posts
        SET published_at = NOW(),
            updated_at   = NOW()
        WHERE id = $1
        "#,
    )
    .bind(post_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Write the hash into the post row. Called on insert path.
pub async fn set_content_hash(post_id: Uuid, hash: &str, pool: &PgPool) -> Result<()> {
    sqlx::query("UPDATE posts SET content_hash = $2 WHERE id = $1")
        .bind(post_id)
        .bind(hash)
        .execute(pool)
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_input_same_hash() {
        let a = compute_content_hash(
            "Road Closure",
            Some("https://example.com/road"),
            None,
            &["hennepin-county".into()],
        );
        let b = compute_content_hash(
            "Road Closure",
            Some("https://example.com/road"),
            None,
            &["hennepin-county".into()],
        );
        assert_eq!(a, b);
    }

    #[test]
    fn title_normalisation_is_case_insensitive() {
        let a = compute_content_hash(
            "Road Closure",
            Some("https://example.com/road"),
            None,
            &["hennepin-county".into()],
        );
        let b = compute_content_hash(
            "ROAD closure",
            Some("https://example.com/road"),
            None,
            &["hennepin-county".into()],
        );
        assert_eq!(a, b);
    }

    #[test]
    fn service_area_order_does_not_matter() {
        let a = compute_content_hash(
            "x",
            None,
            None,
            &["hennepin-county".into(), "ramsey-county".into()],
        );
        let b = compute_content_hash(
            "x",
            None,
            None,
            &["ramsey-county".into(), "hennepin-county".into()],
        );
        assert_eq!(a, b);
    }

    #[test]
    fn url_scheme_and_www_normalised() {
        let a = compute_content_hash("x", Some("https://www.example.com/road/"), None, &[]);
        let b = compute_content_hash("x", Some("http://example.com/road"), None, &[]);
        assert_eq!(a, b);
    }

    #[test]
    fn distinct_urls_distinct_hash() {
        let a = compute_content_hash("x", Some("https://example.com/road"), None, &[]);
        let b = compute_content_hash("x", Some("https://example.com/bridge"), None, &[]);
        assert_ne!(a, b);
    }
}
