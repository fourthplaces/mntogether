//! Organisation dedup ladder (spec §7.1).
//!
//! Four steps, in order:
//!
//!   1. `already_known_org_id` — client-supplied, trusted. Direct lookup.
//!   2. `website` domain match — normalise, strip `www.` + trailing slash,
//!      look up via `website_sources.domain`.
//!   3. Exact `name` match (case-insensitive).
//!   4. Insert a new `organizations` row and create a website source for it.
//!
//! Returns the organisation plus a `source_id` suitable for feeding into
//! `post_sources.source_id`. When the website URL is absent we still need a
//! source row — falls back to an `other` source in that case so post_sources
//! always has something to point at. (The `source_type` on the resulting
//! `post_sources` row is 'website' when a domain is known, else the ingest
//! orchestrator decides based on the citation's platform context.)
//!
//! "Stale metadata" soft-fail: when a stored org has non-NULL metadata that
//! conflicts with the submission (e.g. different website), we flag the
//! `SourceStale` signal for the orchestrator without applying the change.

use anyhow::Result;
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::utils::urls::normalise_domain;
use crate::common::OrganizationId;
use crate::domains::organization::models::Organization;

#[derive(Debug, Clone)]
pub struct OrganizationSubmission<'a> {
    pub name: &'a str,
    pub website: Option<&'a str>,
    pub description: Option<&'a str>,
    pub already_known_org_id: Option<Uuid>,
}

#[derive(Debug, Clone)]
pub struct ResolvedOrganization {
    pub org: Organization,
    /// A `sources.id` row owned by this organisation that `post_sources.source_id`
    /// can reference. Guaranteed non-null for ingested posts.
    pub source_id: Uuid,
    pub source_type: String,
    /// Freshly-inserted vs matched existing.
    pub created: bool,
    /// True when the stored row has conflicting non-NULL metadata. Orchestrator
    /// lands the post `in_review` with `source_stale`.
    pub stale: bool,
}

/// Execute the dedup ladder. When the submission has no website (rare for
/// organisations but possible for small community groups) we skip straight to
/// step 3.
pub async fn resolve_organization(
    submission: OrganizationSubmission<'_>,
    pool: &PgPool,
) -> Result<ResolvedOrganization> {
    // ------ Step 1 — client-hinted id -----------------------------------------
    if let Some(hint) = submission.already_known_org_id {
        let org = Organization::find_by_id(OrganizationId::from_uuid(hint), pool).await;
        if let Ok(org) = org {
            return finalise_existing(org, &submission, pool).await;
        }
        // Fall through: unknown id → keep dedupping rather than 422.
        tracing::debug!(
            ?hint,
            "already_known_org_id did not resolve; falling back to dedup ladder"
        );
    }

    // ------ Step 2 — website domain match -------------------------------------
    let domain = submission.website.and_then(normalise_domain);
    if let Some(domain) = domain.clone() {
        if let Some(org) = Organization::find_by_website_domain(&domain, pool).await? {
            return finalise_existing(org, &submission, pool).await;
        }
    }

    // ------ Step 3 — exact name match -----------------------------------------
    if let Some(org) = Organization::find_by_name(submission.name, pool).await? {
        // Name matched but website didn't — soft-fail per spec §11.2.
        let mut resolved = finalise_existing(org, &submission, pool).await?;
        if domain.is_some() {
            resolved.stale = true;
        }
        return Ok(resolved);
    }

    // ------ Step 4 — insert new ------------------------------------------------
    let org = Organization::create_with_source_type(
        submission.name,
        submission.description,
        "root_signal",
        "root_signal",
        pool,
    )
    .await?;

    let (source_id, source_type) = ensure_source_for_org(&org, &submission, pool).await?;

    Ok(ResolvedOrganization {
        org,
        source_id,
        source_type,
        created: true,
        stale: false,
    })
}

async fn finalise_existing(
    org: Organization,
    submission: &OrganizationSubmission<'_>,
    pool: &PgPool,
) -> Result<ResolvedOrganization> {
    // Enrich NULL columns from the submission. Non-NULL conflicts are left
    // alone — the orchestrator detects drift via the returned `stale` flag.
    let org = Organization::enrich_if_null(org.id, &org.name, submission.description, pool).await?;

    let (source_id, source_type) = ensure_source_for_org(&org, submission, pool).await?;

    Ok(ResolvedOrganization {
        org,
        source_id,
        source_type,
        created: false,
        stale: false,
    })
}

/// Ensure the organisation has a `sources` row we can attach `post_sources` to.
/// Prefers the submission's website domain; falls back to any existing source.
async fn ensure_source_for_org(
    org: &Organization,
    submission: &OrganizationSubmission<'_>,
    pool: &PgPool,
) -> Result<(Uuid, String)> {
    if let (Some(url), Some(domain)) = (
        submission.website,
        submission.website.and_then(normalise_domain),
    ) {
        let source_id = Organization::ensure_website_source(org.id, &domain, url, pool).await?;
        return Ok((source_id, "website".to_string()));
    }

    if let Some(existing) = Organization::primary_source_id(org.id, pool).await? {
        // Inspect type cheaply.
        let source_type: String = sqlx::query_scalar("SELECT source_type FROM sources WHERE id = $1")
            .bind(existing)
            .fetch_one(pool)
            .await?;
        return Ok((existing, source_type));
    }

    // No website, no existing source — mint a bare `sources` row. Source_type
    // 'other' avoids asserting a channel we don't know about.
    let source_id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO sources (source_type, url, organization_id, status, active)
        VALUES ('other', NULL, $1, 'approved', true)
        RETURNING id
        "#,
    )
    .bind(org.id)
    .fetch_one(pool)
    .await?;

    Ok((source_id, "other".to_string()))
}
