use crate::common::WebsiteId;
use crate::domains::listings::data::ListingData;
use crate::domains::listings::models::listing::Listing;
use crate::domains::website::models::{Website, WebsiteSnapshot};
use crate::server::graphql::context::GraphQLContext;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// GraphQL-friendly representation of a website snapshot (scraped page)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebsiteSnapshotData {
    pub id: String,
    pub page_url: String,
    pub scrape_status: String,
    pub scrape_error: Option<String>,
    pub last_scraped_at: Option<String>,
    pub submitted_at: String,
}

impl From<WebsiteSnapshot> for WebsiteSnapshotData {
    fn from(snapshot: WebsiteSnapshot) -> Self {
        Self {
            id: snapshot.id.to_string(),
            page_url: snapshot.page_url,
            scrape_status: snapshot.scrape_status,
            scrape_error: snapshot.scrape_error,
            last_scraped_at: snapshot.last_scraped_at.map(|dt| dt.to_rfc3339()),
            submitted_at: snapshot.submitted_at.to_rfc3339(),
        }
    }
}

#[juniper::graphql_object(Context = GraphQLContext)]
impl WebsiteSnapshotData {
    fn id(&self) -> &str {
        &self.id
    }

    fn page_url(&self) -> &str {
        &self.page_url
    }

    fn scrape_status(&self) -> &str {
        &self.scrape_status
    }

    fn scrape_error(&self) -> Option<&str> {
        self.scrape_error.as_deref()
    }

    fn last_scraped_at(&self) -> Option<&str> {
        self.last_scraped_at.as_deref()
    }

    fn submitted_at(&self) -> &str {
        &self.submitted_at
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
    pub agent_id: Option<String>,
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
            agent_id: website.agent_id.map(|id| id.to_string()),
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

    fn agent_id(&self) -> Option<String> {
        self.agent_id.clone()
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

    /// Get count of website snapshots (submitted pages)
    async fn snapshots_count(&self, context: &GraphQLContext) -> juniper::FieldResult<i32> {
        let uuid = Uuid::parse_str(&self.id)?;
        let website_id = WebsiteId::from_uuid(uuid);
        let count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM website_snapshots WHERE website_id = $1",
        )
        .bind(website_id)
        .fetch_one(&context.db_pool)
        .await?;
        Ok(count as i32)
    }

    /// Get count of listings from this website
    async fn listings_count(&self, context: &GraphQLContext) -> juniper::FieldResult<i32> {
        let uuid = Uuid::parse_str(&self.id)?;
        let website_id = WebsiteId::from_uuid(uuid);
        let count =
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM listings WHERE website_id = $1")
                .bind(website_id)
                .fetch_one(&context.db_pool)
                .await?;
        Ok(count as i32)
    }

    /// Get all listings scraped from this website
    async fn listings(&self, context: &GraphQLContext) -> juniper::FieldResult<Vec<ListingData>> {
        let uuid = Uuid::parse_str(&self.id)?;
        let website_id = WebsiteId::from_uuid(uuid);
        let listings = Listing::find_by_website_id(website_id, &context.db_pool).await?;
        Ok(listings.into_iter().map(ListingData::from).collect())
    }

    /// Get all snapshots (scraped pages) for this website
    async fn snapshots(
        &self,
        context: &GraphQLContext,
    ) -> juniper::FieldResult<Vec<WebsiteSnapshotData>> {
        let uuid = Uuid::parse_str(&self.id)?;
        let website_id = WebsiteId::from_uuid(uuid);
        let snapshots = WebsiteSnapshot::find_by_website(&context.db_pool, website_id).await?;
        Ok(snapshots.into_iter().map(WebsiteSnapshotData::from).collect())
    }
}
