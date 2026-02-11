use restate_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::common::auth::restate_auth::require_admin;
use crate::common::{EmptyRequest, OrganizationId, SocialProfileId};
use crate::domains::social_profile::activities;
use crate::domains::social_profile::models::SocialProfile;
use crate::impl_restate_serde;
use crate::kernel::ServerDeps;

// =============================================================================
// Request types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSocialProfileRequest {
    pub organization_id: Uuid,
    pub platform: String,
    pub handle: String,
    pub url: Option<String>,
}

impl_restate_serde!(CreateSocialProfileRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListByOrganizationRequest {
    pub organization_id: Uuid,
}

impl_restate_serde!(ListByOrganizationRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteSocialProfileRequest {
    pub id: Uuid,
}

impl_restate_serde!(DeleteSocialProfileRequest);

// =============================================================================
// Response types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialProfileResult {
    pub id: String,
    pub organization_id: Option<String>,
    pub platform: String,
    pub handle: String,
    pub url: Option<String>,
    pub scrape_frequency_hours: i32,
    pub last_scraped_at: Option<String>,
    pub active: bool,
    pub created_at: String,
}

impl_restate_serde!(SocialProfileResult);

impl From<SocialProfile> for SocialProfileResult {
    fn from(sp: SocialProfile) -> Self {
        Self {
            id: sp.id.to_string(),
            organization_id: sp.organization_id.map(|id| id.to_string()),
            platform: sp.platform,
            handle: sp.handle,
            url: sp.url,
            scrape_frequency_hours: sp.scrape_frequency_hours,
            last_scraped_at: sp.last_scraped_at.map(|dt| dt.to_rfc3339()),
            active: sp.active,
            created_at: sp.created_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialProfileListResult {
    pub profiles: Vec<SocialProfileResult>,
}

impl_restate_serde!(SocialProfileListResult);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledScrapeResult {
    pub profiles_scraped: i32,
    pub posts_created: i32,
}

impl_restate_serde!(ScheduledScrapeResult);

// =============================================================================
// Service definition
// =============================================================================

#[restate_sdk::service]
#[name = "SocialProfiles"]
pub trait SocialProfilesService {
    async fn create(
        req: CreateSocialProfileRequest,
    ) -> Result<SocialProfileResult, HandlerError>;
    async fn list_by_organization(
        req: ListByOrganizationRequest,
    ) -> Result<SocialProfileListResult, HandlerError>;
    async fn delete(req: DeleteSocialProfileRequest) -> Result<EmptyRequest, HandlerError>;
    async fn run_scheduled_scrape(
        req: EmptyRequest,
    ) -> Result<ScheduledScrapeResult, HandlerError>;
}

pub struct SocialProfilesServiceImpl {
    deps: Arc<ServerDeps>,
}

impl SocialProfilesServiceImpl {
    pub fn with_deps(deps: Arc<ServerDeps>) -> Self {
        Self { deps }
    }
}

impl SocialProfilesService for SocialProfilesServiceImpl {
    async fn create(
        &self,
        ctx: Context<'_>,
        req: CreateSocialProfileRequest,
    ) -> Result<SocialProfileResult, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        let profile = SocialProfile::create(
            OrganizationId::from(req.organization_id),
            &req.platform,
            &req.handle,
            req.url.as_deref(),
            &self.deps.db_pool,
        )
        .await
        .map_err(|e| TerminalError::new(e.to_string()))?;

        Ok(SocialProfileResult::from(profile))
    }

    async fn list_by_organization(
        &self,
        ctx: Context<'_>,
        req: ListByOrganizationRequest,
    ) -> Result<SocialProfileListResult, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        let profiles = SocialProfile::find_by_organization(
            OrganizationId::from(req.organization_id),
            &self.deps.db_pool,
        )
        .await
        .map_err(|e| TerminalError::new(e.to_string()))?;

        Ok(SocialProfileListResult {
            profiles: profiles.into_iter().map(SocialProfileResult::from).collect(),
        })
    }

    async fn delete(
        &self,
        ctx: Context<'_>,
        req: DeleteSocialProfileRequest,
    ) -> Result<EmptyRequest, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        SocialProfile::delete(SocialProfileId::from(req.id), &self.deps.db_pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        Ok(EmptyRequest {})
    }

    async fn run_scheduled_scrape(
        &self,
        _ctx: Context<'_>,
        _req: EmptyRequest,
    ) -> Result<ScheduledScrapeResult, HandlerError> {
        let due = SocialProfile::find_due_for_scraping(&self.deps.db_pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        let mut profiles_scraped = 0;
        let mut total_posts_created = 0;

        for profile in &due {
            match profile.platform.as_str() {
                "instagram" => {
                    match activities::ingest_instagram_profile(profile, &self.deps).await {
                        Ok(post_ids) => {
                            profiles_scraped += 1;
                            total_posts_created += post_ids.len() as i32;
                        }
                        Err(e) => {
                            tracing::error!(
                                profile_id = %profile.id,
                                handle = %profile.handle,
                                error = %e,
                                "Failed to ingest Instagram profile"
                            );
                        }
                    }
                }
                platform => {
                    tracing::warn!(
                        platform,
                        handle = %profile.handle,
                        "Unsupported social platform, skipping"
                    );
                }
            }
        }

        Ok(ScheduledScrapeResult {
            profiles_scraped,
            posts_created: total_posts_created,
        })
    }
}
