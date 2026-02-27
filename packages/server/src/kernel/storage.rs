//! S3-compatible storage adapter for media uploads.
//!
//! Uses the AWS SDK to talk to any S3-compatible endpoint (MinIO for local dev,
//! AWS S3 or R2 for production). Implements `BaseStorageService` from traits.rs.
//!
//! In Docker dev environments the server reaches MinIO via `minio:9000` (internal),
//! but presigned URLs must use `localhost:9000` (browser-reachable). We solve this
//! with two S3 clients: one for server operations, one for presigning.

use anyhow::Result;
use async_trait::async_trait;
use aws_sdk_s3::presigning::PresigningConfig;
use std::time::Duration;

use super::BaseStorageService;

/// S3-compatible storage adapter (works with MinIO, AWS S3, Cloudflare R2).
pub struct S3StorageAdapter {
    /// Client for server-side operations (delete, etc.) — uses internal endpoint.
    client: aws_sdk_s3::Client,
    /// Client for generating presigned URLs — uses browser-facing endpoint.
    presign_client: aws_sdk_s3::Client,
    bucket: String,
    public_url: String,
}

/// Build an S3 client for the given endpoint.
async fn build_s3_client(endpoint: Option<&str>, region: &str) -> aws_sdk_s3::Client {
    let mut config_loader = aws_config::defaults(aws_config::BehaviorVersion::latest())
        .region(aws_config::Region::new(region.to_string()));
    if let Some(ep) = endpoint {
        config_loader = config_loader.endpoint_url(ep);
    }
    let sdk_config = config_loader.load().await;

    let mut s3_config = aws_sdk_s3::config::Builder::from(&sdk_config)
        .force_path_style(endpoint.is_some());
    if let Some(ep) = endpoint {
        s3_config = s3_config.endpoint_url(ep);
    }

    aws_sdk_s3::Client::from_conf(s3_config.build())
}

impl S3StorageAdapter {
    /// Create a new adapter.
    ///
    /// - `endpoint`: Internal S3 endpoint, e.g. `Some("http://minio:9000")`.
    /// - `presign_endpoint`: Browser-facing endpoint for presigned URLs, e.g.
    ///   `Some("http://localhost:9000")`. Falls back to `endpoint` if not set.
    ///   Only needed in Docker dev where internal ≠ external hostname.
    pub async fn new(
        endpoint: Option<&str>,
        presign_endpoint: Option<&str>,
        region: &str,
        bucket: &str,
        public_url: &str,
    ) -> Self {
        let client = build_s3_client(endpoint, region).await;

        // Use a separate client for presigning if browser endpoint differs
        let presign_client = match presign_endpoint {
            Some(pe) if Some(pe) != endpoint => build_s3_client(Some(pe), region).await,
            _ => client.clone(),
        };

        Self {
            client,
            presign_client,
            bucket: bucket.to_string(),
            public_url: public_url.trim_end_matches('/').to_string(),
        }
    }
}

#[async_trait]
impl BaseStorageService for S3StorageAdapter {
    async fn presigned_upload_url(
        &self,
        key: &str,
        content_type: &str,
        expires_secs: u64,
    ) -> Result<String> {
        let presigning = PresigningConfig::expires_in(Duration::from_secs(expires_secs))
            .map_err(|e| anyhow::anyhow!("presign config: {}", e))?;

        // Use the presign_client so the URL contains the browser-reachable host
        let presigned = self
            .presign_client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .content_type(content_type)
            .presigned(presigning)
            .await
            .map_err(|e| anyhow::anyhow!("presign error: {}", e))?;

        Ok(presigned.uri().to_string())
    }

    async fn delete(&self, key: &str) -> Result<()> {
        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("delete error: {}", e))?;
        Ok(())
    }

    fn public_url(&self, key: &str) -> String {
        format!("{}/{}", self.public_url, key)
    }
}
