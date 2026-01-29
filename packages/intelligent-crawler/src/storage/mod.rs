use crate::types::*;
use anyhow::Result;
use async_trait::async_trait;

// Old implementation (backward compatibility)
pub mod postgres;
pub use postgres::PostgresStorage;

// New Seesaw-compatible implementation
pub mod crawler_storage;
pub use crawler_storage::PostgresCrawlerStorage;

/// Storage trait for intelligent crawler data (OLD - will be deprecated)
#[async_trait]
pub trait Storage: Send + Sync {
    // Page snapshots
    async fn save_page_snapshot(&self, snapshot: &PageSnapshot) -> Result<()>;
    async fn get_page_snapshot(&self, id: PageSnapshotId) -> Result<Option<PageSnapshot>>;
    async fn find_page_snapshot_by_url_and_hash(
        &self,
        url: &str,
        content_hash: &ContentHash,
    ) -> Result<Option<PageSnapshot>>;
    async fn list_page_snapshots_by_url(&self, url: &str) -> Result<Vec<PageSnapshot>>;

    // Schemas
    async fn save_schema(&self, schema: &Schema) -> Result<()>;
    async fn get_schema(&self, id: SchemaId) -> Result<Option<Schema>>;
    async fn find_schema_by_name_version(&self, name: &str, version: u32) -> Result<Option<Schema>>;
    async fn list_schemas(&self) -> Result<Vec<Schema>>;

    // Detections
    async fn save_detection(&self, detection: &Detection) -> Result<()>;
    async fn get_detection(&self, id: DetectionId) -> Result<Option<Detection>>;
    async fn list_detections_by_snapshot(
        &self,
        snapshot_id: PageSnapshotId,
    ) -> Result<Vec<Detection>>;
    async fn list_detections_by_kind(&self, kind: &str) -> Result<Vec<Detection>>;

    // Extractions
    async fn save_extraction(&self, extraction: &Extraction, provenance: &[FieldProvenance]) -> Result<()>;
    async fn get_extraction(&self, id: ExtractionId) -> Result<Option<Extraction>>;
    async fn get_extraction_provenance(&self, id: ExtractionId) -> Result<Vec<FieldProvenance>>;
    async fn find_extraction_by_fingerprint(
        &self,
        fingerprint: &ContentHash,
        schema_id: SchemaId,
        schema_version: u32,
    ) -> Result<Option<Extraction>>;
    async fn list_extractions_by_snapshot(
        &self,
        snapshot_id: PageSnapshotId,
    ) -> Result<Vec<Extraction>>;
    async fn list_extractions_by_schema(&self, schema_id: SchemaId) -> Result<Vec<Extraction>>;

    // Relationships
    async fn save_relationship(&self, relationship: &Relationship) -> Result<()>;
    async fn get_relationship(&self, id: RelationshipId) -> Result<Option<Relationship>>;
    async fn list_relationships_from(&self, extraction_id: ExtractionId) -> Result<Vec<Relationship>>;
    async fn list_relationships_to(&self, extraction_id: ExtractionId) -> Result<Vec<Relationship>>;
    async fn list_relationships_by_kind(&self, kind: &str) -> Result<Vec<Relationship>>;
}
