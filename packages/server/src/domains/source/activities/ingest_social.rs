//! Ingest social media sources into extraction_pages for org-level extraction.
//!
//! Uses platform-specific Ingestors that output uniform RawPage objects,
//! which are converted to CachedPages and stored in extraction_pages.

use anyhow::{Context, Result};
use extraction::types::page::CachedPage;
use tracing::info;
use uuid::Uuid;

use crate::common::SourceId;
use crate::domains::source::ingestors::{FacebookIngestor, InstagramIngestor, XIngestor};
use crate::domains::source::models::{SocialSource, Source};
use crate::kernel::ServerDeps;

use extraction::traits::ingestor::{DiscoverConfig, Ingestor};

#[derive(Debug)]
pub struct IngestSocialResult {
    pub pages_stored: usize,
}

/// Ingest a social source: scrape via platform Ingestor, store to extraction_pages.
///
/// This is the new social pipeline entry point. Instead of extracting posts inline,
/// content is stored uniformly in extraction_pages for later org-level extraction.
pub async fn ingest_social_source(source_id: Uuid, deps: &ServerDeps) -> Result<IngestSocialResult> {
    let pool = &deps.db_pool;
    let source = Source::find_by_id(SourceId::from_uuid(source_id), pool)
        .await
        .context("Failed to load source")?;

    let social = SocialSource::find_by_source_id(SourceId::from_uuid(source_id), pool)
        .await
        .context("Failed to load social source")?;

    let site_url = source
        .url
        .clone()
        .unwrap_or_else(|| build_profile_url(&source.source_type, &social.handle));

    let ingestor = build_ingestor(&source.source_type, deps)?;

    let config = DiscoverConfig::new(&site_url)
        .with_limit(50)
        .with_max_depth(0)
        .with_option("handle", &social.handle);

    info!(
        source_id = %source_id,
        source_type = %source.source_type,
        site_url = %site_url,
        "Ingesting social source"
    );

    let pages = ingestor
        .discover(&config)
        .await
        .map_err(|e| anyhow::anyhow!("Ingestor discover failed: {}", e))?;

    // Store each page in extraction_pages
    let extraction = deps
        .extraction
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Extraction service not configured"))?;

    let mut stored = 0;
    for page in &pages {
        let cached = CachedPage::new(&page.url, &site_url, &page.content)
            .with_title(page.title.clone().unwrap_or_default());
        extraction.store_page(&cached).await?;
        stored += 1;
    }

    // Update last_scraped_at
    Source::update_last_scraped(SourceId::from_uuid(source_id), pool).await?;

    info!(
        source_id = %source_id,
        source_type = %source.source_type,
        pages_stored = stored,
        "Social source ingested"
    );

    Ok(IngestSocialResult { pages_stored: stored })
}

pub fn build_profile_url(source_type: &str, handle: &str) -> String {
    match source_type {
        "instagram" => format!("https://www.instagram.com/{}/", handle),
        "facebook" => format!("https://www.facebook.com/{}/", handle),
        "x" | "twitter" => format!("https://x.com/{}", handle),
        _ => format!("https://{}", handle),
    }
}

fn build_ingestor(source_type: &str, deps: &ServerDeps) -> Result<Box<dyn Ingestor>> {
    let apify = deps
        .apify_client
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Apify client not configured"))?;

    match source_type {
        "instagram" => Ok(Box::new(InstagramIngestor::new(apify.clone()))),
        "facebook" => Ok(Box::new(FacebookIngestor::new(apify.clone()))),
        "x" | "twitter" => Ok(Box::new(XIngestor::new(apify.clone()))),
        other => anyhow::bail!("Unsupported social source type: {}", other),
    }
}
