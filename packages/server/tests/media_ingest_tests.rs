//! End-to-end integration test for the Root Signal media ingest
//! pipeline.
//!
//! What this exercises:
//!
//!   * Boot testcontainers Postgres and apply every migration
//!     (`./migrations`) — so the DB state matches production.
//!   * Build a `ServerDeps` with an in-memory `BaseStorageService`
//!     mock so we can assert what landed in "MinIO" without standing
//!     up a real S3.
//!   * Create a minimum-viable post via `Post::create`.
//!   * Call `ingest_from_body` with a real JPEG fixture; assert the
//!     resulting `media` row, `post_media` row, and stored object
//!     round-trip.
//!   * Call it a second time with the same bytes; assert dedup
//!     (`reused_existing = true`, only one row in `media`).
//!
//! Why `ingest_from_body` and not `ingest_source_image`: the SSRF
//! guard plus `reqwest::https_only(true)` block the obvious
//! `http://127.0.0.1:{port}` test-server trick. A full end-to-end
//! test would require a TLS cert; deferred. The fetch layer is
//! covered by the SSRF + format-detection unit tests. Everything
//! past fetch — magic bytes, normalise, EXIF strip, content-hash
//! dedup, storage put, DB writes, post linkage — runs here.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use async_trait::async_trait;
use server_core::domains::media::activities::ingest_from_body;
use server_core::domains::media::models::{Media, MediaReference};
use server_core::domains::posts::models::{CreatePost, Post};
use server_core::kernel::{BaseStorageService, TestDependencies};
use sqlx::PgPool;
use testcontainers::core::{ContainerPort, WaitFor};
use testcontainers::{runners::AsyncRunner, ContainerAsync, GenericImage, ImageExt};

// -----------------------------------------------------------------------------
// In-memory storage mock
// -----------------------------------------------------------------------------

#[derive(Clone, Default)]
struct InMemoryStorage {
    objects: Arc<Mutex<HashMap<String, (Vec<u8>, String)>>>,
    public_base: String,
}

impl InMemoryStorage {
    fn new() -> Self {
        Self {
            objects: Arc::new(Mutex::new(HashMap::new())),
            public_base: "http://minio.test/bucket".to_string(),
        }
    }

    fn object_keys(&self) -> Vec<String> {
        self.objects
            .lock()
            .unwrap()
            .keys()
            .cloned()
            .collect()
    }

    fn get(&self, key: &str) -> Option<(Vec<u8>, String)> {
        self.objects.lock().unwrap().get(key).cloned()
    }
}

#[async_trait]
impl BaseStorageService for InMemoryStorage {
    async fn presigned_upload_url(
        &self,
        _key: &str,
        _content_type: &str,
        _expires_secs: u64,
    ) -> Result<String> {
        // Not used by the ingest path — ingest writes via put_object.
        unimplemented!("presigned_upload_url not needed for ingest tests");
    }

    async fn put_object(
        &self,
        key: &str,
        body: Vec<u8>,
        content_type: &str,
    ) -> Result<()> {
        self.objects
            .lock()
            .unwrap()
            .insert(key.to_string(), (body, content_type.to_string()));
        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<()> {
        self.objects.lock().unwrap().remove(key);
        Ok(())
    }

    fn public_url(&self, key: &str) -> String {
        format!("{}/{}", self.public_base, key)
    }
}

// -----------------------------------------------------------------------------
// Postgres bootstrap
// -----------------------------------------------------------------------------

async fn bootstrap_db() -> (ContainerAsync<GenericImage>, PgPool) {
    // Match the production image. Migration 000001 runs `CREATE
    // EXTENSION vector`, which is only present on pgvector/pgvector;
    // the vanilla postgres image fails the migration step.
    let container = GenericImage::new("pgvector/pgvector", "pg16")
        .with_exposed_port(ContainerPort::Tcp(5432))
        .with_wait_for(WaitFor::message_on_stderr(
            "database system is ready to accept connections",
        ))
        .with_env_var("POSTGRES_PASSWORD", "postgres")
        .with_env_var("POSTGRES_USER", "postgres")
        // Match the production DB name — migrations 000195/000196
        // do `ALTER DATABASE rooteditorial SET pg_trgm.*` and fail if
        // the DB is named anything else.
        .with_env_var("POSTGRES_DB", "rooteditorial")
        .start()
        .await
        .expect("start postgres container");

    let host = container
        .get_host()
        .await
        .expect("get container host");
    let host_port = container
        .get_host_port_ipv4(5432)
        .await
        .expect("map postgres port");
    let url = format!("postgres://postgres:postgres@{host}:{host_port}/rooteditorial");

    let pool = PgPool::connect(&url)
        .await
        .expect("connect to pg");

    // Apply every migration in ./migrations. The test is pinned to
    // the live migration set by the `migrate!` macro's compile-time
    // directory reference, so schema drift between HEAD and the test
    // fixture can't silently happen.
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    (container, pool)
}

// -----------------------------------------------------------------------------
// JPEG fixture generator
// -----------------------------------------------------------------------------

fn make_jpeg_fixture(seed: u8) -> Vec<u8> {
    use std::io::Cursor;
    let img = image::DynamicImage::ImageRgb8(image::RgbImage::from_fn(
        32,
        16,
        |x, y| {
            let r = ((x as u16 * 7 + seed as u16) % 256) as u8;
            let g = ((y as u16 * 13 + seed as u16) % 256) as u8;
            image::Rgb([r, g, seed])
        },
    ));
    let mut out = Vec::new();
    img.write_to(&mut Cursor::new(&mut out), image::ImageFormat::Jpeg)
        .expect("encode jpeg fixture");
    // Sanity: JPEG SOI bytes.
    assert_eq!(&out[0..3], &[0xFF, 0xD8, 0xFF]);
    out
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[tokio::test]
async fn end_to_end_ingest_creates_media_and_links_post() {
    let (_container, pool) = bootstrap_db().await;

    let storage = Arc::new(InMemoryStorage::new());
    let deps = TestDependencies::new()
        .with_storage(storage.clone())
        .into_server_deps(pool.clone());

    // Minimum-viable post — we only care that post_media.post_id
    // points at a real row.
    let post = Post::create(
        CreatePost::builder()
            .title("A post".to_string())
            .body_raw("x".repeat(260))
            .post_type("story")
            .build(),
        &pool,
    )
    .await
    .expect("create post");

    let jpeg = make_jpeg_fixture(42);
    let source_url = "https://example.org/photos/volunteers.jpg";

    let result = ingest_from_body(
        source_url,
        post.id.into(),
        jpeg.clone(),
        Some("Volunteers sorting produce"),
        Some("Photo by Jane Doe"),
        Some("Three volunteers sorting produce into crates"),
        &deps,
    )
    .await
    .expect("ingest");

    assert!(!result.reused_existing, "first ingest must not dedup");

    // Media row: present, content_hash populated, storage_key + url
    // both point at the in-memory bucket.
    let media = Media::find_by_id(result.media_id, &pool)
        .await
        .expect("find media")
        .expect("media exists");
    assert_eq!(media.content_type, "image/webp");
    assert!(media.content_hash.is_some(), "content_hash populated");
    assert_eq!(media.source_url.as_deref(), Some(source_url));
    assert!(media.source_ingested_at.is_some());
    // Must NOT be the raw upstream URL — this is the whole point of
    // the pipeline: publication references internal storage.
    assert!(
        media.url.starts_with("http://minio.test/bucket/media/"),
        "media.url should point at internal bucket, was {}",
        media.url,
    );
    assert_ne!(media.url, source_url);

    // The object is actually in storage, as WebP.
    let stored = storage
        .get(&media.storage_key)
        .expect("object in storage");
    assert_eq!(stored.1, "image/webp");
    assert_eq!(&stored.0[0..4], b"RIFF");
    assert_eq!(&stored.0[8..12], b"WEBP");

    // post_media row points at the new media.
    let pm: (Option<uuid::Uuid>, Option<String>, Option<String>) =
        sqlx::query_as(
            "SELECT media_id, caption, credit FROM post_media WHERE post_id = $1",
        )
        .bind::<uuid::Uuid>(post.id.into())
        .fetch_one(&pool)
        .await
        .expect("fetch post_media");
    assert_eq!(pm.0, Some(media.id));
    assert_eq!(pm.1.as_deref(), Some("Volunteers sorting produce"));
    assert_eq!(pm.2.as_deref(), Some("Photo by Jane Doe"));

    // alt_text was backfilled onto the media row from the envelope.
    assert_eq!(
        media.alt_text.as_deref(),
        Some("Three volunteers sorting produce into crates"),
    );

    // Polymorphic media_reference for post_hero exists.
    let refs = MediaReference::find_by_entity("post_hero", post.id.into(), &pool)
        .await
        .expect("find refs");
    assert_eq!(refs.len(), 1);
    assert_eq!(refs[0].media_id, media.id);
}

#[tokio::test]
async fn second_ingest_of_same_bytes_dedups() {
    let (_container, pool) = bootstrap_db().await;

    let storage = Arc::new(InMemoryStorage::new());
    let deps = TestDependencies::new()
        .with_storage(storage.clone())
        .into_server_deps(pool.clone());

    let post_a = Post::create(
        CreatePost::builder()
            .title("Post A".to_string())
            .body_raw("a".repeat(260))
            .post_type("story")
            .build(),
        &pool,
    )
    .await
    .unwrap();
    let post_b = Post::create(
        CreatePost::builder()
            .title("Post B".to_string())
            .body_raw("b".repeat(260))
            .post_type("story")
            .build(),
        &pool,
    )
    .await
    .unwrap();

    let jpeg = make_jpeg_fixture(99);

    let first = ingest_from_body(
        "https://example.org/a.jpg",
        post_a.id.into(),
        jpeg.clone(),
        None,
        None,
        None,
        &deps,
    )
    .await
    .unwrap();
    assert!(!first.reused_existing);

    // Second submission of identical bytes — different source URL,
    // different post. Must reuse the media row.
    let second = ingest_from_body(
        "https://other.example/duplicate.jpg",
        post_b.id.into(),
        jpeg.clone(),
        None,
        None,
        None,
        &deps,
    )
    .await
    .unwrap();
    assert!(second.reused_existing, "second ingest must dedup");
    assert_eq!(
        first.media_id, second.media_id,
        "dedup must reuse the same media_id",
    );

    // Only one storage object, only one media row.
    assert_eq!(storage.object_keys().len(), 1, "only one object stored");
    let (media_count,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM media WHERE source_url IS NOT NULL")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(media_count, 1);

    // Both posts link to the same media row.
    let rows: Vec<(uuid::Uuid,)> = sqlx::query_as(
        "SELECT media_id FROM post_media WHERE post_id = ANY($1) AND media_id IS NOT NULL",
    )
    .bind(vec![uuid::Uuid::from(post_a.id), uuid::Uuid::from(post_b.id)])
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(rows.len(), 2);
    for (mid,) in rows {
        assert_eq!(mid, first.media_id);
    }
}

#[tokio::test]
async fn different_bytes_produce_distinct_media_rows() {
    let (_container, pool) = bootstrap_db().await;

    let storage = Arc::new(InMemoryStorage::new());
    let deps = TestDependencies::new()
        .with_storage(storage.clone())
        .into_server_deps(pool.clone());

    let post = Post::create(
        CreatePost::builder()
            .title("P".to_string())
            .body_raw("p".repeat(260))
            .post_type("story")
            .build(),
        &pool,
    )
    .await
    .unwrap();

    let j1 = make_jpeg_fixture(1);
    let j2 = make_jpeg_fixture(200);

    let r1 = ingest_from_body(
        "https://example.org/1.jpg",
        post.id.into(),
        j1,
        None,
        None,
        None,
        &deps,
    )
    .await
    .unwrap();
    let r2 = ingest_from_body(
        "https://example.org/2.jpg",
        post.id.into(),
        j2,
        None,
        None,
        None,
        &deps,
    )
    .await
    .unwrap();

    assert_ne!(r1.media_id, r2.media_id, "distinct bytes => distinct rows");
    assert!(!r1.reused_existing);
    assert!(!r2.reused_existing);
    assert_eq!(storage.object_keys().len(), 2);
}

#[tokio::test]
async fn rejects_non_image_body() {
    let (_container, pool) = bootstrap_db().await;

    let storage = Arc::new(InMemoryStorage::new());
    let deps = TestDependencies::new()
        .with_storage(storage.clone())
        .into_server_deps(pool.clone());

    let post = Post::create(
        CreatePost::builder()
            .title("P".to_string())
            .body_raw("p".repeat(260))
            .post_type("story")
            .build(),
        &pool,
    )
    .await
    .unwrap();

    let html = b"<!DOCTYPE html><html><body>not an image</body></html>".to_vec();
    let err = ingest_from_body(
        "https://example.org/phishing.jpg",
        post.id.into(),
        html,
        None,
        None,
        None,
        &deps,
    )
    .await
    .unwrap_err();

    assert!(
        matches!(
            err,
            server_core::domains::media::activities::IngestError::Validate(_)
        ),
        "expected IngestError::Validate, got {err:?}",
    );

    // Nothing written to storage or to media table.
    assert!(storage.object_keys().is_empty());
    let (n,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM media")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(n, 0);
}
