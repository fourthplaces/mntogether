//! Pure Apify REST API client.
//!
//! A minimal client for the Apify platform API. Supports starting actor runs,
//! polling for completion, and fetching dataset results.
//!
//! # Example
//!
//! ```rust,ignore
//! use apify_client::ApifyClient;
//!
//! let client = ApifyClient::new("your-api-token".into());
//!
//! let posts = client.scrape_instagram_posts("natgeo", 50).await?;
//! for post in &posts {
//!     println!("{}", post.caption.as_deref().unwrap_or("(no caption)"));
//! }
//! ```

pub mod error;
pub mod types;

pub use error::{ApifyError, Result};
pub use types::{InstagramPost, InstagramScraperInput, RunData};

use serde::de::DeserializeOwned;
use types::ApiResponse;

const BASE_URL: &str = "https://api.apify.com/v2";

/// Actor ID for apify/instagram-post-scraper.
const INSTAGRAM_POST_SCRAPER: &str = "nH2AHrwxeTRJoN5hX";

pub struct ApifyClient {
    client: reqwest::Client,
    token: String,
}

impl ApifyClient {
    pub fn new(token: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            token,
        }
    }

    /// Start an Instagram profile scrape run. Returns immediately with run metadata.
    pub async fn start_instagram_scrape(
        &self,
        username: &str,
        limit: u32,
    ) -> Result<RunData> {
        let input = InstagramScraperInput {
            username: vec![username.to_string()],
            results_limit: limit,
        };

        let url = format!("{}/acts/{}/runs", BASE_URL, INSTAGRAM_POST_SCRAPER);
        let resp = self
            .client
            .post(&url)
            .bearer_auth(&self.token)
            .json(&input)
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(ApifyError::Api {
                status: status.as_u16(),
                message: body,
            });
        }

        let api_resp: ApiResponse<RunData> = resp.json().await?;
        Ok(api_resp.data)
    }

    /// Poll until a run completes. Uses `waitForFinish=60` for efficient long-polling.
    pub async fn wait_for_run(&self, run_id: &str) -> Result<RunData> {
        loop {
            let url = format!(
                "{}/actor-runs/{}?waitForFinish=60",
                BASE_URL, run_id
            );
            let resp = self
                .client
                .get(&url)
                .bearer_auth(&self.token)
                .send()
                .await?;

            let status = resp.status();
            if !status.is_success() {
                let body = resp.text().await.unwrap_or_default();
                return Err(ApifyError::Api {
                    status: status.as_u16(),
                    message: body,
                });
            }

            let api_resp: ApiResponse<RunData> = resp.json().await?;
            match api_resp.data.status.as_str() {
                "SUCCEEDED" => return Ok(api_resp.data),
                "FAILED" | "ABORTED" | "TIMED-OUT" => {
                    return Err(ApifyError::RunFailed(api_resp.data.status));
                }
                _ => {
                    tracing::debug!(run_id, status = %api_resp.data.status, "Run still in progress");
                    continue;
                }
            }
        }
    }

    /// Fetch dataset items from a completed run.
    pub async fn get_dataset_items<T: DeserializeOwned>(
        &self,
        dataset_id: &str,
    ) -> Result<Vec<T>> {
        let url = format!(
            "{}/datasets/{}/items?format=json",
            BASE_URL, dataset_id
        );
        let resp = self
            .client
            .get(&url)
            .bearer_auth(&self.token)
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(ApifyError::Api {
                status: status.as_u16(),
                message: body,
            });
        }

        let items: Vec<T> = resp.json().await?;
        Ok(items)
    }

    /// Scrape Instagram profile posts end-to-end: start run, poll, fetch results.
    pub async fn scrape_instagram_posts(
        &self,
        username: &str,
        limit: u32,
    ) -> Result<Vec<InstagramPost>> {
        tracing::info!(username, limit, "Starting Instagram profile scrape");

        let run = self.start_instagram_scrape(username, limit).await?;
        tracing::info!(run_id = %run.id, "Apify run started, polling for completion");

        let completed = self.wait_for_run(&run.id).await?;
        tracing::info!(
            run_id = %completed.id,
            dataset_id = %completed.default_dataset_id,
            "Run completed, fetching results"
        );

        let posts: Vec<InstagramPost> = self
            .get_dataset_items(&completed.default_dataset_id)
            .await?;
        tracing::info!(count = posts.len(), "Fetched Instagram posts");

        Ok(posts)
    }
}
