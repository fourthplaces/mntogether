//! Job registry for deserializing and executing jobs.
//!
//! The registry maps job type strings (e.g., "crawl_website") to:
//! - Deserializers that reconstruct typed job structs from JSON
//! - Handlers that execute the job logic
//!
//! This allows the JobRunner to claim jobs from the database and dispatch
//! them to the appropriate domain handlers without knowing the concrete types.

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use serde::de::DeserializeOwned;

use super::queue::{ClaimedJob, CommandMeta};
use crate::kernel::ServerDeps;

/// Type alias for the async handler function.
///
/// Handlers take a reference to ServerDeps and return a Result.
/// The job data is captured in the closure when registering.
type BoxedHandler = Box<
    dyn Fn(serde_json::Value, Arc<ServerDeps>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>>
        + Send
        + Sync,
>;

/// Registration entry containing the handler.
struct JobRegistration {
    handler: BoxedHandler,
}

/// Registry that maps job type strings to handlers.
///
/// Each domain registers its job types at startup. When the JobRunner
/// claims a job, it uses this registry to deserialize and execute
/// the job in one step.
///
/// # Example
///
/// ```ignore
/// let mut registry = JobRegistry::new();
///
/// // Register with handler function
/// registry.register::<CrawlWebsiteJob>(
///     CrawlWebsiteJob::JOB_TYPE,
///     |job, deps| async move {
///         crawl_actions::crawl_website(job.website_id, job.use_firecrawl, &deps).await
///     },
/// );
///
/// // Later, in JobRunner
/// registry.execute(&claimed_job, deps.clone()).await?;
/// ```
#[derive(Default)]
pub struct JobRegistry {
    registrations: HashMap<&'static str, JobRegistration>,
}

impl JobRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            registrations: HashMap::new(),
        }
    }

    /// Register a job type with its handler.
    ///
    /// The handler is an async function that receives the deserialized job
    /// and ServerDeps, and returns a Result.
    ///
    /// # Arguments
    ///
    /// * `job_type` - The job type string (e.g., "crawl_website")
    /// * `handler` - Async function to execute the job
    ///
    /// # Example
    ///
    /// ```ignore
    /// registry.register::<CrawlWebsiteJob>(
    ///     CrawlWebsiteJob::JOB_TYPE,
    ///     |job, deps| async move {
    ///         crawl_actions::crawl_website(job.website_id, job.use_firecrawl, &deps).await
    ///     },
    /// );
    /// ```
    pub fn register<J, F, Fut>(&mut self, job_type: &'static str, handler: F)
    where
        J: CommandMeta + DeserializeOwned + Send + Sync + 'static,
        F: Fn(J, Arc<ServerDeps>) -> Fut + Send + Sync + Clone + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let boxed_handler: BoxedHandler = Box::new(move |value, deps| {
            let handler = handler.clone();
            Box::pin(async move {
                let job: J = serde_json::from_value(value)
                    .map_err(|e| anyhow!("Failed to deserialize {}: {}", job_type, e))?;
                handler(job, deps).await
            })
        });

        self.registrations.insert(
            job_type,
            JobRegistration {
                handler: boxed_handler,
            },
        );
    }

    /// Execute a claimed job using its registered handler.
    ///
    /// Returns an error if:
    /// - The job type is not registered
    /// - The JSON payload cannot be deserialized
    /// - The handler returns an error
    pub async fn execute(&self, job: &ClaimedJob, deps: Arc<ServerDeps>) -> Result<()> {
        let job_type = job.command_type();
        let registration = self
            .registrations
            .get(job_type)
            .ok_or_else(|| anyhow!("Unknown job type: {}", job_type))?;

        let args = job
            .job
            .args
            .clone()
            .ok_or_else(|| anyhow!("Job {} has no args", job.id))?;

        (registration.handler)(args, deps).await
    }

    /// Check if a job type is registered.
    pub fn is_registered(&self, job_type: &str) -> bool {
        self.registrations.contains_key(job_type)
    }

    /// Get all registered job types.
    pub fn registered_types(&self) -> Vec<&'static str> {
        self.registrations.keys().copied().collect()
    }
}

/// Thread-safe registry wrapped in Arc.
pub type SharedJobRegistry = Arc<JobRegistry>;

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use uuid::Uuid;

    use crate::kernel::jobs::JobPriority;

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct TestJob {
        pub id: Uuid,
        pub name: String,
    }

    impl CommandMeta for TestJob {
        fn command_type(&self) -> &'static str {
            "test_job"
        }

        fn priority(&self) -> JobPriority {
            JobPriority::Normal
        }
    }

    #[test]
    fn test_register_and_check() {
        let mut registry = JobRegistry::new();
        registry.register::<TestJob, _, _>("test_job", |_job, _deps| async move { Ok(()) });

        assert!(registry.is_registered("test_job"));
        assert!(!registry.is_registered("unknown_job"));
    }

    #[test]
    fn test_registered_types() {
        let mut registry = JobRegistry::new();
        registry.register::<TestJob, _, _>("test_job", |_job, _deps| async move { Ok(()) });

        let types = registry.registered_types();
        assert!(types.contains(&"test_job"));
    }
}
