//! Jobs service (stateless)
//!
//! Lists workflow invocations from Restate's introspection API with live progress.

use restate_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

use crate::common::auth::restate_auth::require_admin;
use crate::domains::website::models::Website;
use crate::impl_restate_serde;
use crate::kernel::ServerDeps;

// =============================================================================
// Request types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListJobsRequest {
    pub status: Option<String>,
    pub limit: Option<i32>,
}

impl_restate_serde!(ListJobsRequest);

// =============================================================================
// Response types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobResult {
    pub id: String,
    pub workflow_name: String,
    pub workflow_key: String,
    pub status: String,
    pub progress: Option<String>,
    pub created_at: Option<String>,
    pub modified_at: Option<String>,
    pub completed_at: Option<String>,
    pub completion_result: Option<String>,
    pub website_domain: Option<String>,
    pub website_id: Option<String>,
}

impl_restate_serde!(JobResult);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobListResult {
    pub jobs: Vec<JobResult>,
}

impl_restate_serde!(JobListResult);

// =============================================================================
// Restate introspection response types
// =============================================================================

#[derive(Debug, Deserialize)]
struct IntrospectionResponse {
    rows: Vec<IntrospectionRow>,
}

#[derive(Debug, Deserialize)]
struct IntrospectionRow {
    id: Option<String>,
    target_service_name: Option<String>,
    target_service_key: Option<String>,
    status: Option<String>,
    created_at: Option<String>,
    modified_at: Option<String>,
    #[serde(rename = "pinned_deployment_id")]
    _pinned_deployment_id: Option<String>,
    completion_result: Option<String>,
    progress: Option<String>,
}

// =============================================================================
// Service definition
// =============================================================================

#[restate_sdk::service]
#[name = "Jobs"]
pub trait JobsService {
    async fn list(req: ListJobsRequest) -> Result<JobListResult, HandlerError>;
}

pub struct JobsServiceImpl {
    deps: Arc<ServerDeps>,
}

impl JobsServiceImpl {
    pub fn with_deps(deps: Arc<ServerDeps>) -> Self {
        Self { deps }
    }
}

/// Validate status filter against allowed values
fn validate_status(status: &str) -> bool {
    matches!(
        status,
        "running" | "completed" | "backing-off" | "suspended"
    )
}

/// Extract website UUID from a workflow key.
///
/// Workflow keys follow patterns like:
/// - `crawl-{uuid}-{timestamp}`
/// - `regen-{uuid}-{timestamp}`
/// - `dedup-{uuid}-{timestamp}`
/// - `research-{uuid}-{timestamp}`
/// - `extract-{uuid}-{timestamp}`
///
/// We look for a UUID pattern anywhere in the key.
fn extract_website_id(key: &str) -> Option<Uuid> {
    // Try to find a UUID (8-4-4-4-12 hex pattern) in the key
    for part in key.split('-').collect::<Vec<_>>().windows(5) {
        let candidate = format!(
            "{}-{}-{}-{}-{}",
            part[0], part[1], part[2], part[3], part[4]
        );
        if let Ok(uuid) = Uuid::parse_str(&candidate) {
            return Some(uuid);
        }
    }
    // Also try the key itself if it starts with a UUID directly
    if key.len() >= 36 {
        if let Ok(uuid) = Uuid::parse_str(&key[..36]) {
            return Some(uuid);
        }
    }
    None
}

/// Strip surrounding JSON quotes from Restate state values.
/// Restate stores strings as `"\"value\""`, so we need to unescape.
fn clean_progress(raw: &str) -> String {
    let trimmed = raw.trim();
    // Remove outer quotes if present
    let unquoted = if trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2 {
        &trimmed[1..trimmed.len() - 1]
    } else {
        trimmed
    };
    // Unescape inner quotes
    unquoted.replace("\\\"", "\"")
}

/// Map Restate invocation status to a friendly status string
fn map_status(status: &str) -> &str {
    match status {
        "running" => "running",
        "completed" => "completed",
        "backing-off" => "failed",
        "suspended" => "suspended",
        other => other,
    }
}

impl JobsService for JobsServiceImpl {
    async fn list(
        &self,
        ctx: Context<'_>,
        req: ListJobsRequest,
    ) -> Result<JobListResult, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        let restate_admin_url = std::env::var("RESTATE_ADMIN_URL")
            .unwrap_or_else(|_| "http://restate:9070".to_string());

        let limit = req.limit.unwrap_or(50);

        // Build SQL query against Restate's introspection API
        let mut sql = String::from(
            r#"SELECT
                i.id,
                i.target_service_name,
                i.target_service_key,
                i.status,
                i.created_at,
                i.modified_at,
                i.pinned_deployment_id,
                i.completion_result,
                s.value_utf8 as progress
            FROM sys_invocation i
            LEFT JOIN state s
                ON s.service_name = i.target_service_name
                AND s.service_key = i.target_service_key
                AND s.key = 'status'
            WHERE i.target_service_ty = 'workflow'
                AND i.target_handler_name = 'run'"#,
        );

        // Apply optional status filter
        if let Some(ref status) = req.status {
            if !validate_status(status) {
                return Err(
                    TerminalError::new(format!("Invalid status filter: {}", status)).into(),
                );
            }
            sql.push_str(&format!(" AND i.status = '{}'", status));
        }

        sql.push_str(" ORDER BY i.created_at DESC");
        sql.push_str(&format!(" LIMIT {}", limit));

        // Query Restate introspection API
        let client = reqwest::Client::new();
        let response = client
            .post(format!("{}/query", restate_admin_url))
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({ "query": sql }))
            .send()
            .await
            .map_err(|e| TerminalError::new(format!("Failed to query Restate: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "unknown".to_string());
            return Err(
                TerminalError::new(format!("Restate query failed ({}): {}", status, body)).into(),
            );
        }

        let introspection: IntrospectionResponse = response
            .json()
            .await
            .map_err(|e| TerminalError::new(format!("Failed to parse Restate response: {}", e)))?;

        // Extract website UUIDs from workflow keys for batch lookup
        let mut website_ids: Vec<Uuid> = Vec::new();
        let mut key_to_website_id: HashMap<String, Uuid> = HashMap::new();

        for row in &introspection.rows {
            if let Some(ref key) = row.target_service_key {
                if let Some(uuid) = extract_website_id(key) {
                    if !website_ids.contains(&uuid) {
                        website_ids.push(uuid);
                    }
                    key_to_website_id.insert(key.clone(), uuid);
                }
            }
        }

        // Batch-lookup website domains
        let domain_map: HashMap<Uuid, String> = if !website_ids.is_empty() {
            Website::find_domains_by_ids(&website_ids, &self.deps.db_pool)
                .await
                .unwrap_or_default()
                .into_iter()
                .collect()
        } else {
            HashMap::new()
        };

        // Build results
        let jobs: Vec<JobResult> = introspection
            .rows
            .into_iter()
            .map(|row| {
                let key = row.target_service_key.clone().unwrap_or_default();
                let website_uuid = key_to_website_id.get(&key).copied();
                let website_domain = website_uuid.and_then(|id| domain_map.get(&id).cloned());

                // Determine completed_at: use modified_at if status is completed
                let completed_at = if row.status.as_deref() == Some("completed") {
                    row.modified_at.clone()
                } else {
                    None
                };

                let progress = row.progress.as_deref().map(clean_progress);
                let status = row
                    .status
                    .as_deref()
                    .map(map_status)
                    .unwrap_or("unknown")
                    .to_string();

                JobResult {
                    id: row.id.unwrap_or_default(),
                    workflow_name: row.target_service_name.unwrap_or_default(),
                    workflow_key: key,
                    status,
                    progress,
                    created_at: row.created_at,
                    modified_at: row.modified_at,
                    completed_at,
                    completion_result: row.completion_result,
                    website_domain,
                    website_id: website_uuid.map(|id| id.to_string()),
                }
            })
            .collect();

        Ok(JobListResult { jobs })
    }
}
