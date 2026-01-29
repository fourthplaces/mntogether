use crate::storage::Storage;
use crate::types::*;
use anyhow::{Context, Result};
use async_trait::async_trait;
use sqlx::{PgPool, Row};

pub struct PostgresStorage {
    pool: PgPool,
}

impl PostgresStorage {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl Storage for PostgresStorage {
    async fn save_page_snapshot(&self, snapshot: &PageSnapshot) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO page_snapshots (
                id, url, content_hash, html, markdown, fetched_via, metadata, crawled_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT (url, content_hash) DO NOTHING
            "#,
        )
        .bind(snapshot.id.0)
        .bind(&snapshot.url)
        .bind(&snapshot.content_hash.0)
        .bind(&snapshot.html)
        .bind(&snapshot.markdown)
        .bind(&snapshot.fetched_via)
        .bind(serde_json::to_value(&snapshot.metadata)?)
        .bind(snapshot.crawled_at)
        .execute(&self.pool)
        .await
        .context("Failed to save page snapshot")?;
        Ok(())
    }

    async fn get_page_snapshot(&self, id: PageSnapshotId) -> Result<Option<PageSnapshot>> {
        let row = sqlx::query(
            r#"
            SELECT id, url, content_hash, html, markdown, fetched_via, metadata, crawled_at
            FROM page_snapshots
            WHERE id = $1
            "#,
        )
        .bind(id.0)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to get page snapshot")?;

        Ok(row.map(|r| PageSnapshot {
            id: PageSnapshotId(r.get("id")),
            url: r.get("url"),
            content_hash: ContentHash(r.get("content_hash")),
            html: r.get("html"),
            markdown: r.get("markdown"),
            fetched_via: r.get("fetched_via"),
            metadata: serde_json::from_value(r.get("metadata")).unwrap_or_default(),
            crawled_at: r.get("crawled_at"),
        }))
    }

    async fn find_page_snapshot_by_url_and_hash(
        &self,
        url: &str,
        content_hash: &ContentHash,
    ) -> Result<Option<PageSnapshot>> {
        let row = sqlx::query(
            r#"
            SELECT id, url, content_hash, html, markdown, fetched_via, metadata, crawled_at
            FROM page_snapshots
            WHERE url = $1 AND content_hash = $2
            "#,
        )
        .bind(url)
        .bind(&content_hash.0)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to find page snapshot by url and hash")?;

        Ok(row.map(|r| PageSnapshot {
            id: PageSnapshotId(r.get("id")),
            url: r.get("url"),
            content_hash: ContentHash(r.get("content_hash")),
            html: r.get("html"),
            markdown: r.get("markdown"),
            fetched_via: r.get("fetched_via"),
            metadata: serde_json::from_value(r.get("metadata")).unwrap_or_default(),
            crawled_at: r.get("crawled_at"),
        }))
    }

    async fn list_page_snapshots_by_url(&self, url: &str) -> Result<Vec<PageSnapshot>> {
        let rows = sqlx::query(
            r#"
            SELECT id, url, content_hash, html, markdown, fetched_via, metadata, crawled_at
            FROM page_snapshots
            WHERE url = $1
            ORDER BY crawled_at DESC
            "#,
        )
        .bind(url)
        .fetch_all(&self.pool)
        .await
        .context("Failed to list page snapshots by url")?;

        Ok(rows
            .into_iter()
            .map(|r| PageSnapshot {
                id: PageSnapshotId(r.get("id")),
                url: r.get("url"),
                content_hash: ContentHash(r.get("content_hash")),
                html: r.get("html"),
                markdown: r.get("markdown"),
                fetched_via: r.get("fetched_via"),
                metadata: serde_json::from_value(r.get("metadata")).unwrap_or_default(),
                crawled_at: r.get("crawled_at"),
            })
            .collect())
    }

    async fn save_schema(&self, schema: &Schema) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO schemas (id, name, version, json_schema, created_at)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (name, version) DO NOTHING
            "#,
        )
        .bind(schema.id.0)
        .bind(&schema.name)
        .bind(schema.version as i32)
        .bind(&schema.json_schema)
        .bind(schema.created_at)
        .execute(&self.pool)
        .await
        .context("Failed to save schema")?;
        Ok(())
    }

    async fn get_schema(&self, id: SchemaId) -> Result<Option<Schema>> {
        let row = sqlx::query(
            r#"
            SELECT id, name, version, json_schema, created_at
            FROM schemas
            WHERE id = $1
            "#,
        )
        .bind(id.0)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to get schema")?;

        Ok(row.map(|r| Schema {
            id: SchemaId(r.get("id")),
            name: r.get("name"),
            version: r.get::<i32, _>("version") as u32,
            json_schema: r.get("json_schema"),
            created_at: r.get("created_at"),
        }))
    }

    async fn find_schema_by_name_version(&self, name: &str, version: u32) -> Result<Option<Schema>> {
        let row = sqlx::query(
            r#"
            SELECT id, name, version, json_schema, created_at
            FROM schemas
            WHERE name = $1 AND version = $2
            "#,
        )
        .bind(name)
        .bind(version as i32)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to find schema by name and version")?;

        Ok(row.map(|r| Schema {
            id: SchemaId(r.get("id")),
            name: r.get("name"),
            version: r.get::<i32, _>("version") as u32,
            json_schema: r.get("json_schema"),
            created_at: r.get("created_at"),
        }))
    }

    async fn list_schemas(&self) -> Result<Vec<Schema>> {
        let rows = sqlx::query(
            r#"
            SELECT id, name, version, json_schema, created_at
            FROM schemas
            ORDER BY name, version DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .context("Failed to list schemas")?;

        Ok(rows
            .into_iter()
            .map(|r| Schema {
                id: SchemaId(r.get("id")),
                name: r.get("name"),
                version: r.get::<i32, _>("version") as u32,
                json_schema: r.get("json_schema"),
                created_at: r.get("created_at"),
            })
            .collect())
    }

    async fn save_detection(&self, detection: &Detection) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO detections (
                id, page_snapshot_id, kind,
                confidence_overall, confidence_heuristic, confidence_ai,
                origin, evidence, detected_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            "#,
        )
        .bind(detection.id.0)
        .bind(detection.page_snapshot_id.0)
        .bind(&detection.kind)
        .bind(detection.confidence.overall)
        .bind(detection.confidence.heuristic)
        .bind(detection.confidence.ai)
        .bind(serde_json::to_value(&detection.origin)?)
        .bind(serde_json::to_value(&detection.evidence)?)
        .bind(detection.detected_at)
        .execute(&self.pool)
        .await
        .context("Failed to save detection")?;
        Ok(())
    }

    async fn get_detection(&self, id: DetectionId) -> Result<Option<Detection>> {
        let row = sqlx::query(
            r#"
            SELECT id, page_snapshot_id, kind,
                   confidence_overall, confidence_heuristic, confidence_ai,
                   origin, evidence, detected_at
            FROM detections
            WHERE id = $1
            "#,
        )
        .bind(id.0)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to get detection")?;

        Ok(row.map(|r| Detection {
            id: DetectionId(r.get("id")),
            page_snapshot_id: PageSnapshotId(r.get("page_snapshot_id")),
            kind: r.get("kind"),
            confidence: ConfidenceScores {
                overall: r.get("confidence_overall"),
                heuristic: r.get("confidence_heuristic"),
                ai: r.get("confidence_ai"),
            },
            origin: serde_json::from_value(r.get("origin")).unwrap(),
            evidence: serde_json::from_value(r.get("evidence")).unwrap_or_default(),
            detected_at: r.get("detected_at"),
        }))
    }

    async fn list_detections_by_snapshot(
        &self,
        snapshot_id: PageSnapshotId,
    ) -> Result<Vec<Detection>> {
        let rows = sqlx::query(
            r#"
            SELECT id, page_snapshot_id, kind,
                   confidence_overall, confidence_heuristic, confidence_ai,
                   origin, evidence, detected_at
            FROM detections
            WHERE page_snapshot_id = $1
            ORDER BY confidence_overall DESC
            "#,
        )
        .bind(snapshot_id.0)
        .fetch_all(&self.pool)
        .await
        .context("Failed to list detections by snapshot")?;

        Ok(rows
            .into_iter()
            .map(|r| Detection {
                id: DetectionId(r.get("id")),
                page_snapshot_id: PageSnapshotId(r.get("page_snapshot_id")),
                kind: r.get("kind"),
                confidence: ConfidenceScores {
                    overall: r.get("confidence_overall"),
                    heuristic: r.get("confidence_heuristic"),
                    ai: r.get("confidence_ai"),
                },
                origin: serde_json::from_value(r.get("origin")).unwrap(),
                evidence: serde_json::from_value(r.get("evidence")).unwrap_or_default(),
                detected_at: r.get("detected_at"),
            })
            .collect())
    }

    async fn list_detections_by_kind(&self, kind: &str) -> Result<Vec<Detection>> {
        let rows = sqlx::query(
            r#"
            SELECT id, page_snapshot_id, kind,
                   confidence_overall, confidence_heuristic, confidence_ai,
                   origin, evidence, detected_at
            FROM detections
            WHERE kind = $1
            ORDER BY detected_at DESC
            "#,
        )
        .bind(kind)
        .fetch_all(&self.pool)
        .await
        .context("Failed to list detections by kind")?;

        Ok(rows
            .into_iter()
            .map(|r| Detection {
                id: DetectionId(r.get("id")),
                page_snapshot_id: PageSnapshotId(r.get("page_snapshot_id")),
                kind: r.get("kind"),
                confidence: ConfidenceScores {
                    overall: r.get("confidence_overall"),
                    heuristic: r.get("confidence_heuristic"),
                    ai: r.get("confidence_ai"),
                },
                origin: serde_json::from_value(r.get("origin")).unwrap(),
                evidence: serde_json::from_value(r.get("evidence")).unwrap_or_default(),
                detected_at: r.get("detected_at"),
            })
            .collect())
    }

    async fn save_extraction(&self, extraction: &Extraction, provenance: &[FieldProvenance]) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        sqlx::query(
            r#"
            INSERT INTO extractions (
                id, fingerprint, page_snapshot_id, schema_id, schema_version,
                data, confidence_overall, confidence_heuristic, confidence_ai,
                origin, extracted_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            ON CONFLICT (fingerprint, schema_id, schema_version) DO NOTHING
            "#,
        )
        .bind(extraction.id.0)
        .bind(&extraction.fingerprint.0)
        .bind(extraction.page_snapshot_id.0)
        .bind(extraction.schema_id.0)
        .bind(extraction.schema_version as i32)
        .bind(&extraction.data)
        .bind(extraction.confidence.overall)
        .bind(extraction.confidence.heuristic)
        .bind(extraction.confidence.ai)
        .bind(serde_json::to_value(&extraction.origin)?)
        .bind(extraction.extracted_at)
        .execute(&mut *tx)
        .await
        .context("Failed to save extraction")?;

        for prov in provenance {
            sqlx::query(
                r#"
                INSERT INTO field_provenance (extraction_id, field_path, source_location, extraction_method)
                VALUES ($1, $2, $3, $4)
                "#,
            )
            .bind(extraction.id.0)
            .bind(&prov.field_path)
            .bind(&prov.source_location)
            .bind(&prov.extraction_method)
            .execute(&mut *tx)
            .await
            .context("Failed to save field provenance")?;
        }

        tx.commit().await?;
        Ok(())
    }

    async fn get_extraction(&self, id: ExtractionId) -> Result<Option<Extraction>> {
        let row = sqlx::query(
            r#"
            SELECT id, fingerprint, page_snapshot_id, schema_id, schema_version,
                   data, confidence_overall, confidence_heuristic, confidence_ai,
                   origin, extracted_at
            FROM extractions
            WHERE id = $1
            "#,
        )
        .bind(id.0)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to get extraction")?;

        Ok(row.map(|r| {
            let provenance = Vec::new(); // Will be fetched separately if needed
            Extraction {
                id: ExtractionId(r.get("id")),
                fingerprint: ContentHash(r.get("fingerprint")),
                page_snapshot_id: PageSnapshotId(r.get("page_snapshot_id")),
                schema_id: SchemaId(r.get("schema_id")),
                schema_version: r.get::<i32, _>("schema_version") as u32,
                data: r.get("data"),
                confidence: ConfidenceScores {
                    overall: r.get("confidence_overall"),
                    heuristic: r.get("confidence_heuristic"),
                    ai: r.get("confidence_ai"),
                },
                origin: serde_json::from_value(r.get("origin")).unwrap(),
                field_provenance: provenance,
                extracted_at: r.get("extracted_at"),
            }
        }))
    }

    async fn get_extraction_provenance(&self, id: ExtractionId) -> Result<Vec<FieldProvenance>> {
        let rows = sqlx::query(
            r#"
            SELECT field_path, source_location, extraction_method
            FROM field_provenance
            WHERE extraction_id = $1
            "#,
        )
        .bind(id.0)
        .fetch_all(&self.pool)
        .await
        .context("Failed to get extraction provenance")?;

        Ok(rows
            .into_iter()
            .map(|r| FieldProvenance {
                field_path: r.get("field_path"),
                source_location: r.get("source_location"),
                extraction_method: r.get("extraction_method"),
            })
            .collect())
    }

    async fn find_extraction_by_fingerprint(
        &self,
        fingerprint: &ContentHash,
        schema_id: SchemaId,
        schema_version: u32,
    ) -> Result<Option<Extraction>> {
        let row = sqlx::query(
            r#"
            SELECT id, fingerprint, page_snapshot_id, schema_id, schema_version,
                   data, confidence_overall, confidence_heuristic, confidence_ai,
                   origin, extracted_at
            FROM extractions
            WHERE fingerprint = $1 AND schema_id = $2 AND schema_version = $3
            "#,
        )
        .bind(&fingerprint.0)
        .bind(schema_id.0)
        .bind(schema_version as i32)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to find extraction by fingerprint")?;

        Ok(row.map(|r| Extraction {
            id: ExtractionId(r.get("id")),
            fingerprint: ContentHash(r.get("fingerprint")),
            page_snapshot_id: PageSnapshotId(r.get("page_snapshot_id")),
            schema_id: SchemaId(r.get("schema_id")),
            schema_version: r.get::<i32, _>("schema_version") as u32,
            data: r.get("data"),
            confidence: ConfidenceScores {
                overall: r.get("confidence_overall"),
                heuristic: r.get("confidence_heuristic"),
                ai: r.get("confidence_ai"),
            },
            origin: serde_json::from_value(r.get("origin")).unwrap(),
            field_provenance: Vec::new(),
            extracted_at: r.get("extracted_at"),
        }))
    }

    async fn list_extractions_by_snapshot(
        &self,
        snapshot_id: PageSnapshotId,
    ) -> Result<Vec<Extraction>> {
        let rows = sqlx::query(
            r#"
            SELECT id, fingerprint, page_snapshot_id, schema_id, schema_version,
                   data, confidence_overall, confidence_heuristic, confidence_ai,
                   origin, extracted_at
            FROM extractions
            WHERE page_snapshot_id = $1
            ORDER BY extracted_at DESC
            "#,
        )
        .bind(snapshot_id.0)
        .fetch_all(&self.pool)
        .await
        .context("Failed to list extractions by snapshot")?;

        Ok(rows
            .into_iter()
            .map(|r| Extraction {
                id: ExtractionId(r.get("id")),
                fingerprint: ContentHash(r.get("fingerprint")),
                page_snapshot_id: PageSnapshotId(r.get("page_snapshot_id")),
                schema_id: SchemaId(r.get("schema_id")),
                schema_version: r.get::<i32, _>("schema_version") as u32,
                data: r.get("data"),
                confidence: ConfidenceScores {
                    overall: r.get("confidence_overall"),
                    heuristic: r.get("confidence_heuristic"),
                    ai: r.get("confidence_ai"),
                },
                origin: serde_json::from_value(r.get("origin")).unwrap(),
                field_provenance: Vec::new(),
                extracted_at: r.get("extracted_at"),
            })
            .collect())
    }

    async fn list_extractions_by_schema(&self, schema_id: SchemaId) -> Result<Vec<Extraction>> {
        let rows = sqlx::query(
            r#"
            SELECT id, fingerprint, page_snapshot_id, schema_id, schema_version,
                   data, confidence_overall, confidence_heuristic, confidence_ai,
                   origin, extracted_at
            FROM extractions
            WHERE schema_id = $1
            ORDER BY extracted_at DESC
            "#,
        )
        .bind(schema_id.0)
        .fetch_all(&self.pool)
        .await
        .context("Failed to list extractions by schema")?;

        Ok(rows
            .into_iter()
            .map(|r| Extraction {
                id: ExtractionId(r.get("id")),
                fingerprint: ContentHash(r.get("fingerprint")),
                page_snapshot_id: PageSnapshotId(r.get("page_snapshot_id")),
                schema_id: SchemaId(r.get("schema_id")),
                schema_version: r.get::<i32, _>("schema_version") as u32,
                data: r.get("data"),
                confidence: ConfidenceScores {
                    overall: r.get("confidence_overall"),
                    heuristic: r.get("confidence_heuristic"),
                    ai: r.get("confidence_ai"),
                },
                origin: serde_json::from_value(r.get("origin")).unwrap(),
                field_provenance: Vec::new(),
                extracted_at: r.get("extracted_at"),
            })
            .collect())
    }

    async fn save_relationship(&self, relationship: &Relationship) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO relationships (
                id, from_extraction_id, to_extraction_id, kind,
                confidence_overall, confidence_heuristic, confidence_ai,
                origin, metadata, created_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            ON CONFLICT (from_extraction_id, to_extraction_id, kind) DO NOTHING
            "#,
        )
        .bind(relationship.id.0)
        .bind(relationship.from_extraction_id.0)
        .bind(relationship.to_extraction_id.0)
        .bind(&relationship.kind)
        .bind(relationship.confidence.overall)
        .bind(relationship.confidence.heuristic)
        .bind(relationship.confidence.ai)
        .bind(serde_json::to_value(&relationship.origin)?)
        .bind(serde_json::to_value(&relationship.metadata)?)
        .bind(relationship.created_at)
        .execute(&self.pool)
        .await
        .context("Failed to save relationship")?;
        Ok(())
    }

    async fn get_relationship(&self, id: RelationshipId) -> Result<Option<Relationship>> {
        let row = sqlx::query(
            r#"
            SELECT id, from_extraction_id, to_extraction_id, kind,
                   confidence_overall, confidence_heuristic, confidence_ai,
                   origin, metadata, created_at
            FROM relationships
            WHERE id = $1
            "#,
        )
        .bind(id.0)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to get relationship")?;

        Ok(row.map(|r| Relationship {
            id: RelationshipId(r.get("id")),
            from_extraction_id: ExtractionId(r.get("from_extraction_id")),
            to_extraction_id: ExtractionId(r.get("to_extraction_id")),
            kind: r.get("kind"),
            confidence: ConfidenceScores {
                overall: r.get("confidence_overall"),
                heuristic: r.get("confidence_heuristic"),
                ai: r.get("confidence_ai"),
            },
            origin: serde_json::from_value(r.get("origin")).unwrap(),
            metadata: serde_json::from_value(r.get("metadata")).unwrap_or_default(),
            created_at: r.get("created_at"),
        }))
    }

    async fn list_relationships_from(&self, extraction_id: ExtractionId) -> Result<Vec<Relationship>> {
        let rows = sqlx::query(
            r#"
            SELECT id, from_extraction_id, to_extraction_id, kind,
                   confidence_overall, confidence_heuristic, confidence_ai,
                   origin, metadata, created_at
            FROM relationships
            WHERE from_extraction_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(extraction_id.0)
        .fetch_all(&self.pool)
        .await
        .context("Failed to list relationships from extraction")?;

        Ok(rows
            .into_iter()
            .map(|r| Relationship {
                id: RelationshipId(r.get("id")),
                from_extraction_id: ExtractionId(r.get("from_extraction_id")),
                to_extraction_id: ExtractionId(r.get("to_extraction_id")),
                kind: r.get("kind"),
                confidence: ConfidenceScores {
                    overall: r.get("confidence_overall"),
                    heuristic: r.get("confidence_heuristic"),
                    ai: r.get("confidence_ai"),
                },
                origin: serde_json::from_value(r.get("origin")).unwrap(),
                metadata: serde_json::from_value(r.get("metadata")).unwrap_or_default(),
                created_at: r.get("created_at"),
            })
            .collect())
    }

    async fn list_relationships_to(&self, extraction_id: ExtractionId) -> Result<Vec<Relationship>> {
        let rows = sqlx::query(
            r#"
            SELECT id, from_extraction_id, to_extraction_id, kind,
                   confidence_overall, confidence_heuristic, confidence_ai,
                   origin, metadata, created_at
            FROM relationships
            WHERE to_extraction_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(extraction_id.0)
        .fetch_all(&self.pool)
        .await
        .context("Failed to list relationships to extraction")?;

        Ok(rows
            .into_iter()
            .map(|r| Relationship {
                id: RelationshipId(r.get("id")),
                from_extraction_id: ExtractionId(r.get("from_extraction_id")),
                to_extraction_id: ExtractionId(r.get("to_extraction_id")),
                kind: r.get("kind"),
                confidence: ConfidenceScores {
                    overall: r.get("confidence_overall"),
                    heuristic: r.get("confidence_heuristic"),
                    ai: r.get("confidence_ai"),
                },
                origin: serde_json::from_value(r.get("origin")).unwrap(),
                metadata: serde_json::from_value(r.get("metadata")).unwrap_or_default(),
                created_at: r.get("created_at"),
            })
            .collect())
    }

    async fn list_relationships_by_kind(&self, kind: &str) -> Result<Vec<Relationship>> {
        let rows = sqlx::query(
            r#"
            SELECT id, from_extraction_id, to_extraction_id, kind,
                   confidence_overall, confidence_heuristic, confidence_ai,
                   origin, metadata, created_at
            FROM relationships
            WHERE kind = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(kind)
        .fetch_all(&self.pool)
        .await
        .context("Failed to list relationships by kind")?;

        Ok(rows
            .into_iter()
            .map(|r| Relationship {
                id: RelationshipId(r.get("id")),
                from_extraction_id: ExtractionId(r.get("from_extraction_id")),
                to_extraction_id: ExtractionId(r.get("to_extraction_id")),
                kind: r.get("kind"),
                confidence: ConfidenceScores {
                    overall: r.get("confidence_overall"),
                    heuristic: r.get("confidence_heuristic"),
                    ai: r.get("confidence_ai"),
                },
                origin: serde_json::from_value(r.get("origin")).unwrap(),
                metadata: serde_json::from_value(r.get("metadata")).unwrap_or_default(),
                created_at: r.get("created_at"),
            })
            .collect())
    }
}
