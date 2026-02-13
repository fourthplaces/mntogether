//! Backfill activities for posts - admin batch operations

use anyhow::Result;
use tracing::{error, info};

use crate::domains::locations::models::Location;
use crate::domains::posts::models::post::Post;
use crate::domains::posts::models::post_location::PostLocation;
use crate::kernel::ServerDeps;

/// Result of backfilling post embeddings
#[derive(Debug)]
pub struct BackfillEmbeddingsResult {
    pub processed: i32,
    pub failed: i32,
    pub remaining: i32,
}

/// Backfill embeddings for posts that don't have them.
///
/// Finds active posts without embeddings and generates them in a loop.
pub async fn backfill_post_embeddings(
    limit: i32,
    deps: &ServerDeps,
) -> Result<BackfillEmbeddingsResult> {
    use super::post_operations;

    info!(limit = %limit, "Backfilling post embeddings");

    let posts = Post::find_without_embeddings(limit, &deps.db_pool).await?;

    let mut processed = 0;
    let mut failed = 0;

    for post in posts {
        match post_operations::generate_post_embedding(
            post.id,
            deps.embedding_service.as_ref(),
            &deps.db_pool,
        )
        .await
        {
            Ok(_) => processed += 1,
            Err(e) => {
                error!(post_id = %post.id, error = %e, "Failed to generate embedding");
                failed += 1;
            }
        }
    }

    // Count remaining posts without embeddings
    let remaining = if processed == 0 && failed == 0 {
        0
    } else {
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM posts WHERE embedding IS NULL AND deleted_at IS NULL AND status = 'active'",
        )
        .fetch_one(&deps.db_pool)
        .await
        .unwrap_or(0) as i32
    };

    info!(processed, failed, remaining, "Backfill completed");

    Ok(BackfillEmbeddingsResult {
        processed,
        failed,
        remaining,
    })
}

/// Result of backfilling post locations
#[derive(Debug)]
pub struct BackfillLocationsResult {
    pub processed: i32,
    pub failed: i32,
    pub remaining: i32,
}

/// Extract a 5-digit zip code from location text
fn extract_zip_from_text(text: &str) -> Option<String> {
    let re = regex::Regex::new(r"\b(\d{5})\b").ok()?;
    re.find(text).map(|m| m.as_str().to_string())
}

/// Extract a city name from location text (assumes "City, ST" or "City, State" pattern)
fn extract_city_from_text(text: &str) -> Option<String> {
    let re = regex::Regex::new(r"(?i)^([A-Za-z\s]+),\s*(?:MN|Minnesota)").ok()?;
    re.captures(text)
        .map(|c| c.get(1).unwrap().as_str().trim().to_string())
}

/// Backfill location records for posts that have location text but no post_locations.
///
/// Parses existing `location` text field to extract city/state/zip and creates
/// Location + PostLocation records.
pub async fn backfill_post_locations(
    batch_size: i64,
    deps: &ServerDeps,
) -> Result<BackfillLocationsResult> {
    info!(batch_size = %batch_size, "Backfilling post locations");

    // Find active posts with location text but no post_locations record
    let posts = sqlx::query_as::<_, Post>(
        r#"
        SELECT p.* FROM posts p
        WHERE p.status = 'active'
          AND p.deleted_at IS NULL
          AND p.revision_of_post_id IS NULL
          AND p.location IS NOT NULL
          AND p.location != ''
          AND NOT EXISTS (
              SELECT 1 FROM post_locations pl WHERE pl.post_id = p.id
          )
        ORDER BY p.created_at DESC
        LIMIT $1
        "#,
    )
    .bind(batch_size)
    .fetch_all(&deps.db_pool)
    .await?;

    let mut processed = 0;
    let mut failed = 0;

    for post in &posts {
        let location_text = post.location.as_deref().unwrap_or_default();

        let zip = extract_zip_from_text(location_text);
        let city = extract_city_from_text(location_text);

        if zip.is_none() && city.is_none() {
            failed += 1;
            continue;
        }

        match Location::find_or_create_from_extraction(
            city.as_deref(),
            Some("MN"),
            zip.as_deref(),
            None,
            &deps.db_pool,
        )
        .await
        {
            Ok(loc) => {
                if PostLocation::create(post.id, loc.id, true, None, &deps.db_pool)
                    .await
                    .is_ok()
                {
                    processed += 1;
                } else {
                    failed += 1;
                }
            }
            Err(_) => {
                failed += 1;
            }
        }
    }

    let remaining = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*) FROM posts p
        WHERE p.status = 'active' AND p.deleted_at IS NULL AND p.revision_of_post_id IS NULL
          AND p.location IS NOT NULL AND p.location != ''
          AND NOT EXISTS (SELECT 1 FROM post_locations pl WHERE pl.post_id = p.id)
        "#,
    )
    .fetch_one(&deps.db_pool)
    .await
    .unwrap_or(0) as i32;

    info!(
        processed,
        failed, remaining, "Post location backfill completed"
    );

    Ok(BackfillLocationsResult {
        processed,
        failed,
        remaining,
    })
}
