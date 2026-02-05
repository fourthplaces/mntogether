//! Job information for UI display.

use anyhow::Result;
use uuid::Uuid;

/// Query result for job status
#[derive(Debug, Clone)]
pub struct JobInfo {
    pub job_id: Uuid,
    pub job_type: String,
    pub status: String,
    pub error_message: Option<String>,
}

impl JobInfo {
    /// Find the latest job for a website by job type
    pub async fn find_latest_for_website(
        website_id: Uuid,
        job_type: &str,
        pool: &sqlx::PgPool,
    ) -> Result<Option<Self>> {
        let row = sqlx::query_as::<_, (Uuid, String, String, Option<String>)>(
            r#"
            SELECT id, job_type, status::text, error_message
            FROM jobs
            WHERE reference_id = $1 AND job_type = $2
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .bind(website_id)
        .bind(job_type)
        .fetch_optional(pool)
        .await?;

        Ok(
            row.map(|(job_id, job_type, status, error_message)| JobInfo {
                job_id,
                job_type,
                status,
                error_message,
            }),
        )
    }
}
