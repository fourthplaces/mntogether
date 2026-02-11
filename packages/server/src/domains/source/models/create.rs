use anyhow::Result;
use sqlx::PgPool;

use crate::common::{MemberId, OrganizationId, SourceId};

use super::social_source::SocialSource;
use super::source::Source;
use super::website_source::WebsiteSource;

/// Create a website source (inserts into both sources + website_sources)
/// Returns the existing source if the domain already exists.
pub async fn create_website_source(
    url_or_domain: &str,
    organization_id: Option<OrganizationId>,
    submitted_by: Option<MemberId>,
    submitter_type: &str,
    submission_context: Option<&str>,
    max_crawl_depth: i32,
    pool: &PgPool,
) -> Result<(Source, WebsiteSource)> {
    let domain = WebsiteSource::normalize_domain(url_or_domain)?;
    let url = format!("https://{}", domain);

    // Use a transaction to insert both rows atomically
    let mut tx = pool.begin().await?;

    let source = sqlx::query_as::<_, Source>(
        r#"
        INSERT INTO sources (source_type, url, organization_id, status, submitted_by, submitter_type, submission_context)
        VALUES ('website', $1, $2, 'pending_review', $3, $4, $5)
        RETURNING *
        "#,
    )
    .bind(&url)
    .bind(organization_id)
    .bind(submitted_by)
    .bind(submitter_type)
    .bind(submission_context)
    .fetch_one(&mut *tx)
    .await?;

    let website_source = sqlx::query_as::<_, WebsiteSource>(
        r#"
        INSERT INTO website_sources (source_id, domain, max_crawl_depth)
        VALUES ($1, $2, $3)
        RETURNING *
        "#,
    )
    .bind(source.id)
    .bind(&domain)
    .bind(max_crawl_depth)
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok((source, website_source))
}

/// Find or create a website source by domain (handles concurrent requests)
pub async fn find_or_create_website_source(
    url_or_domain: &str,
    submitted_by: Option<MemberId>,
    submitter_type: &str,
    submission_context: Option<&str>,
    max_crawl_depth: i32,
    pool: &PgPool,
) -> Result<(Source, WebsiteSource)> {
    let domain = WebsiteSource::normalize_domain(url_or_domain)?;

    // Check if already exists
    if let Some(ws) = WebsiteSource::find_by_domain(&domain, pool).await? {
        let source = Source::find_by_id(ws.source_id, pool).await?;
        return Ok((source, ws));
    }

    // Create new
    create_website_source(
        url_or_domain,
        None,
        submitted_by,
        submitter_type,
        submission_context,
        max_crawl_depth,
        pool,
    )
    .await
}

/// Create a social source (inserts into both sources + social_sources)
/// Social sources default to 'pending_review' status.
pub async fn create_social_source(
    platform: &str,
    handle: &str,
    url: Option<&str>,
    organization_id: Option<OrganizationId>,
    pool: &PgPool,
) -> Result<(Source, SocialSource)> {
    let resolved_url = url
        .map(|u| u.to_string())
        .unwrap_or_else(|| {
            let clean_handle = handle.trim_start_matches('@');
            match platform {
                "instagram" => format!("https://instagram.com/{}", clean_handle),
                "facebook" => format!("https://facebook.com/{}", clean_handle),
                "tiktok" => format!("https://tiktok.com/@{}", clean_handle),
                _ => format!("https://{}.com/{}", platform, clean_handle),
            }
        });

    let mut tx = pool.begin().await?;

    let source = sqlx::query_as::<_, Source>(
        r#"
        INSERT INTO sources (source_type, url, organization_id, status)
        VALUES ($1, $2, $3, 'pending_review')
        RETURNING *
        "#,
    )
    .bind(platform)
    .bind(&resolved_url)
    .bind(organization_id)
    .fetch_one(&mut *tx)
    .await?;

    let social_source = sqlx::query_as::<_, SocialSource>(
        r#"
        INSERT INTO social_sources (source_id, source_type, handle)
        VALUES ($1, $2, $3)
        RETURNING *
        "#,
    )
    .bind(source.id)
    .bind(platform)
    .bind(handle)
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok((source, social_source))
}

/// Find or create a social source by platform + handle
pub async fn find_or_create_social_source(
    platform: &str,
    handle: &str,
    url: Option<&str>,
    organization_id: Option<OrganizationId>,
    pool: &PgPool,
) -> Result<(Source, SocialSource)> {
    // Check if already exists
    if let Some(ss) = SocialSource::find_by_handle(platform, handle, pool).await? {
        let source = Source::find_by_id(ss.source_id, pool).await?;
        return Ok((source, ss));
    }

    create_social_source(platform, handle, url, organization_id, pool).await
}

/// Helper to get the display identifier for a source
pub async fn get_source_identifier(source_id: SourceId, pool: &PgPool) -> Result<String> {
    // Try website first
    if let Some(ws) = WebsiteSource::find_by_source_id_optional(source_id, pool).await? {
        return Ok(ws.domain);
    }
    // Try social
    if let Some(ss) = SocialSource::find_by_source_id_optional(source_id, pool).await? {
        return Ok(ss.handle);
    }
    Err(anyhow::anyhow!("No website_source or social_source found for source {}", source_id))
}
