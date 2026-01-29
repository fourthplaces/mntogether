use crate::storage::Storage;
use crate::types::{PageSnapshot, PageSnapshotId};
use anyhow::{Context, Result};

/// Trait for web crawling clients (to allow mocking)
#[async_trait::async_trait]
pub trait WebCrawler: Send + Sync {
    async fn crawl(&self, url: &str, page_limit: Option<usize>) -> Result<Vec<CrawlPage>>;
}

/// A single page from a crawl
#[derive(Debug, Clone)]
pub struct CrawlPage {
    pub url: String,
    pub html: String,
    pub markdown: Option<String>,
    pub title: Option<String>,
}

/// Crawl a site and store page snapshots
pub async fn crawl_site(
    url: &str,
    crawler: &impl WebCrawler,
    storage: &impl Storage,
    page_limit: Option<usize>,
) -> Result<Vec<PageSnapshotId>> {
    tracing::info!(url = %url, page_limit = ?page_limit, "Starting crawl");

    let pages = crawler
        .crawl(url, page_limit)
        .await
        .context("Failed to crawl site")?;

    let mut snapshot_ids = Vec::new();

    for page in pages {
        // Check if we already have this exact content
        let snapshot = PageSnapshot::new(
            page.url.clone(),
            page.html,
            page.markdown,
            "firecrawl".to_string(),
        );

        // Check for deduplication
        if let Some(existing) = storage
            .find_page_snapshot_by_url_and_hash(&snapshot.url, &snapshot.content_hash)
            .await?
        {
            tracing::info!(
                url = %snapshot.url,
                snapshot_id = %existing.id.0,
                content_hash = %snapshot.content_hash.to_hex(),
                "Page unchanged since last crawl - using cached snapshot"
            );
            snapshot_ids.push(existing.id);
            continue;
        }

        // Store new snapshot
        let snapshot_id = snapshot.id;
        storage
            .save_page_snapshot(&snapshot)
            .await
            .context("Failed to save page snapshot")?;

        tracing::info!(
            url = %snapshot.url,
            snapshot_id = %snapshot_id.0,
            content_hash = %snapshot.content_hash.to_hex(),
            content_length = snapshot.markdown.as_ref().map(|m| m.len()).unwrap_or(0),
            "Stored new page snapshot"
        );

        snapshot_ids.push(snapshot_id);
    }

    tracing::info!(
        url = %url,
        snapshots_stored = snapshot_ids.len(),
        "Crawl completed"
    );

    Ok(snapshot_ids)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;

    struct MockStorage {
        snapshots: std::sync::Mutex<HashMap<PageSnapshotId, PageSnapshot>>,
    }

    impl MockStorage {
        fn new() -> Self {
            Self {
                snapshots: std::sync::Mutex::new(HashMap::new()),
            }
        }
    }

    #[async_trait::async_trait]
    impl Storage for MockStorage {
        async fn save_page_snapshot(&self, snapshot: &PageSnapshot) -> Result<()> {
            self.snapshots.lock().unwrap().insert(snapshot.id, snapshot.clone());
            Ok(())
        }

        async fn get_page_snapshot(&self, id: PageSnapshotId) -> Result<Option<PageSnapshot>> {
            Ok(self.snapshots.lock().unwrap().get(&id).cloned())
        }

        async fn find_page_snapshot_by_url_and_hash(
            &self,
            url: &str,
            content_hash: &ContentHash,
        ) -> Result<Option<PageSnapshot>> {
            Ok(self
                .snapshots
                .lock()
                .unwrap()
                .values()
                .find(|s| s.url == url && s.content_hash == *content_hash)
                .cloned())
        }

        async fn list_page_snapshots_by_url(&self, _url: &str) -> Result<Vec<PageSnapshot>> {
            unimplemented!()
        }

        async fn save_schema(&self, _schema: &Schema) -> Result<()> {
            unimplemented!()
        }

        async fn get_schema(&self, _id: SchemaId) -> Result<Option<Schema>> {
            unimplemented!()
        }

        async fn find_schema_by_name_version(&self, _name: &str, _version: u32) -> Result<Option<Schema>> {
            unimplemented!()
        }

        async fn list_schemas(&self) -> Result<Vec<Schema>> {
            unimplemented!()
        }

        async fn save_detection(&self, _detection: &Detection) -> Result<()> {
            unimplemented!()
        }

        async fn get_detection(&self, _id: DetectionId) -> Result<Option<Detection>> {
            unimplemented!()
        }

        async fn list_detections_by_snapshot(
            &self,
            _snapshot_id: PageSnapshotId,
        ) -> Result<Vec<Detection>> {
            unimplemented!()
        }

        async fn list_detections_by_kind(&self, _kind: &str) -> Result<Vec<Detection>> {
            unimplemented!()
        }

        async fn save_extraction(&self, _extraction: &Extraction, _provenance: &[FieldProvenance]) -> Result<()> {
            unimplemented!()
        }

        async fn get_extraction(&self, _id: ExtractionId) -> Result<Option<Extraction>> {
            unimplemented!()
        }

        async fn get_extraction_provenance(&self, _id: ExtractionId) -> Result<Vec<FieldProvenance>> {
            unimplemented!()
        }

        async fn find_extraction_by_fingerprint(
            &self,
            _fingerprint: &ContentHash,
            _schema_id: SchemaId,
            _schema_version: u32,
        ) -> Result<Option<Extraction>> {
            unimplemented!()
        }

        async fn list_extractions_by_snapshot(
            &self,
            _snapshot_id: PageSnapshotId,
        ) -> Result<Vec<Extraction>> {
            unimplemented!()
        }

        async fn list_extractions_by_schema(&self, _schema_id: SchemaId) -> Result<Vec<Extraction>> {
            unimplemented!()
        }

        async fn save_relationship(&self, _relationship: &Relationship) -> Result<()> {
            unimplemented!()
        }

        async fn get_relationship(&self, _id: RelationshipId) -> Result<Option<Relationship>> {
            unimplemented!()
        }

        async fn list_relationships_from(&self, _extraction_id: ExtractionId) -> Result<Vec<Relationship>> {
            unimplemented!()
        }

        async fn list_relationships_to(&self, _extraction_id: ExtractionId) -> Result<Vec<Relationship>> {
            unimplemented!()
        }

        async fn list_relationships_by_kind(&self, _kind: &str) -> Result<Vec<Relationship>> {
            unimplemented!()
        }
    }

    struct MockCrawler {
        pages: Vec<CrawlPage>,
    }

    #[async_trait::async_trait]
    impl WebCrawler for MockCrawler {
        async fn crawl(&self, _url: &str, _page_limit: Option<usize>) -> Result<Vec<CrawlPage>> {
            Ok(self.pages.clone())
        }
    }

    #[tokio::test]
    async fn test_crawl_site_deduplication() {
        let storage = MockStorage::new();
        let crawler = MockCrawler {
            pages: vec![
                CrawlPage {
                    url: "https://example.com".to_string(),
                    html: "<html><body>Test</body></html>".to_string(),
                    markdown: Some("Test".to_string()),
                    title: Some("Test Page".to_string()),
                },
            ],
        };

        // First crawl
        let ids1 = crawl_site("https://example.com", &crawler, &storage, None)
            .await
            .unwrap();
        assert_eq!(ids1.len(), 1);

        // Second crawl with same content - should deduplicate
        let ids2 = crawl_site("https://example.com", &crawler, &storage, None)
            .await
            .unwrap();
        assert_eq!(ids2.len(), 1);
        assert_eq!(ids1[0], ids2[0]);
    }
}
