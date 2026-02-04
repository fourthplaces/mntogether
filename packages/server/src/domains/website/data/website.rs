//! Website data types for GraphQL.

use crate::common::WebsiteId;
use crate::domains::extraction::ExtractionPageData;
use crate::domains::posts::data::PostData;
use crate::domains::posts::models::post::Post;
use crate::domains::website::models::Website;
use crate::server::graphql::context::GraphQLContext;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// GraphQL-friendly representation of a crawl job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawlJobData {
    pub job_id: String,
    pub job_type: String,
    pub status: String,
    pub error_message: Option<String>,
}

impl From<crate::domains::crawling::JobInfo> for CrawlJobData {
    fn from(job: crate::domains::crawling::JobInfo) -> Self {
        Self {
            job_id: job.job_id.to_string(),
            job_type: job.job_type,
            status: job.status,
            error_message: job.error_message,
        }
    }
}

#[juniper::graphql_object(Context = GraphQLContext)]
impl CrawlJobData {
    fn job_id(&self) -> &str {
        &self.job_id
    }

    fn job_type(&self) -> &str {
        &self.job_type
    }

    fn status(&self) -> &str {
        &self.status
    }

    fn error_message(&self) -> Option<&str> {
        self.error_message.as_deref()
    }
}

/// GraphQL-friendly representation of a website (for scraping/monitoring)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebsiteData {
    pub id: String,
    pub domain: String,
    pub last_scraped_at: Option<String>,
    pub scrape_frequency_hours: i32,
    pub active: bool,
    pub status: String,
    pub submitted_by: Option<String>,
    pub submitter_type: Option<String>,
    pub created_at: String,
    // Crawl tracking fields
    pub crawl_status: Option<String>,
    pub crawl_attempt_count: Option<i32>,
    pub max_crawl_retries: Option<i32>,
    pub last_crawl_started_at: Option<String>,
    pub last_crawl_completed_at: Option<String>,
    pub pages_crawled_count: Option<i32>,
    pub max_pages_per_crawl: Option<i32>,
}

impl From<Website> for WebsiteData {
    fn from(website: Website) -> Self {
        Self {
            id: website.id.to_string(),
            domain: website.domain,
            last_scraped_at: website.last_scraped_at.map(|dt| dt.to_rfc3339()),
            scrape_frequency_hours: website.scrape_frequency_hours,
            active: website.active,
            status: website.status,
            submitted_by: website.submitted_by.map(|id| id.to_string()),
            submitter_type: website.submitter_type,
            created_at: website.created_at.to_rfc3339(),
            // Crawl tracking fields
            crawl_status: website.crawl_status,
            crawl_attempt_count: website.crawl_attempt_count,
            max_crawl_retries: website.max_crawl_retries,
            last_crawl_started_at: website.last_crawl_started_at.map(|dt| dt.to_rfc3339()),
            last_crawl_completed_at: website.last_crawl_completed_at.map(|dt| dt.to_rfc3339()),
            pages_crawled_count: website.pages_crawled_count,
            max_pages_per_crawl: website.max_pages_per_crawl,
        }
    }
}

#[juniper::graphql_object(Context = GraphQLContext)]
impl WebsiteData {
    fn id(&self) -> String {
        self.id.clone()
    }

    fn domain(&self) -> String {
        self.domain.clone()
    }

    /// Alias for domain (backward compatibility)
    fn url(&self) -> String {
        self.domain.clone()
    }

    fn last_scraped_at(&self) -> Option<String> {
        self.last_scraped_at.clone()
    }

    fn scrape_frequency_hours(&self) -> i32 {
        self.scrape_frequency_hours
    }

    fn active(&self) -> bool {
        self.active
    }

    fn status(&self) -> String {
        self.status.clone()
    }

    fn submitted_by(&self) -> Option<String> {
        self.submitted_by.clone()
    }

    fn submitter_type(&self) -> Option<String> {
        self.submitter_type.clone()
    }

    fn created_at(&self) -> String {
        self.created_at.clone()
    }

    // Crawl tracking fields
    fn crawl_status(&self) -> Option<String> {
        self.crawl_status.clone()
    }

    fn crawl_attempt_count(&self) -> Option<i32> {
        self.crawl_attempt_count
    }

    fn max_crawl_retries(&self) -> Option<i32> {
        self.max_crawl_retries
    }

    fn last_crawl_started_at(&self) -> Option<String> {
        self.last_crawl_started_at.clone()
    }

    fn last_crawl_completed_at(&self) -> Option<String> {
        self.last_crawl_completed_at.clone()
    }

    fn pages_crawled_count(&self) -> Option<i32> {
        self.pages_crawled_count
    }

    fn max_pages_per_crawl(&self) -> Option<i32> {
        self.max_pages_per_crawl
    }

    /// Get count of extraction pages for this website's domain
    async fn snapshots_count(&self, context: &GraphQLContext) -> juniper::FieldResult<i32> {
        let count = ExtractionPageData::count_by_domain(&self.domain, &context.db_pool)
            .await
            .map_err(|e| juniper::FieldError::new(e.to_string(), juniper::Value::null()))?;
        Ok(count)
    }

    /// Get count of listings from this website (excludes soft-deleted)
    async fn listings_count(&self, context: &GraphQLContext) -> juniper::FieldResult<i32> {
        let uuid = Uuid::parse_str(&self.id)?;
        let website_id = WebsiteId::from_uuid(uuid);
        let count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM posts WHERE website_id = $1 AND deleted_at IS NULL",
        )
        .bind(website_id)
        .fetch_one(&context.db_pool)
        .await?;
        Ok(count as i32)
    }

    /// Get all listings scraped from this website
    async fn listings(&self, context: &GraphQLContext) -> juniper::FieldResult<Vec<PostData>> {
        let uuid = Uuid::parse_str(&self.id)?;
        let website_id = WebsiteId::from_uuid(uuid);
        let posts = Post::find_by_website_id(website_id, &context.db_pool).await?;
        Ok(posts.into_iter().map(PostData::from).collect())
    }

    /// Get all extraction pages for this website's domain
    ///
    /// This now queries the extraction_pages table instead of the deprecated
    /// website_snapshots table.
    async fn snapshots(
        &self,
        context: &GraphQLContext,
    ) -> juniper::FieldResult<Vec<ExtractionPageData>> {
        let pages = ExtractionPageData::find_by_domain(&self.domain, 100, &context.db_pool)
            .await
            .map_err(|e| juniper::FieldError::new(e.to_string(), juniper::Value::null()))?;
        Ok(pages)
    }

    /// Get extraction pages for this website's domain (alias for snapshots)
    async fn extraction_pages(
        &self,
        context: &GraphQLContext,
        limit: Option<i32>,
    ) -> juniper::FieldResult<Vec<ExtractionPageData>> {
        let limit = limit.unwrap_or(100);
        let pages = ExtractionPageData::find_by_domain(&self.domain, limit, &context.db_pool)
            .await
            .map_err(|e| juniper::FieldError::new(e.to_string(), juniper::Value::null()))?;
        Ok(pages)
    }

    /// Get the latest crawl job for this website
    async fn crawl_job(
        &self,
        context: &GraphQLContext,
    ) -> juniper::FieldResult<Option<CrawlJobData>> {
        use crate::domains::crawling::{CrawlWebsiteJob, JobInfo};
        let uuid = Uuid::parse_str(&self.id)?;
        let job =
            JobInfo::find_latest_for_website(uuid, CrawlWebsiteJob::JOB_TYPE, &context.db_pool)
                .await?;
        Ok(job.map(CrawlJobData::from))
    }

    /// Get the latest regenerate posts job for this website
    async fn regenerate_posts_job(
        &self,
        context: &GraphQLContext,
    ) -> juniper::FieldResult<Option<CrawlJobData>> {
        use crate::domains::crawling::{JobInfo, RegeneratePostsJob};
        let uuid = Uuid::parse_str(&self.id)?;
        let job =
            JobInfo::find_latest_for_website(uuid, RegeneratePostsJob::JOB_TYPE, &context.db_pool)
                .await?;
        Ok(job.map(CrawlJobData::from))
    }
}

// ============================================================================
// Relay Pagination Types
// ============================================================================

/// Edge containing a website and its cursor (Relay spec)
#[derive(Debug, Clone)]
pub struct WebsiteEdge {
    pub node: WebsiteData,
    pub cursor: String,
}

#[juniper::graphql_object(Context = GraphQLContext)]
impl WebsiteEdge {
    /// The website at the end of the edge
    fn node(&self) -> &WebsiteData {
        &self.node
    }
    /// A cursor for pagination
    fn cursor(&self) -> &str {
        &self.cursor
    }
}

/// Connection type for paginated websites (Relay spec)
#[derive(Debug, Clone)]
pub struct WebsiteConnection {
    pub edges: Vec<WebsiteEdge>,
    pub page_info: crate::common::PageInfo,
    pub total_count: i32,
}

#[juniper::graphql_object(Context = GraphQLContext)]
impl WebsiteConnection {
    /// A list of edges (website + cursor pairs)
    fn edges(&self) -> &[WebsiteEdge] {
        &self.edges
    }
    /// Information about pagination
    fn page_info(&self) -> &crate::common::PageInfo {
        &self.page_info
    }
    /// Total count of websites matching the filter
    fn total_count(&self) -> i32 {
        self.total_count
    }
    /// Convenience: direct access to nodes (for simpler queries)
    fn nodes(&self) -> Vec<&WebsiteData> {
        self.edges.iter().map(|e| &e.node).collect()
    }
}
